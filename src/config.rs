use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub vision_orchestrator: VisionOrchestratorConfig,
    #[serde(default)]
    pub mqtt: MqttConfig,
    #[serde(default)]
    pub stt: SttConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub tts: TtsConfig,
    #[serde(default)]
    pub audio: AudioConfig,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionOrchestratorConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_false")]
    pub simulated_vision: bool,
    #[serde(default = "default_conf")]
    pub confidence_threshold: f32,
    #[serde(default = "default_true")]
    pub mirror_pan: bool,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    8088
}
fn default_false() -> bool {
    false
}
fn default_true() -> bool {
    true
}
fn default_conf() -> f32 {
    0.5
}

impl Default for VisionOrchestratorConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            simulated_vision: default_false(),
            confidence_threshold: default_conf(),
            mirror_pan: default_true(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttConfig {
    #[serde(default = "default_mqtt_host")]
    pub host: String,
    #[serde(default = "default_mqtt_port")]
    pub port: u16,
    #[serde(default = "default_client_id")]
    pub client_id: String,
    #[serde(default = "default_reconnect_interval")]
    pub reconnect_interval_sec: u64,
}

fn default_mqtt_host() -> String {
    "127.0.0.1".to_string()
}
fn default_mqtt_port() -> u16 {
    1883
}
fn default_client_id() -> String {
    "localmind-orchestrator".to_string()
}
fn default_reconnect_interval() -> u64 {
    5
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            host: default_mqtt_host(),
            port: default_mqtt_port(),
            client_id: default_client_id(),
            reconnect_interval_sec: default_reconnect_interval(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttConfig {
    #[serde(default = "default_stt_url")]
    pub url: String,
    #[serde(default = "default_stt_model")]
    pub model_size: String,
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_stt_url() -> String {
    "http://127.0.0.1:8080/inference".to_string()
}
fn default_stt_model() -> String {
    "small".to_string()
}
fn default_language() -> String {
    "en".to_string()
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            url: default_stt_url(),
            model_size: default_stt_model(),
            language: default_language(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_llm_host")]
    pub host: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
    #[serde(default = "default_system_prompt_path")]
    pub system_prompt_path: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_llm_host() -> String {
    "http://127.0.0.1:11434".to_string()
}
fn default_llm_model() -> String {
    "qwen2.5:3b-instruct".to_string()
}
fn default_system_prompt_path() -> String {
    "hub/persona/system.md".to_string()
}
fn default_temperature() -> f32 {
    0.7
}
fn default_max_tokens() -> u32 {
    100
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            host: default_llm_host(),
            model: default_llm_model(),
            system_prompt_path: default_system_prompt_path(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsConfig {
    #[serde(default = "default_tts_url")]
    pub url: String,
    #[serde(default = "default_tts_voice")]
    pub voice: String,
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    #[serde(default = "default_frame_duration")]
    pub frame_duration_ms: u32,
}

fn default_tts_url() -> String {
    "http://127.0.0.1:8000".to_string()
}
fn default_tts_voice() -> String {
    "alba".to_string()
}
fn default_sample_rate() -> u32 {
    16000
}
fn default_frame_duration() -> u32 {
    60
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            url: default_tts_url(),
            voice: default_tts_voice(),
            sample_rate: default_sample_rate(),
            frame_duration_ms: default_frame_duration(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    #[serde(default = "default_channels")]
    pub channels: u16,
    #[serde(default = "default_frame_duration")]
    pub frame_duration_ms: u32,
    #[serde(default = "default_frame_size")]
    pub frame_size: usize,
}

fn default_channels() -> u16 {
    1
}
fn default_frame_size() -> usize {
    960
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: default_sample_rate(),
            channels: default_channels(),
            frame_duration_ms: default_frame_duration(),
            frame_size: default_frame_size(),
        }
    }
}

impl AppConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Self {
        let path_ref = path.as_ref();
        match std::fs::read_to_string(path_ref) {
            Ok(content) => match serde_yaml::from_str(&content) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse config file {:?}: {}. Using default configuration.",
                        path_ref,
                        e
                    );
                    Self::default()
                }
            },
            Err(e) => {
                tracing::info!(
                    "Config file {:?} not found or unreadable ({}). Using default configuration.",
                    path_ref,
                    e
                );
                Self::default()
            }
        }
    }
}

