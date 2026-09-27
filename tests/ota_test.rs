use axum::body::Body;
use axum::http::{Request, StatusCode};
use cynpase_bot::{create_router, AppState, HubConfig, PipelineEngine};
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

#[tokio::test]
async fn test_ota_endpoint() {
    let config = Arc::new(HubConfig {
        public_ws_url: "ws://192.168.1.50:8000/xiaozhi/ws".to_string(),
        auth_token: "test-token-123".to_string(),
        ..Default::default()
    });

    let pipeline = PipelineEngine::new(config.clone());
    let (telemetry_tx, _) = tokio::sync::broadcast::channel(10);
    let state = AppState {
        config: config.clone(),
        pipeline,
        session: Arc::new(Mutex::new(None)),
        telemetry_tx,
        vision: cynpase_bot::VisionManager::new(),
    };

    let router = create_router(state);

    let req = Request::builder()
        .uri("/xiaozhi/ota/")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["websocket"]["url"], "ws://192.168.1.50:8000/xiaozhi/ws");
    assert_eq!(json["websocket"]["token"], "test-token-123");
    assert_eq!(json["websocket"]["version"], 1);
}
