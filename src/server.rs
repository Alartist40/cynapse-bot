use crate::config::HubConfig;
use crate::dashboard::{handle_dashboard_html, handle_monitor_ws, TelemetryEvent, TelemetrySender};
use crate::mobile::{
    handle_api_celestial_memory, handle_api_chat_audio, handle_api_chat_send, handle_api_robot_control, handle_api_status,
    handle_manifest_json, handle_mobile_app_html, handle_service_worker_js,
};
use crate::ota::handle_ota_discovery;
use crate::pipeline::PipelineEngine;
use crate::protocol::{AudioParams, ClientMessage, ServerMessage};
use crate::session::Session;
use crate::vision::VisionManager;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Router,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<HubConfig>,
    pub pipeline: PipelineEngine,
    pub session: Arc<Mutex<Option<Session>>>,
    pub telemetry_tx: TelemetrySender,
    pub device_cmd_tx: broadcast::Sender<ServerMessage>,
    pub vision: VisionManager,
}

#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: Option<String>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(handle_dashboard_html))
        .route("/dashboard", get(handle_dashboard_html))
        .route("/mobile", get(handle_mobile_app_html))
        .route("/app", get(handle_mobile_app_html))
        .route("/manifest.json", get(handle_manifest_json))
        .route("/service-worker.js", get(handle_service_worker_js))
        .route("/camera/latest.jpg", get(handle_latest_camera_image))
        .route("/ws/monitor", get(handle_monitor_ws))
        .route("/xiaozhi/ota/", get(handle_ota_discovery))
        .route("/xiaozhi/ota", get(handle_ota_discovery))
        .route("/xiaozhi/ws", get(ws_handler))
        .route("/api/status", get(handle_api_status))
        .route("/api/robot/control", post(handle_api_robot_control))
        .route("/api/chat/send", post(handle_api_chat_send))
        .route("/api/chat/audio", post(handle_api_chat_audio))
        .route("/api/memory/celestial", get(handle_api_celestial_memory))
        .route("/health", get(|| async { "OK" }))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(Extension(state.telemetry_tx.clone()))
        .layer(Extension(state.clone()))
        .with_state(state.config.clone())
}

async fn handle_latest_camera_image(
    Extension(state): Extension<AppState>,
) -> Response {
    if let Some(jpeg) = state.vision.get_latest_frame().await {
        ([(axum::http::header::CONTENT_TYPE, "image/jpeg")], jpeg).into_response()
    } else {
        (StatusCode::NO_CONTENT, "No camera frame received yet").into_response()
    }
}

