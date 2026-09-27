use clap::Parser;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Parser, Debug, Clone, Serialize, Deserialize)]
#[command(author, version, about = "cynpase-bot Local AI Hub")]
pub struct HubConfig {
    #[arg(long, default_value = "0.0.0.0:8000")]
    pub bind_addr: SocketAddr,

    #[arg(long, default_value = "ws://127.0.0.1:8000/xiaozhi/ws")]
    pub public_ws_url: String,

    #[arg(long, env = "CYNPASE_AUTH_TOKEN", default_value = "cynpase-secret-token")]
    pub auth_token: String,

    #[arg(long, env = "CYNPASE_LEAFCUTTER_URL", default_value = "http://127.0.0.1:8081/v1/chat/completions")]
    pub leafcutter_url: String,

    #[arg(long, env = "CYNPASE_MODEL_NAME", default_value = "qwen2.5-7b")]
    pub model_name: String,
}

impl Default for HubConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:8000".parse().unwrap(),
            public_ws_url: "ws://127.0.0.1:8000/xiaozhi/ws".to_string(),
            auth_token: "cynpase-secret-token".to_string(),
            leafcutter_url: "http://127.0.0.1:8081/v1/chat/completions".to_string(),
            model_name: "qwen2.5-7b".to_string(),
        }
    }
}
