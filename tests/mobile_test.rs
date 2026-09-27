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

fn create_test_app() -> (axum::Router, broadcast::Receiver<TelemetryEvent>) {
    let config = Arc::new(HubConfig::default());
    let pipeline = PipelineEngine::new(config.clone());
    let (telemetry_tx, telemetry_rx) = broadcast::channel(32);
    let vision = VisionManager::new();

    let state = AppState {
        config: config.clone(),
        pipeline,
        session: Arc::new(Mutex::new(None)),
        telemetry_tx,
        vision,
    };

    (create_router(state), telemetry_rx)
}

#[tokio::test]
async fn test_mobile_pwa_endpoints() {
    let (app, _rx) = create_test_app();

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
    let (app, mut rx) = create_test_app();

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
    assert_eq!(resp["actions_dispatched"], 4); // servos, led, animation, expression

    // Check telemetry broadcast
    let event = rx.recv().await.unwrap();
    assert_eq!(event.event_type, "mobile_robot_control");
    assert_eq!(event.payload["raw"]["animation"], "dance");
}

#[tokio::test]
async fn test_mobile_chat_and_status() {
    let (app, _rx) = create_test_app();

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
}
