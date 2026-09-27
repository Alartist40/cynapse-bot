use crate::audio::{AudioConfig, AudioEngine};
use crate::config::HubConfig;
use crate::mcp::{HardwareAction, McpDispatcher};
use crate::persona::{PersonaConfig, PersonaManager};
use crate::protocol::ServerMessage;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

pub enum PipelineTurnResult {
    FastAction {
        action: String,
        expression: Option<String>,
        reply_text: String,
    },
    LlmReply {
        text: String,
        emotion: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct TurnExecution {
    pub messages: Vec<ServerMessage>,
    pub audio_frames: Vec<Vec<u8>>, // 24 kHz Opus binary packets
    pub spoken_text: String,
}

#[derive(Clone)]
pub struct PipelineEngine {
    config: Arc<HubConfig>,
    http_client: reqwest::Client,
    pub audio_engine: Arc<AudioEngine>,
    pub persona_manager: Arc<Mutex<PersonaManager>>,
}

impl PipelineEngine {
    pub fn new(config: Arc<HubConfig>) -> Self {
        let audio_engine = Arc::new(AudioEngine::new(AudioConfig::default()));
        let persona_manager = Arc::new(Mutex::new(PersonaManager::new(PersonaConfig::default())));
        Self {
            config,
            http_client: reqwest::Client::new(),
            audio_engine,
            persona_manager,
        }
    }

    /// Fast rule tier 1 matching
    pub fn try_fast_action(&self, input: &str) -> Option<PipelineTurnResult> {
        if let Some(hw_action) = McpDispatcher::parse_command_text(input) {
            match hw_action {
                HardwareAction::EmergencyStop => Some(PipelineTurnResult::FastAction {
                    action: "stop".to_string(),
                    expression: Some("neutral".to_string()),
                    reply_text: "Stopping all movement immediately.".to_string(),
                }),
                HardwareAction::PlayAnimation { animation } => Some(PipelineTurnResult::FastAction {
                    action: format!("play_{}", animation),
                    expression: Some("happy".to_string()),
                    reply_text: format!("Playing {} animation.", animation),
                }),
                HardwareAction::MoveServo { pan, tilt } => Some(PipelineTurnResult::FastAction {
                    action: format!("servo_{}_{}", pan, tilt),
                    expression: Some("curious".to_string()),
                    reply_text: "Adjusting head position.".to_string(),
                }),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Execute a turn given transcribed user text, generating both text messages and Opus audio
    pub async fn process_text_turn(&self, text: &str) -> anyhow::Result<TurnExecution> {
        let mut messages = Vec::new();

        // 1. Acknowledge STT
        messages.push(ServerMessage::Stt {
            text: text.to_string(),
        });

        // 2. Check Tier 1 Fast Actions
        if let Some(fast) = self.try_fast_action(text) {
            match fast {
                PipelineTurnResult::FastAction { action, expression, reply_text } => {
                    messages.push(ServerMessage::Action { action, expression });
                    messages.push(ServerMessage::Tts {
                        state: "start".to_string(),
                        text: Some(reply_text.clone()),
                    });
                    messages.push(ServerMessage::Tts {
                        state: "stop".to_string(),
                        text: None,
                    });

                    // Synthesize real 24 kHz Opus audio frames
                    let audio_frames = self.audio_engine.synthesize(&reply_text).await.unwrap_or_default();

                    return Ok(TurnExecution {
                        messages,
                        audio_frames,
                        spoken_text: reply_text,
                    });
                }
                _ => {}
            }
        }

        // 3. Fallback to Tier 3 LLM (Leafcutter / OpenAI seam)
        let system_prompt = {
            let persona = self.persona_manager.lock().await;
            persona.build_system_prompt()
        };

        info!(model = %self.config.model_name, "Calling Leafcutter LLM");
        let payload = json!({
            "model": self.config.model_name,
            "messages": [
                {
                    "role": "system",
                    "content": system_prompt
                },
                {
                    "role": "user",
                    "content": text
                }
            ],
            "max_tokens": 128,
            "temperature": 0.7
        });

        let reply_text = match self
            .http_client
            .post(&self.config.leafcutter_url)
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let body: serde_json::Value = resp.json().await.unwrap_or_default();
                body["choices"][0]["message"]["content"]
                    .as_str()
                    .unwrap_or("I understood your message.")
                    .to_string()
            }
            Ok(resp) => {
                format!("Leafcutter LLM returned status: {}", resp.status())
            }
            Err(e) => {
                format!("Leafcutter LLM offline fallback ({}).", e)
            }
        };

        // Record turn in Dendrite memory
        {
            let persona = self.persona_manager.lock().await;
            let _ = persona.record_interaction(&format!("Turn: {}", text), &reply_text);
        }

        messages.push(ServerMessage::Llm {
            text: reply_text.clone(),
            emotion: Some("neutral".to_string()),
        });
        messages.push(ServerMessage::Tts {
            state: "start".to_string(),
            text: Some(reply_text.clone()),
        });
        messages.push(ServerMessage::Tts {
            state: "stop".to_string(),
            text: None,
        });

        // Synthesize real 24 kHz Opus audio frames
        let audio_frames = self.audio_engine.synthesize(&reply_text).await.unwrap_or_default();

        Ok(TurnExecution {
            messages,
            audio_frames,
            spoken_text: reply_text,
        })
    }

    /// Process audio utterance frames
    pub async fn process_audio_frames(&self, frames: Vec<Vec<u8>>) -> anyhow::Result<TurnExecution> {
        let transcribed_text = self.audio_engine.transcribe(&frames).await?;
        let query = if transcribed_text.trim().is_empty() {
            "Hello"
        } else {
            &transcribed_text
        };
        self.process_text_turn(query).await
    }
}
