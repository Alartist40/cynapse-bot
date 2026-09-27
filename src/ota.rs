use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::config::HubConfig;

#[derive(Debug, Deserialize)]
pub struct OtaQuery {
    pub token: Option<String>,
}

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
    headers: HeaderMap,
    Query(query): Query<OtaQuery>,
) -> Response {
    let auth_header = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").trim());

    let token = auth_header.or(query.token.as_deref());

    // Prevent leaking master secret token to arbitrary unauthenticated LAN requests
    match token {
        Some(tok) if tok == config.auth_token => {
            Json(OtaResponse {
                websocket: WebSocketConfig {
                    url: config.public_ws_url.clone(),
                    token: config.auth_token.clone(),
                    version: 1,
                },
            }).into_response()
        }
        Some(_) => {
            (StatusCode::UNAUTHORIZED, "Invalid authorization token for OTA provisioning").into_response()
        }
        None => {
            (StatusCode::UNAUTHORIZED, "Missing authorization header for OTA discovery").into_response()
        }
    }
}
