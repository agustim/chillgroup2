use serde::{Deserialize, Serialize};
use super::{ApiClient, ApiError};

// Matches server models::message::Message
#[derive(Debug, Clone, Deserialize)]
pub struct MessageInfo {
    pub id: String,
    pub channel_id: String,
    pub sender_user_id: String,
    #[serde(default)]
    pub sender_username: Option<String>,
    pub encrypted_payload: String,
    #[serde(default)]
    pub iv: String,
    pub key_version: Option<i32>,
    pub timestamp: String,
}

// PaginatedResponse wrapper from server
#[derive(Debug, Deserialize)]
pub struct PaginatedResponse {
    pub data: Vec<MessageInfo>,
}

#[derive(Debug, Serialize)]
struct SendMessageRequest<'a> {
    encrypted_payload: &'a str,
    iv: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    key_version: Option<i32>,
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
        let mut paginated = serde_json::from_str::<PaginatedResponse>(&raw)
            .map_err(|e| ApiError::Server(format!("JSON: {e} — body: {raw}")))?;
        // Server returns newest-first; reverse to oldest-first for display
        paginated.data.reverse();
        Ok(paginated.data)
    } else {
        Err(ApiError::Server(format!("HTTP {status}")))
    }
}

pub async fn send(
    client: &ApiClient,
    channel_id: &str,
    encrypted_payload: &str,
    iv: &str,
    key_version: Option<i32>,
) -> Result<(), ApiError> {
    let response = client
        .post(&format!("/api/channels/{}/messages", channel_id))
        .json(&SendMessageRequest { encrypted_payload, iv, key_version })
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
