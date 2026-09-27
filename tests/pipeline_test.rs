use cynpase_bot::{HubConfig, PipelineEngine};
use cynpase_bot::protocol::ServerMessage;
use std::sync::Arc;

#[tokio::test]
async fn test_pipeline_turn() {
    let config = Arc::new(HubConfig::default());
    let pipeline = PipelineEngine::new(config);

    // Test Tier 1 Fast Action: "stop"
    let replies = pipeline.process_text_turn("stop").await.unwrap();
    let has_action = replies.iter().any(|m| matches!(m, ServerMessage::Action { action, .. } if action == "stop"));
    assert!(has_action, "Stop command must return Action message immediately");

    // Test Tier 1 Fast Action: "wave"
    let replies = pipeline.process_text_turn("please wave to me").await.unwrap();
    let has_wave = replies.iter().any(|m| matches!(m, ServerMessage::Action { action, .. } if action.contains("wave")));
    assert!(has_wave, "Wave command must return Action message immediately");

    // Test Tier 3 General Turn (Offline fallback handling)
    let replies = pipeline.process_text_turn("What is the capital of France?").await.unwrap();
    let has_stt = replies.iter().any(|m| matches!(m, ServerMessage::Stt { .. }));
    let has_tts = replies.iter().any(|m| matches!(m, ServerMessage::Tts { .. }));
    assert!(has_stt, "Turn must acknowledge STT");
    assert!(has_tts, "Turn must produce TTS response");
}
