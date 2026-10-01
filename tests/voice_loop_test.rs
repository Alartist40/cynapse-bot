use cynapse_bot::voice::VoiceOrchestrator;
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
async fn test_voice_loop_honest_error_when_services_offline() {
    // When dev_fallbacks is false (default production mode), offline services return Err
    let voice = VoiceOrchestrator::new(
        "http://127.0.0.1:8080/inference",
        "http://127.0.0.1:11434",
        "qwen2.5:3b-instruct",
        "You are StackChan.",
        0.7,
        100,
        "http://127.0.0.1:8000",
        "alba",
        false, // dev_fallbacks = false
    );

    let dummy_wav = create_dummy_wav();
    let result = voice.process_turn(&dummy_wav, None).await;

    // Must fail honestly with Err when services are offline
    assert!(result.is_err(), "Voice loop must fail with Err when STT/LLM/TTS are offline");
}

#[tokio::test]
async fn test_voice_loop_dev_fallbacks_and_cooldown() {
    // When dev_fallbacks is explicitly enabled for offline unit testing:
    let voice = VoiceOrchestrator::new(
        "http://127.0.0.1:8080/inference",
        "http://127.0.0.1:11434",
        "qwen2.5:3b-instruct",
        "You are StackChan.",
        0.7,
        100,
        "http://127.0.0.1:8000",
        "alba",
        true, // dev_fallbacks = true
    );

    let dummy_wav = create_dummy_wav();
    let (transcript, reply, audio) = voice.process_turn(&dummy_wav, Some("1 person detected")).await.unwrap();
    assert!(!transcript.is_empty());
    assert!(!reply.is_empty());
    assert!(!audio.is_empty());

    // Test rate-limited narration cooldown
    let narration1 = voice.trigger_vision_narration("person entered room").await;
    assert!(narration1.is_some());

    let narration2 = voice.trigger_vision_narration("person moved").await;
    assert!(narration2.is_none(), "Narration within 30s must be suppressed by cooldown");

    println!("VOICE_LOOP_PASS");
}
