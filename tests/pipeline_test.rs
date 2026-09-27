use cynpase_bot::{HubConfig, PipelineEngine};
use cynpase_bot::protocol::ServerMessage;
use std::sync::Arc;

#[tokio::test]
async fn test_pipeline_turn() {
    let config = Arc::new(HubConfig::default());
    let pipeline = PipelineEngine::new(config);

    // Test Tier 1 Fast Action: "stop"
    let turn = pipeline.process_text_turn("stop").await.unwrap();
    let has_mcp = turn.messages.iter().any(|m| matches!(m, ServerMessage::Mcp { tool, .. } if tool == "self.robot.set_head_angles"));
    assert!(has_mcp, "Stop command must return Mcp tool message immediately for hal_mcp execution");
    assert!(!turn.audio_frames.is_empty(), "Fast action turn must generate binary Opus audio frames");

    // Test Tier 1 Fast Action: "wave"
    let turn = pipeline.process_text_turn("please wave to me").await.unwrap();
    let has_wave = turn.messages.iter().any(|m| matches!(m, ServerMessage::Mcp { tool, .. } if tool == "self.robot.set_head_angles"));
    assert!(has_wave, "Wave command must return native hal_mcp head angle message immediately");
    assert!(!turn.audio_frames.is_empty(), "Wave turn must generate binary Opus audio frames");

    // Test Tier 3 General Turn (Offline fallback handling)
    let turn = pipeline.process_text_turn("What is the capital of France?").await.unwrap();
    let has_stt = turn.messages.iter().any(|m| matches!(m, ServerMessage::Stt { .. }));
    let has_tts = turn.messages.iter().any(|m| matches!(m, ServerMessage::Tts { .. }));
    assert!(has_stt, "Turn must acknowledge STT");
    assert!(has_tts, "Turn must produce TTS response");
    assert!(!turn.audio_frames.is_empty(), "LLM turn must generate binary Opus audio frames");
}
