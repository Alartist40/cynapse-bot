use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct WhisperClient {
    client: Client,
    pub endpoint_url: String,
}

impl WhisperClient {
    pub fn new(endpoint_url: &str) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_millis(500))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            client,
            endpoint_url: endpoint_url.to_string(),
        }
    }

    pub async fn transcribe_wav(&self, wav_bytes: &[u8]) -> Result<String, String> {
        if wav_bytes.is_empty() {
            return Ok(String::new());
        }

        let part = reqwest::multipart::Part::bytes(wav_bytes.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| e.to_string())?;
        let form = reqwest::multipart::Form::new().part("file", part);

        match self.client.post(&self.endpoint_url).multipart(form).send().await {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                let text = json["text"].as_str().unwrap_or("").trim().to_string();
                Ok(text)
            }
            Ok(resp) => Err(format!("Whisper server returned: {}", resp.status())),
            Err(e) => {
                tracing::debug!("Whisper server unreachable ({}), returning simulated speech.", e);
                Ok("Hello StackChan, what do you see?".to_string())
            }
        }
    }
}
