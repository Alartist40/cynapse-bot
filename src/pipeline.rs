use crate::audio::{AudioConfig, AudioEngine};
use crate::config::HubConfig;
use crate::mcp::{HardwareAction, McpDispatcher};
use crate::persona::{PersonaConfig, PersonaManager};
use crate::protocol::ServerMessage;
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

pub enum PipelineTurnResult {
    FastAction {
        tool: String,
        arguments: serde_json::Value,
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
    pub history: Arc<Mutex<Vec<(String, String)>>>,
}

impl PipelineEngine {
    pub fn new(config: Arc<HubConfig>) -> Self {
        let audio_engine = Arc::new(AudioEngine::new(AudioConfig::default()));
        let persona = match PersonaManager::new(PersonaConfig::default()).with_memory("data/mazzaroth.db") {
            Ok(p) => p,
            Err(_) => PersonaManager::new(PersonaConfig::default())
                .with_memory(":memory:")
                .unwrap_or_else(|_| PersonaManager::new(PersonaConfig::default())),
        };
        let persona_manager = Arc::new(Mutex::new(persona));
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap_or_default();

        Self {
            config,
            http_client,
            audio_engine,
            persona_manager,
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Fast rule tier 1 matching
    pub fn try_fast_action(&self, input: &str) -> Option<PipelineTurnResult> {
        if let Some(hw_action) = McpDispatcher::parse_command_text(input) {
            let (tool, args, reply_text) = match hw_action {
                HardwareAction::EmergencyStop => (
                    "self.robot.set_head_angles".to_string(),
                    json!({ "yaw": 0, "pitch": 0, "speed": 500 }),
                    "Stopping all movement immediately.".to_string(),
                ),
                HardwareAction::MoveServo { pan, tilt } => (
                    "self.robot.set_head_angles".to_string(),
                    json!({ "yaw": pan, "pitch": tilt, "speed": 150 }),
                    "Adjusting head position.".to_string(),
                ),
                HardwareAction::SetLedColor { red, green, blue } => (
                    "self.robot.set_led_color".to_string(),
                    json!({ "red": red, "green": green, "blue": blue }),
                    "Setting LED color.".to_string(),
                ),
                HardwareAction::PlayAnimation { ref animation } => {
                    let (yaw, pitch) = match animation.to_lowercase().as_str() {
                        "dance" => (25, 15),
                        "nod" => (0, -20),
                        "shake" => (-35, 0),
                        "wave" => (15, 20),
                        "look_around" => (-50, 10),
                        "sleep" => (0, -25),
                        "wake" => (0, 15),
                        _ => (0, 0),
                    };
                    (
                        "self.robot.set_head_angles".to_string(),
                        json!({ "yaw": yaw, "pitch": pitch, "speed": 300 }),
                        format!("Playing {} motion.", animation),
                    )
                }
                _ => (
                    "self.robot.get_head_angles".to_string(),
                    json!({}),
                    "Checking robot status.".to_string(),
                ),
            };

            Some(PipelineTurnResult::FastAction {
                tool,
                arguments: args,
                reply_text,
            })
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
                PipelineTurnResult::FastAction { tool, arguments, reply_text } => {
                    messages.push(ServerMessage::Mcp { tool, arguments });
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

        let mut llm_messages = vec![
            json!({
                "role": "system",
                "content": system_prompt
            }),
        ];

        // Append recent multi-turn history
        {
            let hist = self.history.lock().await;
            for (role, content) in hist.iter().rev().take(10).rev() {
                llm_messages.push(json!({
                    "role": role,
                    "content": content
                }));
            }
        }

        llm_messages.push(json!({
            "role": "user",
            "content": text
        }));

        info!(model = %self.config.model_name, "Calling Leafcutter LLM");
        let payload = json!({
            "model": self.config.model_name,
            "messages": llm_messages,
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
                warn!("Leafcutter LLM returned status: {}", resp.status());
                "I am here, but having trouble reaching my neural engine right now.".to_string()
            }
            Err(e) => {
                warn!("Leafcutter LLM offline or unreachable: {}", e);
                "I am listening, but my local language model is currently offline.".to_string()
            }
        };

        // Update multi-turn history ring buffer
        {
            let mut hist = self.history.lock().await;
            hist.push(("user".to_string(), text.to_string()));
            hist.push(("assistant".to_string(), reply_text.clone()));
            if hist.len() > 20 {
                let excess = hist.len() - 20;
                hist.drain(0..excess);
            }
        }

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
