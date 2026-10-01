use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct WhisperClient {
    client: Client,
    pub endpoint_url: String,
    pub dev_fallbacks: bool,
}

impl WhisperClient {
    pub fn new(endpoint_url: &str, dev_fallbacks: bool) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_millis(500))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self {
            client,
            endpoint_url: endpoint_url.to_string(),
            dev_fallbacks,
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
            Ok(resp) => {
                if self.dev_fallbacks {
                    Ok("Hello StackChan".to_string())
                } else {
                    Err(format!("Whisper server error status: {}", resp.status()))
                }
            }
            Err(e) => {
                if self.dev_fallbacks {
                    Ok("Hello StackChan".to_string())
                } else {
                    Err(format!("Whisper server unreachable at {}: {}", self.endpoint_url, e))
                }
            }
        }
    }
}
