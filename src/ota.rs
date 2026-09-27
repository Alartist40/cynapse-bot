use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::config::HubConfig;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebSocketConfig {
    pub url: String,
    pub token: String,
    pub version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OtaResponse {
    pub websocket: WebSocketConfig,
}

pub async fn handle_ota_discovery(
    State(config): State<Arc<HubConfig>>,
) -> Json<OtaResponse> {
    Json(OtaResponse {
        websocket: WebSocketConfig {
            url: config.public_ws_url.clone(),
            token: config.auth_token.clone(),
            version: 1,
        },
    })
}
