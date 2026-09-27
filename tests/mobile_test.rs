use axum::{
    body::to_bytes,
    http::{Request, StatusCode},
};
use cynpase_bot::{
    config::HubConfig,
    create_router,
    dashboard::TelemetryEvent,
    pipeline::PipelineEngine,
    server::AppState,
    vision::VisionManager,
};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};
use tower::ServiceExt;

fn create_test_app() -> (axum::Router, broadcast::Receiver<TelemetryEvent>, broadcast::Receiver<cynpase_bot::protocol::ServerMessage>) {
    let config = Arc::new(HubConfig::default());
    let pipeline = PipelineEngine::new(config.clone());
    let (telemetry_tx, telemetry_rx) = broadcast::channel(32);
    let (device_cmd_tx, device_cmd_rx) = broadcast::channel(32);
    let vision = VisionManager::new();

    let state = AppState {
        config: config.clone(),
        pipeline,
        session: Arc::new(Mutex::new(None)),
        telemetry_tx,
        device_cmd_tx,
        vision,
    };

    (create_router(state), telemetry_rx, device_cmd_rx)
}

#[tokio::test]
async fn test_mobile_pwa_endpoints() {
    let (app, _rx, _cmd_rx) = create_test_app();

    // 1. GET /mobile
    let req = Request::builder()
        .uri("/mobile")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("CynapseBot Mobile"));
    assert!(html.contains("avatarCanvas"));
    assert!(html.contains("joyBase"));
    assert!(html.contains("celestialCanvas"));

    // 2. GET /manifest.json
    let req = Request::builder()
        .uri("/manifest.json")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(manifest["name"], "CynapseBot Mobile Companion");
    assert_eq!(manifest["start_url"], "/mobile");

    // 3. GET /service-worker.js
    let req = Request::builder()
        .uri("/service-worker.js")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let sw = String::from_utf8_lossy(&body);
    assert!(sw.contains("cynpase-mobile-v1"));
}

#[tokio::test]
async fn test_mobile_robot_control_and_telemetry() {
    let (app, mut rx, mut cmd_rx) = create_test_app();

    let payload = serde_json::json!({
        "pan": 45,
        "tilt": -10,
        "r": 56,
        "g": 189,
        "b": 248,
        "animation": "dance",
        "expression": "happy"
    });

    let req = Request::builder()
        .method("POST")
        .uri("/api/robot/control")
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(serde_json::to_vec(&payload).unwrap()))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp["status"], "ok");
    assert_eq!(resp["device_connected"], false);
    assert_eq!(resp["actions_dispatched"], 0); // 0 dispatched to socket because no robot is connected

    // Check telemetry broadcast
    let event = rx.recv().await.unwrap();
    assert_eq!(event.event_type, "mobile_robot_control");
    assert_eq!(event.payload["raw"]["animation"], "dance");

    // Check that real ServerMessage::Mcp were emitted into device_cmd_tx
    let mcp1 = cmd_rx.recv().await.unwrap();
    match mcp1 {
        cynpase_bot::protocol::ServerMessage::Mcp { payload } => {
            assert_eq!(payload.jsonrpc, "2.0");
            assert_eq!(payload.method, "tools/call");
            assert_eq!(payload.params.name, "self.robot.set_head_angles");
            assert_eq!(payload.params.arguments["yaw"], 45);
            assert_eq!(payload.params.arguments["pitch"], -10);
        }
        _ => panic!("Expected Mcp message"),
    }
}

