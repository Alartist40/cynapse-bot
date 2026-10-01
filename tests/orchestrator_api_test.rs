use axum::body::Body;
use axum::http::{Request, StatusCode};
use cynpase_bot::api::{create_api_router, AppState};
use cynpase_bot::vision::{VisionDetector, VisionTracker};
use cynpase_bot::voice::VoiceOrchestrator;
use http_body_util::BodyExt;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tower::ServiceExt;

#[tokio::test]
async fn test_orchestrator_api_endpoints() {
    let detector = VisionDetector::new("yolo11s", 0.5);
    let tracker = Mutex::new(VisionTracker::new());
    let voice = VoiceOrchestrator::new(
        "http://127.0.0.1:8080/inference",
        "http://127.0.0.1:11434",
        "qwen2.5:3b-instruct",
        "You are StackChan.",
        "http://127.0.0.1:8000",
        "alba",
    );

    let state = Arc::new(AppState {
        start_time: Instant::now(),
        detector,
        tracker,
        voice,
        bus: None,
        frame_counter: AtomicU64::new(0),
    });

    let app = create_api_router(state);

    // 1. GET /health
    let req = Request::builder().uri("/health").body(Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let health: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(health["status"], "ok");

    // 2. GET /stats
    let req = Request::builder().uri("/stats").body(Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 3. POST /say
    let say_payload = serde_json::json!({
        "text": "Hello world from StackChan!",
        "expression": "happy"
    });
    let req = Request::builder()
        .method("POST")
        .uri("/say")
        .header("content-type", "application/json")
        .body(Body::from(say_payload.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 4. POST /api/vision/frame
    let dummy_frame = vec![0u8; 100];
    let req = Request::builder()
        .method("POST")
        .uri("/api/vision/frame")
        .header("content-type", "application/octet-stream")
        .body(Body::from(dummy_frame))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let frame_resp: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(frame_resp["detections"].is_array());

    println!("ORCHESTRATOR_API_PASS");
}
