use axum::body::Body;
use axum::http::{Request, StatusCode};
use cynpase_bot::{create_router, AppState, HubConfig, PipelineEngine};
use futures_util::StreamExt;
use http_body_util::BodyExt;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, Mutex};
use tokio_tungstenite::connect_async;
use tower::ServiceExt;

#[tokio::test]
async fn test_web_dashboard_and_monitor() {
    let config = Arc::new(HubConfig::default());
    let pipeline = PipelineEngine::new(config.clone());
    let (telemetry_tx, _) = broadcast::channel(50);
    let (device_cmd_tx, _) = broadcast::channel(50);

    let state = AppState {
        config: config.clone(),
        pipeline,
        session: Arc::new(Mutex::new(None)),
        telemetry_tx: telemetry_tx.clone(),
        device_cmd_tx,
        vision: cynpase_bot::VisionManager::new(),
    };

    let router = create_router(state.clone());

    // 1. Test GET /dashboard HTML page
    let req = Request::builder()
        .uri("/dashboard")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);
    assert!(body_str.contains("CynapseBot Dashboard"));

    // 2. Test /ws/monitor WebSocket telemetry connection
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let app = create_router(state);
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let ws_url = format!("ws://{}/ws/monitor", addr);
    let (mut ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect to /ws/monitor");

    // Broadcast test event
    telemetry_tx
        .send(cynpase_bot::dashboard::TelemetryEvent {
            event_type: "ping".to_string(),
            payload: serde_json::json!({"status": "ok"}),
            timestamp_ms: 1234,
        })
        .unwrap();

    if let Some(Ok(tokio_tungstenite::tungstenite::Message::Text(msg))) = ws_stream.next().await {
        let val: serde_json::Value = serde_json::from_str(&msg).unwrap();
        assert_eq!(val["event_type"], "ping");
        assert_eq!(val["payload"]["status"], "ok");
    } else {
        panic!("Failed to receive broadcast telemetry event");
    }
}
