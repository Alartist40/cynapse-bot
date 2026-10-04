use crate::bus::MqttBus;
use crate::vision::{Detection, GazeCommand, VisionDetector, VisionTracker};
use crate::voice::VoiceOrchestrator;

use axum::extract::{DefaultBodyLimit, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;

pub struct AppState {
    pub start_time: Instant,
    pub detector: VisionDetector,
    pub tracker: Mutex<VisionTracker>,
    pub voice: VoiceOrchestrator,
    pub bus: Option<MqttBus>,
    pub frame_counter: AtomicU64,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_secs: u64,
    pub services: serde_json::Value,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub uptime_secs: u64,
    pub processed_frames: u64,
    pub total_conversations: u64,
    pub active_tracks: usize,
}

#[derive(Deserialize)]
pub struct SayRequest {
    pub text: String,
    pub expression: Option<String>,
}

#[derive(Serialize)]
pub struct SayResponse {
    pub status: String,
    pub text: String,
    pub expression: String,
}

#[derive(Serialize)]
pub struct VisionFrameResponse {
    pub detections: Vec<Detection>,
    pub gaze: Option<GazeCommand>,
    pub tracks_count: usize,
}

pub fn create_api_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/stats", get(stats_handler))
        .route("/say", post(say_handler))
        .route(
            "/api/vision/frame",
            post(vision_frame_handler).layer(DefaultBodyLimit::max(10 * 1024 * 1024)), // 10MB limit
        )
        .route("/xiaozhi/ota", get(ota_handler).post(ota_handler))
        .route("/xiaozhi/ota/", get(ota_handler).post(ota_handler))
        .route("/xiaozhi/ota/activate", get(ota_handler).post(ota_handler))
        .route("/ota", get(ota_handler).post(ota_handler))
        .route("/ota/", get(ota_handler).post(ota_handler))
        .route("/ota/activate", get(ota_handler).post(ota_handler))
        .with_state(state)
}

async fn ota_handler() -> Json<serde_json::Value> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Json(serde_json::json!({
        "websocket": {
            "url": "ws://192.168.10.129:8100/xiaozhi/v1/",
            "token": "local-token"
        },
        "server_time": {
            "timestamp": now,
            "timezone_offset": 0
        }
    }))
}

async fn health_handler(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_secs: uptime,
        services: serde_json::json!({
            "vision": state.detector.status_str(),

            "whisper": state.voice.whisper.endpoint_url,
            "ollama": state.voice.llm.host,
            "tts": state.voice.tts.server_url,
            "mqtt": state.bus.as_ref().map(|b| format!("{}:{}", b.host, b.port)).unwrap_or_else(|| "offline".to_string())
        }),
    })
}

async fn stats_handler(State(state): State<Arc<AppState>>) -> Json<StatsResponse> {
    let uptime = state.start_time.elapsed().as_secs();
    let frames = state.frame_counter.load(Ordering::Relaxed);
    let conversations = state.voice.total_conversations.load(Ordering::Relaxed);
    // Non-mutating read-only peek (does NOT destroy tracks)
    let tracks = state.tracker.lock().await.peek_tracks();

    Json(StatsResponse {
        uptime_secs: uptime,
        processed_frames: frames,
        total_conversations: conversations,
        active_tracks: tracks.len(),
    })
}

async fn say_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SayRequest>,
) -> Json<SayResponse> {
    let expression = payload.expression.unwrap_or_else(|| "happy".to_string());

    // Publish face and say notification commands over MQTT
    if let Some(bus) = &state.bus {
        if let Err(e) = bus.publish_face(&expression).await {
            tracing::warn!("Failed to publish face command for /say: {}", e);
        }
        if let Err(e) = bus.publish_say(&payload.text, &expression).await {
            tracing::warn!("Failed to publish say command for /say: {}", e);
        }
    }

    Json(SayResponse {
        status: "ok".to_string(),
        text: payload.text,
        expression,
    })
}

async fn vision_frame_handler(
    State(state): State<Arc<AppState>>,
    body: axum::body::Bytes,
) -> Json<VisionFrameResponse> {
    state.frame_counter.fetch_add(1, Ordering::Relaxed);
    let detections = state.detector.detect_objects(&body);

    let (tracks, gaze) = state.tracker.lock().await.update(&detections);

    if let Some(bus) = &state.bus {
        if let Err(e) = bus.publish_detections("stackchan", &detections).await {
            tracing::debug!("Failed to publish detections: {}", e);
        }
        if let Some(g) = &gaze {
            if let Err(e) = bus.publish_gaze(g).await {
                tracing::debug!("Failed to publish gaze command: {}", e);
            }
        }
    }

    Json(VisionFrameResponse {
        detections,
        gaze,
        tracks_count: tracks.len(),
    })
}