async fn ws_handler(
    Extension(state): Extension<AppState>,
    headers: HeaderMap,
    Query(query): Query<WsAuthQuery>,
    ws: WebSocketUpgrade,
) -> Response {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").trim());

    let token = auth_header.or(query.token.as_deref());

    // Validate token
    if let Some(tok) = token {
        if tok != state.config.auth_token {
            warn!("Rejected unauthorized WS connection attempt with invalid token");
            return (StatusCode::UNAUTHORIZED, "Invalid auth token").into_response();
        }
    } else {
        warn!("Rejected WS connection attempt without auth token");
        return (StatusCode::UNAUTHORIZED, "Missing auth token").into_response();
    }

    let device_id = headers
        .get("device-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("stackchan-device")
        .to_string();

    let client_id = headers
        .get("client-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("stackchan-client")
        .to_string();

    ws.on_upgrade(move |socket| handle_socket(socket, state, device_id, client_id))
}

async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    device_id: String,
    client_id: String,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut cmd_rx = state.device_cmd_tx.subscribe();
    let session = Session::new(device_id.clone(), client_id.clone());
    let session_id = session.id.clone();
    *state.session.lock().await = Some(session);

    let _ = state.telemetry_tx.send(TelemetryEvent {
        event_type: "device_connected".to_string(),
        payload: serde_json::json!({
            "session_id": session_id,
            "device_id": device_id,
            "client_id": client_id
        }),
        timestamp_ms: 0,
    });

    info!(session_id = %session_id, device_id = %device_id, "Device connected via Xiaozhi WS");

    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Ok(server_msg) => {
                        if let Ok(json_text) = serde_json::to_string(&server_msg) {
                            info!(tool = ?server_msg, "Forwarding external command to connected robot WebSocket");
                            if let Err(e) = sender.send(Message::Text(json_text.into())).await {
                                error!("Failed to send command to robot socket: {}", e);
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("Command receiver lagged by {} messages", skipped);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            msg = receiver.next() => {
                let msg = match msg {
                    Some(Ok(m)) => m,
                    Some(Err(e)) => {
                        warn!("WS receive error: {}", e);
                        break;
                    }
                    None => {
                        break;
                    }
                };

        match msg {
            Message::Text(text) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Hello { version, .. }) => {
                        info!(version, "Received client hello");
                        let server_hello = ServerMessage::Hello {
                            transport: "websocket".to_string(),
                            session_id: session_id.clone(),
                            audio_params: AudioParams {
                                format: "opus".to_string(),
                                sample_rate: 24000,
                                channels: 1,
                                frame_duration: 60,
                            },
                        };
                        let resp = serde_json::to_string(&server_hello).unwrap();
                        if let Err(e) = sender.send(Message::Text(resp.into())).await {
                            error!("Failed to send server hello: {}", e);
                            break;
                        }
                    }
                    Ok(ClientMessage::Listen { state: listen_state, .. }) => {
                        info!(listen_state = %listen_state, "Received listen control event");
                        if listen_state == "start" {
                            if let Some(ref mut sess) = *state.session.lock().await {
                                sess.start_listening();
                            }
                        } else if listen_state == "stop" {
                            let frames = {
                                if let Some(ref mut sess) = *state.session.lock().await {
                                    sess.stop_listening()
                                } else {
                                    Vec::new()
                                }
                            };

                            // Process audio frames through pipeline
                            match state.pipeline.process_audio_frames(frames).await {
                                Ok(turn) => {
                                    for msg in &turn.messages {
                                        let resp = serde_json::to_string(msg).unwrap();
                                        let _ = state.telemetry_tx.send(TelemetryEvent {
                                            event_type: "turn_message".to_string(),
                                            payload: serde_json::to_value(msg).unwrap_or_default(),
                                            timestamp_ms: 0,
                                        });
                                        if let Err(e) = sender.send(Message::Text(resp.into())).await {
                                            error!("Failed to send pipeline reply: {}", e);
                                            break;
                                        }

                                        // Stream binary Opus audio frames between tts start and stop
                                        if let ServerMessage::Tts { state: tts_state, .. } = msg {
                                            if tts_state == "start" {
                                                info!(
                                                    frames_count = turn.audio_frames.len(),
                                                    "Streaming 24kHz binary Opus audio frames to device"
                                                );
                                                for frame in &turn.audio_frames {
                                                    if let Err(e) = sender.send(Message::Binary(frame.clone().into())).await {
                                                        error!("Failed to send binary Opus audio frame: {}", e);
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("Pipeline turn failed: {}", e);
                                }
                            }
                        }
                    }
                    Ok(ClientMessage::Abort { reason }) => {
                        info!(reason = ?reason, "Received client abort");
                        if let Some(ref mut sess) = *state.session.lock().await {
                            sess.audio_buffer.clear();
                            sess.is_listening = false;
                        }
                    }
                    Ok(ClientMessage::Iot { .. }) | Ok(ClientMessage::Mcp { .. }) => {
                        info!("Received IoT/MCP client payload");
                    }
                    Ok(ClientMessage::Unknown) => {
                        warn!("Received unknown text message: {}", text);
                    }
                    Err(e) => {
                        warn!("JSON parse error on message: {} ({})", text, e);
                    }
                }
            }
            Message::Binary(bin) => {
                // Check if binary payload is a JPEG camera frame or audio Opus frame
                if VisionManager::is_jpeg_payload(&bin) {
                    state.vision.ingest_frame(bin.to_vec()).await;
                    let _ = state.telemetry_tx.send(TelemetryEvent {
                        event_type: "camera_frame".to_string(),
                        payload: serde_json::json!({ "bytes": bin.len() }),
                        timestamp_ms: 0,
                    });
                } else if let Some(ref mut sess) = *state.session.lock().await {
                    sess.push_audio_frame(bin.to_vec());
                }
            }
            Message::Ping(p) => {
                if let Err(e) = sender.send(Message::Pong(p)).await {
                    error!("Failed to send Pong: {}", e);
                    break;
                }
            }
            Message::Close(_) => {
                info!("Device closed WS connection");
                break;
            }
            _ => {}
        }
            }
        }
    }

    *state.session.lock().await = None;
    info!(session_id = %session_id, "Session closed");
}
