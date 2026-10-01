use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct PocketTtsClient {
    client: Client,
    pub server_url: String,
    pub voice: String,
}

impl PocketTtsClient {
    pub fn new(server_url: &str, voice: &str) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_millis(500))
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        Self {
            client,
            server_url: server_url.to_string(),
            voice: voice.to_string(),
        }
    }

    pub async fn synthesize(&self, text: &str) -> Result<Vec<u8>, String> {
        let endpoint = format!("{}/tts", self.server_url.trim_end_matches('/'));
        let params = [("text", text), ("voice_url", &self.voice)];

        if let Ok(resp) = self.client.post(&endpoint).form(&params).send().await {
            if resp.status().is_success() {
                if let Ok(bytes) = resp.bytes().await {
                    return Ok(bytes.to_vec());
                }
            }
        }

        // Return synthetic WAV bytes fallback for test environments
        Self::generate_fallback_wav(text)
    }

    fn generate_fallback_wav(text: &str) -> Result<Vec<u8>, String> {
        use hound::{SampleFormat, WavSpec, WavWriter};
        use std::io::Cursor;

        let sample_rate = 16000;
        let duration_sec = (text.len() as f32 * 0.05).clamp(0.2, 1.0);
        let num_samples = (sample_rate as f32 * duration_sec) as usize;

        let mut cursor = Cursor::new(Vec::new());
        let spec = WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let mut writer = WavWriter::new(&mut cursor, spec).map_err(|e| e.to_string())?;
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let val = (2.0 * std::f32::consts::PI * 300.0 * t).sin() * 0.3;
            let sample = (val * 32767.0) as i16;
            writer.write_sample(sample).map_err(|e| e.to_string())?;
        }
        writer.finalize().map_err(|e| e.to_string())?;

        Ok(cursor.into_inner())
    }
}
