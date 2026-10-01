use cynpase_bot::voice::VoiceOrchestrator;
use hound::{SampleFormat, WavSpec, WavWriter};
use std::io::Cursor;

fn create_dummy_wav() -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::new(&mut cursor, spec).unwrap();
    for _ in 0..1600 {
        writer.write_sample(0i16).unwrap();
    }
    writer.finalize().unwrap();
    cursor.into_inner()
}

#[tokio::test]
async fn test_voice_loop_with_visual_context_injection() {
    let voice = VoiceOrchestrator::new(
        "http://127.0.0.1:8080/inference",
        "http://127.0.0.1:11434",
        "qwen2.5:3b-instruct",
        "You are StackChan. Be brief.",
        "http://127.0.0.1:8000",
        "alba",
    );

    let dummy_wav = create_dummy_wav();
    let visual_context = Some("1 person detected 1.2m away smiling");

    let (transcript, reply, audio_out) = voice
        .process_turn(&dummy_wav, visual_context)
        .await
        .expect("Voice turn failed");

    assert!(!transcript.is_empty());
    assert!(!reply.is_empty());
    assert!(!audio_out.is_empty());

    // Test autonomous vision narration (rate-limited)
    let narration = voice.trigger_vision_narration("A new friend walked into the room").await;
    assert!(narration.is_some());

    // Immediate second narration should be suppressed by cooldown (FR-V4: 30s rate limit)
    let suppressed = voice.trigger_vision_narration("Another quick event").await;
    assert!(suppressed.is_none(), "Second narration within 30s must be suppressed");

    println!("VOICE_LOOP_PASS");
}
