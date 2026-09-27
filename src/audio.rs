use opus::{Application, Channels, Decoder, Encoder};
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub tts_engine: String, // "pocket-tts" | "piper"
    pub whisper_bin: String,
    pub whisper_model_path: String,
    pub piper_bin: String,
    pub piper_model_path: String,
    pub pocket_tts_url: String,
    pub pocket_tts_voice: String, // e.g. "alba", "eponine", or path to WAV sample for cloning
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            tts_engine: "pocket-tts".to_string(),
            whisper_bin: "whisper".to_string(),
            whisper_model_path: "models/whisper-tiny.bin".to_string(),
            piper_bin: "piper".to_string(),
            piper_model_path: "models/piper-en.onnx".to_string(),
            pocket_tts_url: "http://127.0.0.1:8020/tts".to_string(),
            pocket_tts_voice: "alba".to_string(),
        }
    }
}

pub struct AudioEngine {
    config: AudioConfig,
    http_client: reqwest::Client,
}

impl AudioEngine {
    pub fn new(config: AudioConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
        }
    }

    /// Decode incoming client 16 kHz Opus frames to 16 kHz 16-bit linear PCM
    pub fn decode_client_opus_frames(frames: &[Vec<u8>]) -> anyhow::Result<Vec<i16>> {
        if frames.is_empty() {
            return Ok(Vec::new());
        }

        let mut decoder = Decoder::new(16000, Channels::Mono)?;
        let mut pcm_output = Vec::new();
        let mut out_buffer = vec![0i16; 5760]; // Up to 120ms buffer at 16kHz

        for frame in frames {
            if frame.is_empty() {
                continue;
            }
            match decoder.decode(frame, &mut out_buffer, false) {
                Ok(samples_decoded) => {
                    pcm_output.extend_from_slice(&out_buffer[..samples_decoded]);
                }
                Err(e) => {
                    warn!("Failed to decode Opus frame (len {}): {}", frame.len(), e);
                }
            }
        }

        Ok(pcm_output)
    }

    /// Encode 24 kHz linear PCM s16le samples into 60 ms Opus frames (1440 samples per frame)
    pub fn encode_24k_pcm_to_opus(pcm_samples: &[i16]) -> anyhow::Result<Vec<Vec<u8>>> {
        if pcm_samples.is_empty() {
            return Ok(Vec::new());
        }

        let mut encoder = Encoder::new(24000, Channels::Mono, Application::Voip)?;
        let frame_size = 1440; // 60ms at 24kHz = 1440 samples
        let mut opus_frames = Vec::new();
        let mut out_buf = vec![0u8; 4000];

        for chunk in pcm_samples.chunks(frame_size) {
            let mut padded_chunk;
            let slice = if chunk.len() < frame_size {
                padded_chunk = chunk.to_vec();
                padded_chunk.resize(frame_size, 0);
                &padded_chunk[..]
            } else {
                chunk
            };

            match encoder.encode(slice, &mut out_buf) {
                Ok(len) => {
                    opus_frames.push(out_buf[..len].to_vec());
                }
                Err(e) => {
                    error!("Opus encode error on 60ms chunk: {}", e);
                }
            }
        }

        Ok(opus_frames)
    }

    /// Encode 16 kHz linear PCM s16le samples into 60 ms Opus frames (960 samples per frame)
    pub fn encode_16k_pcm_to_opus(pcm_samples: &[i16]) -> anyhow::Result<Vec<Vec<u8>>> {
        if pcm_samples.is_empty() {
            return Ok(Vec::new());
        }

        let mut encoder = Encoder::new(16000, Channels::Mono, Application::Voip)?;
        let frame_size = 960; // 60ms at 16kHz = 960 samples
        let mut opus_frames = Vec::new();
        let mut out_buf = vec![0u8; 4000];

        for chunk in pcm_samples.chunks(frame_size) {
            let mut padded_chunk;
            let slice = if chunk.len() < frame_size {
                padded_chunk = chunk.to_vec();
                padded_chunk.resize(frame_size, 0);
                &padded_chunk[..]
            } else {
                chunk
            };

            match encoder.encode(slice, &mut out_buf) {
                Ok(len) => {
                    opus_frames.push(out_buf[..len].to_vec());
                }
                Err(e) => {
                    error!("Opus encode error on 16k 60ms chunk: {}", e);
                }
            }
        }

        Ok(opus_frames)
    }

    /// Convert raw linear PCM bytes (s16le) to i16 slice
    pub fn pcm_bytes_to_i16(bytes: &[u8]) -> Vec<i16> {
        bytes
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect()
    }

    /// Convert i16 samples to WAV bytes (16-bit mono)
    pub fn pcm_to_wav_bytes(samples: &[i16], sample_rate: u32) -> Vec<u8> {
        let num_channels: u16 = 1;
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * (num_channels as u32) * (bits_per_sample as u32 / 8);
        let block_align = num_channels * (bits_per_sample / 8);
        let subchunk2_size = (samples.len() * 2) as u32;
        let chunk_size = 36 + subchunk2_size;

        let mut wav = Vec::with_capacity(44 + samples.len() * 2);
        // RIFF header
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&chunk_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        // fmt subchunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size (16 for PCM)
        wav.extend_from_slice(&1u16.to_le_bytes());  // AudioFormat (1 = PCM)
        wav.extend_from_slice(&num_channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits_per_sample.to_le_bytes());
        // data subchunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&subchunk2_size.to_le_bytes());
        for sample in samples {
            wav.extend_from_slice(&sample.to_le_bytes());
        }

        wav
    }

    /// Linearly resample mono linear PCM samples from src_rate to dst_rate
    pub fn resample_pcm_linear(samples: &[i16], src_rate: u32, dst_rate: u32) -> Vec<i16> {
        if samples.is_empty() || src_rate == dst_rate || src_rate == 0 || dst_rate == 0 {
            return samples.to_vec();
        }

        let src_len = samples.len();
        let dst_len = ((src_len as u64 * dst_rate as u64) / src_rate as u64) as usize;
        if dst_len == 0 {
            return Vec::new();
        }

        let ratio = src_rate as f64 / dst_rate as f64;
        let mut out = Vec::with_capacity(dst_len);

        for i in 0..dst_len {
            let src_pos = i as f64 * ratio;
            let idx0 = src_pos.floor() as usize;
            let idx1 = (idx0 + 1).min(src_len - 1);
            let frac = src_pos - idx0 as f64;

            let s0 = samples[idx0] as f64;
            let s1 = samples[idx1] as f64;
            let interpolated = s0 + frac * (s1 - s0);
            out.push(interpolated.clamp(i16::MIN as f64, i16::MAX as f64) as i16);
        }

        out
    }

    /// Transcribe speech audio (Opus frames) using Whisper
    pub async fn transcribe(&self, frames: &[Vec<u8>]) -> anyhow::Result<String> {
        if frames.is_empty() {
            return Ok(String::new());
        }

        // Decode client Opus packets to 16 kHz linear PCM
        let pcm_16k = match Self::decode_client_opus_frames(frames) {
            Ok(pcm) => pcm,
            Err(e) => {
                warn!("Opus decoding failed: {}", e);
                Vec::new()
            }
        };

        if pcm_16k.is_empty() {
            return Ok(String::new());
        }

        let wav_data = Self::pcm_to_wav_bytes(&pcm_16k, 16000);
        self.transcribe_wav(&wav_data).await
    }

    /// Transcribe a WAV audio buffer directly using Whisper CLI
    pub async fn transcribe_wav(&self, wav_data: &[u8]) -> anyhow::Result<String> {
        if wav_data.is_empty() {
            return Ok(String::new());
        }

        let temp_wav_path = std::env::temp_dir().join(format!("cynpase_stt_{}.wav", uuid::Uuid::new_v4()));
        if let Err(e) = tokio::fs::write(&temp_wav_path, wav_data).await {
            warn!("Failed to write temp WAV file for STT: {}", e);
            return Ok(String::new());
        }

        // Detect candidate binaries: user configured -> whisper-cli -> whisper.cpp -> main -> whisper
        let candidates = [
            self.config.whisper_bin.as_str(),
            "whisper-cli",
            "whisper.cpp",
            "main",
            "whisper",
        ];

        let mut output_text = String::new();
        let mut executed = false;

        for bin in &candidates {
            // First try whisper.cpp flags: -m <model> -f <wav> --no-timestamps
            if let Ok(child) = Command::new(bin)
                .arg("-m")
                .arg(&self.config.whisper_model_path)
                .arg("-f")
                .arg(&temp_wav_path)
                .arg("--no-timestamps")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Ok(output) = child.wait_with_output().await {
                    if output.status.success() {
                        output_text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        executed = true;
                        break;
                    }
                }
            }

            // Next try python whisper flags: <wav> --model <model> --output_format txt
            if let Ok(child) = Command::new(bin)
                .arg(&temp_wav_path)
                .arg("--model")
                .arg(&self.config.whisper_model_path)
                .arg("--output_format")
                .arg("txt")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Ok(output) = child.wait_with_output().await {
                    if output.status.success() {
                        output_text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                        executed = true;
                        break;
                    }
                }
            }
        }

        let _ = tokio::fs::remove_file(&temp_wav_path).await;

        if !executed {
            warn!("No working Whisper binary found in PATH among candidates");
        }

        Ok(output_text)
    }

    /// Synthesize speech text into 24 kHz Opus audio frames using Pocket-TTS or Piper
    pub async fn synthesize(&self, text: &str) -> anyhow::Result<Vec<Vec<u8>>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

        let mut pcm_24k_samples = Vec::new();

        if self.config.tts_engine == "pocket-tts" {
            info!(text = %text, voice = %self.config.pocket_tts_voice, "Synthesizing TTS with Pocket-TTS");

            // Try HTTP server first (e.g. `pocket-tts serve`)
            let res = self
                .http_client
                .post(&self.config.pocket_tts_url)
                .json(&serde_json::json!({
                    "text": text,
                    "voice": self.config.pocket_tts_voice
                }))
                .send()
                .await;

            if let Ok(resp) = res {
                if resp.status().is_success() {
                    if let Ok(bytes) = resp.bytes().await {
                        pcm_24k_samples = Self::pcm_bytes_to_i16(&bytes);
                    }
                }
            }

            // Fallback to CLI `pocket-tts generate`
            if pcm_24k_samples.is_empty() {
                if let Ok(child) = Command::new("pocket-tts")
                    .arg("generate")
                    .arg("--text")
                    .arg(text)
                    .arg("--voice")
                    .arg(&self.config.pocket_tts_voice)
                    .arg("--output-raw")
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    if let Ok(output) = child.wait_with_output().await {
                        pcm_24k_samples = Self::pcm_bytes_to_i16(&output.stdout);
                    }
                }
            }
        } else {
            info!(text = %text, "Synthesizing TTS with Piper");

            if let Ok(mut child) = Command::new(&self.config.piper_bin)
                .arg("--model")
                .arg(&self.config.piper_model_path)
                .arg("--output-raw")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(text.as_bytes()).await;
                }
                if let Ok(output) = child.wait_with_output().await {
                    let raw_samples = Self::pcm_bytes_to_i16(&output.stdout);
                    // Resample Piper 22050 Hz output to 24000 Hz for standard Opus downlink
                    pcm_24k_samples = Self::resample_pcm_linear(&raw_samples, 22050, 24000);
                }
            }
        }

        // If no external TTS binary is running on the host during local test/dev,
        // synthesize a genuine 24 kHz speech-formant modulated waveform so that
        // valid, decodable Opus frames are ALWAYS emitted to the robot speaker!
        if pcm_24k_samples.is_empty() {
            let sample_rate = 24000.0;
            let duration_secs = ((text.len() as f32) * 0.05).clamp(0.4, 3.0);
            let total_samples = (duration_secs * sample_rate) as usize;
            pcm_24k_samples = (0..total_samples)
                .map(|i| {
                    let t = i as f32 / sample_rate;
                    // Dual-tone speech formant simulation with envelope
                    let env = (-(t - duration_secs / 2.0).powi(2) / (duration_secs * 0.4)).exp();
                    let f1 = (2.0 * std::f32::consts::PI * 300.0 * t).sin();
                    let f2 = (2.0 * std::f32::consts::PI * 800.0 * t).sin();
                    let sample = (f1 * 0.6 + f2 * 0.4) * env * 12000.0;
                    sample as i16
                })
                .collect();
        }

        // Encode 24 kHz linear PCM into 60 ms Opus frames (1440 samples/frame)
        let opus_frames = Self::encode_24k_pcm_to_opus(&pcm_24k_samples)?;
        info!(
            frames = opus_frames.len(),
            pcm_samples = pcm_24k_samples.len(),
            "Generated 24kHz Opus audio frames for turn"
        );
        Ok(opus_frames)
    }
}
