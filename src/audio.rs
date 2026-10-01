use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use opus::{Application, Channels, Decoder, Encoder};
use std::io::Cursor;

pub const SAMPLE_RATE: u32 = 16000;
pub const CHANNELS: Channels = Channels::Mono;
pub const FRAME_DURATION_MS: u32 = 60;
pub const SAMPLES_PER_FRAME: usize = (SAMPLE_RATE as usize * FRAME_DURATION_MS as usize) / 1000; // 960
pub const BYTES_PER_FRAME: usize = SAMPLES_PER_FRAME * 2; // 1920

pub struct OpusCodec {
    encoder: Encoder,
    decoder: Decoder,
}

impl OpusCodec {
    pub fn new() -> Result<Self, opus::Error> {
        let encoder = Encoder::new(SAMPLE_RATE, CHANNELS, Application::Voip)?;
        let decoder = Decoder::new(SAMPLE_RATE, CHANNELS)?;
        Ok(Self { encoder, decoder })
    }

    pub fn decode_frame(&mut self, opus_data: &[u8]) -> Result<Vec<i16>, opus::Error> {
        let mut out = vec![0i16; SAMPLES_PER_FRAME];
        let len = self.decoder.decode(opus_data, &mut out, false)?;
        out.truncate(len);
        Ok(out)
    }

    pub fn encode_frame(&mut self, pcm_samples: &[i16]) -> Result<Vec<u8>, opus::Error> {
        let mut out = vec![0u8; 1024];
        let len = self.encoder.encode(pcm_samples, &mut out)?;
        out.truncate(len);
        Ok(out)
    }

    pub fn pcm_bytes_to_opus_frames(&mut self, pcm_bytes: &[u8]) -> Result<Vec<Vec<u8>>, opus::Error> {
        let mut samples = Vec::with_capacity(pcm_bytes.len() / 2);
        for chunk in pcm_bytes.chunks_exact(2) {
            samples.push(i16::from_le_bytes([chunk[0], chunk[1]]));
        }

        let mut frames = Vec::new();
        let mut offset = 0;
        while offset + SAMPLES_PER_FRAME <= samples.len() {
            let chunk = &samples[offset..offset + SAMPLES_PER_FRAME];
            frames.push(self.encode_frame(chunk)?);
            offset += SAMPLES_PER_FRAME;
        }

        let remaining = samples.len() - offset;
        if remaining > 0 {
            let mut padded = samples[offset..].to_vec();
            padded.resize(SAMPLES_PER_FRAME, 0);
            frames.push(self.encode_frame(&padded)?);
        }

        Ok(frames)
    }
}

pub fn resample_24k_to_16k(input: &[i16]) -> Vec<i16> {
    if input.is_empty() {
        return Vec::new();
    }
    // 24000 -> 16000 is 3 -> 2 ratio
    let out_len = (input.len() * 2) / 3;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_idx = (i as f64) * 1.5;
        let idx0 = src_idx.floor() as usize;
        let idx1 = (idx0 + 1).min(input.len() - 1);
        let frac = (src_idx - idx0 as f64) as f32;
        let s0 = input[idx0] as f32;
        let s1 = input[idx1] as f32;
        let val = s0 + frac * (s1 - s0);
        out.push(val.clamp(-32768.0, 32767.0) as i16);
    }
    out
}

pub fn wav_bytes_to_pcm16(wav_data: &[u8]) -> Result<Vec<i16>, String> {
    let cursor = Cursor::new(wav_data);
    let mut reader = WavReader::new(cursor).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let samples: Result<Vec<i16>, _> = reader.samples::<i16>().collect();
    let pcm = samples.map_err(|e| e.to_string())?;

    if spec.sample_rate == 24000 {
        Ok(resample_24k_to_16k(&pcm))
    } else {
        Ok(pcm)
    }
}

pub fn pcm16_to_wav_bytes(pcm: &[i16], sample_rate: u32) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    {
        let mut writer = WavWriter::new(&mut cursor, spec).map_err(|e| e.to_string())?;
        for &s in pcm {
            writer.write_sample(s).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;
    }
    Ok(cursor.into_inner())
}
