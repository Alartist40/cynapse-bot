use cynpase_bot::audio::{AudioConfig, AudioEngine};

#[tokio::test]
async fn test_audio_pipeline() {
    let engine = AudioEngine::new(AudioConfig::default());

    // 1. Test Opus framing concatenation
    let frames = vec![
        vec![0x01, 0x02, 0x03],
        vec![0x04, 0x05, 0x06],
    ];
    let concatenated = engine.concatenate_opus_frames(&frames);
    assert_eq!(concatenated, vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);

    // 2. Test Audio transcription (with fallback support)
    let transcript = engine.transcribe(&frames).await.unwrap();
    assert!(!transcript.is_empty());

    // 3. Test Audio TTS synthesis (generates Opus frames)
    let synthesized_frames = engine.synthesize("Hello StackChan").await.unwrap();
    assert!(!synthesized_frames.is_empty());
}
