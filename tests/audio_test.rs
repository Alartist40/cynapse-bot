use cynpase_bot::audio::{AudioConfig, AudioEngine};
use opus::{Channels, Decoder};

#[tokio::test]
async fn test_audio_pipeline() {
    let engine = AudioEngine::new(AudioConfig::default());

    // 1. Test genuine 24 kHz PCM -> 60 ms Opus encoding
    let _sample_rate = 24000;
    let num_samples = 1440 * 2; // 2 frames of 60ms
    let pcm: Vec<i16> = (0..num_samples)
        .map(|i| ((i as f32 * 0.1).sin() * 10000.0) as i16)
        .collect();

    let opus_frames = AudioEngine::encode_24k_pcm_to_opus(&pcm).unwrap();
    assert_eq!(opus_frames.len(), 2, "Must produce exactly 2 Opus frames for 120ms");
    assert!(opus_frames.iter().all(|f| !f.is_empty()), "All Opus frames must contain encoded bytes");

    // 2. Decode the 24 kHz Opus frames to verify genuine Opus bitstream
    let mut decoder_24k = Decoder::new(24000, Channels::Mono).expect("Failed to create 24k Opus decoder");
    let mut decoded_buf = vec![0i16; 1440];
    for frame in &opus_frames {
        let samples = decoder_24k
            .decode(frame, &mut decoded_buf, false)
            .expect("Opus frame must decode successfully without error");
        assert_eq!(samples, 1440, "Each 60ms frame at 24kHz must decode to 1440 samples");
    }

    // 3. Test Audio TTS synthesis (generates valid, decodable Opus frames)
    let synthesized_frames = engine.synthesize("Hello StackChan").await.unwrap();
    assert!(!synthesized_frames.is_empty(), "Synthesize must return non-empty Opus frames");

    let mut decoder_synth = Decoder::new(24000, Channels::Mono).expect("Decoder init failed");
    for frame in &synthesized_frames {
        let samples = decoder_synth
            .decode(frame, &mut decoded_buf, false)
            .expect("Synthesized frame must be valid Opus");
        assert_eq!(samples, 1440);
    }
}
