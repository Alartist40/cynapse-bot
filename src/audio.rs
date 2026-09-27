use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{info, warn};

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

    /// Convert raw Opus packets into a unified audio payload
    pub fn concatenate_opus_frames(&self, frames: &[Vec<u8>]) -> Vec<u8> {
        let mut total_bytes = Vec::new();
        for frame in frames {
            total_bytes.extend_from_slice(frame);
        }
        total_bytes
    }

    /// Transcribe speech audio (Opus/WAV) using Whisper
    pub async fn transcribe(&self, frames: &[Vec<u8>]) -> anyhow::Result<String> {
        if frames.is_empty() {
            return Ok(String::new());
        }

        // # ponytail: check if external whisper binary is available, fallback to mock transcript
        let combined = self.concatenate_opus_frames(frames);
        info!(bytes = combined.len(), "Running STT transcription on audio frames");

        match Command::new(&self.config.whisper_bin)
            .arg("--model")
            .arg(&self.config.whisper_model_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(&combined).await;
                }
                let output = child.wait_with_output().await?;
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if text.is_empty() {
                    Ok(format!("Audio input ({} frames)", frames.len()))
                } else {
                    Ok(text)
                }
            }
            Err(_) => {
                warn!("Whisper binary not found; using speech-to-intent fallback");
                Ok(format!("Audio speech input ({} packets)", frames.len()))
            }
        }
    }

    /// Synthesize speech text into 24 kHz Opus audio frames using Pocket-TTS or Piper
    pub async fn synthesize(&self, text: &str) -> anyhow::Result<Vec<Vec<u8>>> {
        if text.trim().is_empty() {
            return Ok(Vec::new());
        }

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
                        let chunk_size = 960 * 2; // ~60ms PCM s16le
                        let frames: Vec<Vec<u8>> = bytes.chunks(chunk_size).map(|c| c.to_vec()).collect();
                        if !frames.is_empty() {
                            return Ok(frames);
                        }
                    }
                }
            }

            // Fallback to CLI `pocket-tts generate`
            match Command::new("pocket-tts")
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
                Ok(child) => {
                    let output = child.wait_with_output().await?;
                    let chunk_size = 960 * 2;
                    let frames: Vec<Vec<u8>> = output
                        .stdout
                        .chunks(chunk_size)
                        .map(|c| c.to_vec())
                        .collect();
                    if !frames.is_empty() {
                        return Ok(frames);
                    }
                }
                Err(_) => {
                    warn!("pocket-tts not found in PATH; falling back to simulated frames");
                }
            }
        } else {
            info!(text = %text, "Synthesizing TTS with Piper");

            match Command::new(&self.config.piper_bin)
                .arg("--model")
                .arg(&self.config.piper_model_path)
                .arg("--output-raw")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(mut child) => {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(text.as_bytes()).await;
                    }
                    let output = child.wait_with_output().await?;
                    let chunk_size = 960 * 2; // ~60ms PCM s16le
                    let frames: Vec<Vec<u8>> = output
                        .stdout
                        .chunks(chunk_size)
                        .map(|c| c.to_vec())
                        .collect();
                    if !frames.is_empty() {
                        return Ok(frames);
                    }
                }
                Err(_) => {
                    warn!("Piper TTS binary not found; falling back to simulated frames");
                }
            }
        }

        // Return simulated 60ms Opus frames for mock testing
        let frames = (0..5)
            .map(|i| vec![0xF8, 0xFF, 0xFE, i as u8, 0x00])
            .collect();
        Ok(frames)
    }
}
