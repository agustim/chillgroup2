use serde::{Deserialize, Serialize};
use super::{ApiClient, ApiError};

// Matches shared::types::MessageInfo (snake_case)
#[derive(Debug, Clone, Deserialize)]
pub struct MessageInfo {
    pub message_id: String,
    pub channel_id: String,
    pub sender_user_id: String,
    pub sender_username: String,
    pub encrypted_payload: String,  // plaintext for encryption_type=none channels
    pub iv: String,                  // empty string for none channels
    pub timestamp: String,
}

// PaginatedResponse wrapper from server
#[derive(Debug, Deserialize)]
pub struct PaginatedResponse {
    pub data: Vec<MessageInfo>,
}

// For encryption_type=none: encrypted_payload = plaintext, iv = ""
// For encrypted channels: caller must decrypt encrypted_payload with iv
#[derive(Debug, Serialize)]
struct SendMessageRequest<'a> {
    encrypted_payload: &'a str,
    iv: &'a str,
}

pub async fn list(
    client: &ApiClient,
    channel_id: &str,
    limit: u32,
) -> Result<Vec<MessageInfo>, ApiError> {
    let response = client
        .get(&format!("/api/channels/{}/messages?limit={}", channel_id, limit))
        .send()
        .await?;
    let status = response.status();
    let raw = response.text().await?;
    tracing::debug!("messages {} → {}", status, &raw[..raw.len().min(200)]);

    if status.is_success() {
        let paginated = serde_json::from_str::<PaginatedResponse>(&raw)
            .map_err(|e| ApiError::Server(format!("JSON: {e} — body: {raw}")))?;
        Ok(paginated.data)
    } else {
        Err(ApiError::Server(format!("HTTP {status}")))
    }
}

// Send plaintext message to a none-encryption channel
pub async fn send_plain(
    client: &ApiClient,
    channel_id: &str,
    content: &str,
) -> Result<(), ApiError> {
    let response = client
        .post(&format!("/api/channels/{}/messages", channel_id))
        .json(&SendMessageRequest { encrypted_payload: content, iv: "" })
        .send()
        .await?;
    let status = response.status();
    if status.is_success() {
        Ok(())
    } else {
        let raw = response.text().await.unwrap_or_default();
        Err(ApiError::Server(format!("HTTP {status}: {raw}")))
    }
}