#[tokio::test]
async fn test_mobile_control_reaches_connected_robot_socket() {
    use futures_util::StreamExt;
    use tokio::net::TcpListener;
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let config = Arc::new(HubConfig {
        bind_addr: addr,
        public_ws_url: format!("ws://{}/xiaozhi/ws", addr),
        auth_token: "test-secret".to_string(),
        ..Default::default()
    });

    let pipeline = PipelineEngine::new(config.clone());
    let (telemetry_tx, _) = broadcast::channel(32);
    let (device_cmd_tx, _) = broadcast::channel(32);
    let state = AppState {
        config: config.clone(),
        pipeline,
        session: Arc::new(Mutex::new(None)),
        telemetry_tx,
        device_cmd_tx,
        vision: VisionManager::new(),
    };

    let app = create_router(state.clone());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // 1. Connect a simulated StackChan robot over WebSocket
    let ws_url = format!("ws://{}/xiaozhi/ws?token=test-secret", addr);
    let (mut ws_stream, _) = connect_async(&ws_url).await.expect("Robot failed to connect");

    // Wait a brief moment for session registration
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // 2. Dispatch a REST control command from Mobile / GUI
    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://{}/api/robot/control", addr))
        .json(&serde_json::json!({
            "pan": 30,
            "tilt": 15
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 200);
    let json_resp: serde_json::Value = res.json().await.unwrap();
    assert_eq!(json_resp["status"], "ok");
    assert_eq!(json_resp["device_connected"], true);
    assert_eq!(json_resp["actions_dispatched"], 1);

    // 3. FAIL-IF-DEAD-END GATE: The connected robot WebSocket MUST receive the Mcp command frame
    let received_frame = tokio::time::timeout(tokio::time::Duration::from_millis(500), ws_stream.next())
        .await
        .expect("FAIL-IF-DEAD-END: Timed out waiting for Mcp frame on robot socket!")
        .expect("Stream closed unexpectedly")
        .expect("WS frame error");

    if let Message::Text(text) = received_frame {
        let msg: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(msg["type"], "mcp");
        assert_eq!(msg["payload"]["jsonrpc"], "2.0");
        assert_eq!(msg["payload"]["method"], "tools/call");
        assert_eq!(msg["payload"]["params"]["name"], "self.robot.set_head_angles");
        assert_eq!(msg["payload"]["params"]["arguments"]["yaw"], 30);
        assert_eq!(msg["payload"]["params"]["arguments"]["pitch"], 15);
    } else {
        panic!("FAIL-IF-DEAD-END: Expected Text frame containing Mcp JSON command, got {:?}", received_frame);
    }
}

#[tokio::test]
async fn test_mobile_chat_and_status() {
    let (app, _rx, _cmd_rx) = create_test_app();

    // 1. Status API
    let req = Request::builder()
        .uri("/api/status")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let status: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(status["status"], "online");
    assert_eq!(status["device_connected"], false);

    // 2. Chat API
    let req = Request::builder()
        .method("POST")
        .uri("/api/chat/send")
        .header("Content-Type", "application/json")
        .body(axum::body::Body::from(r#"{"message": "Hello CynapseBot"}"#))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let chat_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(chat_resp["status"], "ok");

    // 3. Celestial Memory API
    let req = Request::builder()
        .uri("/api/memory/celestial")
        .body(axum::body::Body::empty())
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let cel_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(cel_resp["status"], "ok");
    assert!(cel_resp["nodes"].as_array().unwrap().len() >= 4);

    // 4. Audio Chat API: Non-RIFF/WAV payload must be rejected with 400
    let bad_req = Request::builder()
        .method("POST")
        .uri("/api/chat/audio")
        .header("Content-Type", "audio/wav")
        .body(axum::body::Body::from(b"not-a-valid-riff-wav-container-bytes".to_vec()))
        .unwrap();

    let res = app.clone().oneshot(bad_req).await.unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST, "Non-RIFF audio payload must be rejected with 400");

    // 5. Audio Chat API (POST /api/chat/audio) with valid WAV: Fail-if-Hello gate
    let pcm_sample: Vec<i16> = (0..1600).map(|i| ((i as f32 * 0.1).sin() * 5000.0) as i16).collect();
    let wav_bytes = cynpase_bot::audio::AudioEngine::pcm_to_wav_bytes(&pcm_sample, 16000);
    let req = Request::builder()
        .method("POST")
        .uri("/api/chat/audio")
        .header("Content-Type", "audio/wav")
        .body(axum::body::Body::from(wav_bytes))
        .unwrap();

    let res = app.clone().oneshot(req).await.unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
    let audio_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    
    // Assert that untranscribed audio never fabricates a fake "Hello" user turn
    if audio_resp["status"] == "untranscribed" {
        assert_eq!(audio_resp["transcript"], "", "Untranscribed audio must NOT fabricate transcript");
        assert_ne!(audio_resp["transcript"], "Hello", "FAIL-IF-HELLO: Must not fall back to fake 'Hello'");
    } else {
        assert_eq!(audio_resp["status"], "ok");
        assert_ne!(audio_resp["transcript"], "Hello");
    }
}
