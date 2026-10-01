pub mod llm;
pub mod tts;
pub mod whisper;

pub use llm::OllamaVoiceClient;
pub use tts::PocketTtsClient;
pub use whisper::WhisperClient;

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub struct VoiceOrchestrator {
    pub whisper: WhisperClient,
    pub llm: OllamaVoiceClient,
    pub tts: PocketTtsClient,
    last_narration_instant: std::sync::Mutex<Option<Instant>>,
    narration_cooldown: Duration,
    pub total_conversations: AtomicU64,
}

impl VoiceOrchestrator {
    pub fn new(
        whisper_url: &str,
        ollama_host: &str,
        ollama_model: &str,
        system_prompt: &str,
        temperature: f32,
        max_tokens: u32,
        tts_url: &str,
        tts_voice: &str,
        dev_fallbacks: bool,
    ) -> Self {
        Self {
            whisper: WhisperClient::new(whisper_url, dev_fallbacks),
            llm: OllamaVoiceClient::new(
                ollama_host,
                ollama_model,
                system_prompt,
                temperature,
                max_tokens,
                dev_fallbacks,
            ),
            tts: PocketTtsClient::new(tts_url, tts_voice, dev_fallbacks),
            last_narration_instant: std::sync::Mutex::new(None),
            narration_cooldown: Duration::from_secs(30), // FR-V4: 30s rate limit
            total_conversations: AtomicU64::new(0),
        }
    }

    /// Process a conversational voice turn: Audio in -> STT -> LLM (with visual context) -> TTS -> Audio out
    pub async fn process_turn(
        &self,
        wav_audio_in: &[u8],
        visual_context: Option<&str>,
    ) -> Result<(String, String, Vec<u8>), String> {
        let transcript = self.whisper.transcribe_wav(wav_audio_in).await?;
        if transcript.is_empty() {
            return Ok((String::new(), String::new(), Vec::new()));
        }

        let reply = self.llm.generate_reply(&transcript, visual_context).await?;
        let audio_out = self.tts.synthesize(&reply).await?;
        self.total_conversations.fetch_add(1, Ordering::Relaxed);

        Ok((transcript, reply, audio_out))
    }

    /// Rate-limited narration triggered autonomously by perception
    pub async fn trigger_vision_narration(
        &self,
        event_description: &str,
    ) -> Option<(String, Vec<u8>)> {
        {
            let mut last = self.last_narration_instant.lock().unwrap();
            if let Some(prev) = *last {
                if prev.elapsed() < self.narration_cooldown {
                    return None;
                }
            }
            *last = Some(Instant::now());
        }

        let prompt = format!("Say one short observation about: {}", event_description);
        if let Ok(reply) = self.llm.generate_reply(&prompt, None).await {
            if let Ok(audio) = self.tts.synthesize(&reply).await {
                return Some((reply, audio));
            }
        }

        None
    }
}
