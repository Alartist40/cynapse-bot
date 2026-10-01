use axum::body::Body;
use axum::http::{Request, StatusCode};
use cynapse_bot::api::{create_api_router, AppState};
use cynapse_bot::vision::{Detection, VisionDetector, VisionTracker};
use cynapse_bot::voice::VoiceOrchestrator;
use http_body_util::BodyExt;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tower::ServiceExt;

#[tokio::test]
async fn test_orchestrator_api_endpoints_and_readonly_stats() {
    let detector = VisionDetector::new("yolo11s", 0.5, true);
    let tracker = Mutex::new(VisionTracker::new(true));
    let voice = VoiceOrchestrator::new(
        "http://127.0.0.1:8080/inference",
        "http://127.0.0.1:11434",
        "qwen2.5:3b-instruct",
        "You are StackChan.",
        0.7,
        100,
        "http://127.0.0.1:8000",
        "alba",
        true,
    );

    // Pre-seed tracker with 5 detections so a track is in Stable state
    {
        let mut t = tracker.lock().await;
        let det = vec![Detection {
            class: "person".to_string(),
            conf: 0.90,
            cx: 0.5,
            cy: 0.5,
            w: 0.3,
            h: 0.6,
        }];
        for _ in 0..6 {
            t.update(&det);
        }
        assert_eq!(t.peek_tracks().len(), 1);
    }

    let state = Arc::new(AppState {
        start_time: Instant::now(),
        detector,
        tracker,
        voice,
        bus: None,
        frame_counter: AtomicU64::new(0),
    });

    let app = create_api_router(state.clone());

    // 1. GET /health
    let req = Request::builder().uri("/health").body(Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // 2. GET /stats called 20 times in a row — must NOT destroy or modify tracks!
    for _ in 0..20 {
        let req = Request::builder().uri("/stats").body(Body::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = resp.into_body().collect().await.unwrap().to_bytes();
        let stats: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(stats["active_tracks"], 1, "Stats poll must not mutate or drop tracks");
    }

    // Verify track is still present and intact in state
    assert_eq!(state.tracker.lock().await.peek_tracks().len(), 1);

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

    println!("ORCHESTRATOR_API_PASS");
}
