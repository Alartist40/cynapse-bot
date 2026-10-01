use reqwest::Client;
use serde_json::json;
use std::time::Duration;

#[derive(Clone)]
pub struct OllamaVoiceClient {
    client: Client,
    pub host: String,
    pub model: String,
    pub system_prompt: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub dev_fallbacks: bool,
}

impl OllamaVoiceClient {
    pub fn new(
        host: &str,
        model: &str,
        system_prompt: &str,
        temperature: f32,
        max_tokens: u32,
        dev_fallbacks: bool,
    ) -> Self {
        let client = Client::builder()
            .connect_timeout(Duration::from_millis(500))
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_default();
        Self {
            client,
            host: host.to_string(),
            model: model.to_string(),
            system_prompt: system_prompt.to_string(),
            temperature,
            max_tokens,
            dev_fallbacks,
        }
    }

    pub async fn generate_reply(
        &self,
        user_transcript: &str,
        visual_context: Option<&str>,
    ) -> Result<String, String> {
        let mut system = self.system_prompt.clone();
        if let Some(ctx) = visual_context {
            system.push_str(&format!("\n\n[Live Visual Context]: {}", ctx));
        }

        let messages = vec![
            json!({
                "role": "system",
                "content": system
            }),
            json!({
                "role": "user",
                "content": user_transcript
            }),
        ];

        let url = format!("{}/api/chat", self.host.trim_end_matches('/'));
        let req_body = json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "options": {
                "temperature": self.temperature,
                "num_predict": self.max_tokens
            }
        });

        match self.client.post(&url).json(&req_body).send().await {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
                let content = json["message"]["content"].as_str().unwrap_or("").trim().to_string();
                Ok(content)
            }
            Ok(resp) => {
                if self.dev_fallbacks {
                    Ok(format!("I see you! {}", visual_context.unwrap_or("Everything looks clear.")))
                } else {
                    Err(format!("Ollama server error status: {}", resp.status()))
                }
            }
            Err(e) => {
                if self.dev_fallbacks {
                    Ok(format!("I see you! {}", visual_context.unwrap_or("Everything looks clear.")))
                } else {
                    Err(format!("Ollama server unreachable at {}: {}", self.host, e))
                }
            }
        }
    }
}
