use serde::Deserialize;
use super::{ApiClient, ApiError};

#[derive(Debug, Deserialize)]
pub struct LiveKitTokenResponse {
    pub token: String,
    pub url: String,
}

pub async fn get_token(client: &ApiClient, channel_id: &str) -> Result<LiveKitTokenResponse, ApiError> {
    let response = client
        .post("/api/livekit/token")
        .json(&serde_json::json!({ "room": format!("chillgroup-{}", channel_id) }))
        .send()
        .await?;
    let status = response.status();
    let raw = response.text().await?;
    tracing::debug!("livekit token {} → {}", status, &raw[..raw.len().min(200)]);
    if status.is_success() {
        serde_json::from_str::<LiveKitTokenResponse>(&raw)
            .map_err(|e| ApiError::Server(format!("JSON: {e} — body: {raw}")))
    } else {
        Err(ApiError::Server(format!("HTTP {status}: {raw}")))
    }
}
