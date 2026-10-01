use cynapse_bot::audio::{
    pcm16_to_wav_bytes, resample_24k_to_16k, wav_bytes_to_pcm16, OpusCodec, SAMPLES_PER_FRAME,
};

#[test]
fn test_opus_codec_roundtrip() {
    let mut codec = OpusCodec::new().expect("Failed to initialize OpusCodec");
    let mut original_pcm = Vec::with_capacity(SAMPLES_PER_FRAME);
    for i in 0..SAMPLES_PER_FRAME {
        let t = i as f32 / 16000.0;
        let s = ((2.0 * std::f32::consts::PI * 440.0 * t).sin() * 16384.0) as i16;
        original_pcm.push(s);
    }

    let encoded = codec.encode_frame(&original_pcm).expect("Encode failed");
    assert!(!encoded.is_empty());

    let decoded = codec.decode_frame(&encoded).expect("Decode failed");
    assert_eq!(decoded.len(), SAMPLES_PER_FRAME);
}

#[test]
fn test_audio_resampling_and_wav() {
    // 24kHz sine wave (2400 samples = 100ms)
    let mut pcm24 = Vec::with_capacity(2400);
    for i in 0..2400 {
        let t = i as f32 / 24000.0;
        let s = ((2.0 * std::f32::consts::PI * 440.0 * t).sin() * 16384.0) as i16;
        pcm24.push(s);
    }

    let pcm16 = resample_24k_to_16k(&pcm24);
    assert_eq!(pcm16.len(), 1600); // 2400 * 2 / 3 = 1600

    let wav_bytes = pcm16_to_wav_bytes(&pcm16, 16000).expect("WAV write failed");
    assert!(wav_bytes.len() > pcm16.len() * 2);

    let recovered = wav_bytes_to_pcm16(&wav_bytes).expect("WAV parse failed");
    assert_eq!(recovered.len(), pcm16.len());

    println!("AUDIO_TEST_PASS");
}
