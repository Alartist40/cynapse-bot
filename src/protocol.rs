use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AudioParams {
    #[serde(default = "default_audio_format")]
    pub format: String,
    pub sample_rate: u32,
    #[serde(default = "default_channels")]
    pub channels: u32,
    pub frame_duration: u32,
}

fn default_audio_format() -> String {
    "opus".to_string()
}

fn default_channels() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello {
        version: u32,
        #[serde(default)]
        transport: Option<String>,
        #[serde(default)]
        features: Option<serde_json::Value>,
        #[serde(default)]
        audio_params: Option<AudioParams>,
    },
    Listen {
        state: String, // "start", "stop", "detect"
        #[serde(default)]
        mode: Option<String>,
    },
    Abort {
        #[serde(default)]
        reason: Option<String>,
    },
    Iot {
        #[serde(default)]
        commands: Option<serde_json::Value>,
    },
    Mcp {
        #[serde(default)]
        payload: Option<serde_json::Value>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpCallParams {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpCallPayload {
    #[serde(default = "default_jsonrpc_version")]
    pub jsonrpc: String,
    #[serde(default = "default_mcp_id")]
    pub id: u64,
    #[serde(default = "default_mcp_method")]
    pub method: String,
    pub params: McpCallParams,
}

fn default_jsonrpc_version() -> String {
    "2.0".to_string()
}

use std::sync::atomic::{AtomicU64, Ordering};

static MCP_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn next_mcp_id() -> u64 {
    MCP_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn default_mcp_id() -> u64 {
    next_mcp_id()
}

fn default_mcp_method() -> String {
    "tools/call".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Hello {
        transport: String,
        session_id: String,
        audio_params: AudioParams,
    },
    Tts {
        state: String, // "start", "stop", "sentence_start", "sentence_end"
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
    Stt {
        text: String,
    },
    Llm {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        emotion: Option<String>,
    },
    Mcp {
        payload: McpCallPayload,
    },
    Goodbye,
}

impl ServerMessage {
    pub fn mcp(tool: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self::Mcp {
            payload: McpCallPayload {
                jsonrpc: "2.0".to_string(),
                id: next_mcp_id(),
                method: "tools/call".to_string(),
                params: McpCallParams {
                    name: tool.into(),
                    arguments,
                },
            },
        }
    }
}
