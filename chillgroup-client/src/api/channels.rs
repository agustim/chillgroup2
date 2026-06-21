use serde::Deserialize;
use super::{ApiClient, ApiError};

// Matches server models::channel::Channel (snake_case, array returned directly)
#[derive(Debug, Clone, Deserialize)]
pub struct Channel {
    pub id: String,               // server uses `id` not `channel_id`
    pub name: String,
    pub channel_type: ChannelType,
    pub encryption_type: EncryptionType,
    pub position: i32,
    pub unread_count: Option<usize>,
    pub permission_level: Option<i32>,
    #[serde(default)]
    pub message_ttl: Option<i32>,
    #[serde(default)]
    pub key_version: Option<i32>,
    #[serde(default)]
    pub key_version_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Text,
    Voice,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionType {
    #[default]
    None,
    Symmetric,
    Asymmetric,
}

impl ChannelType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChannelType::Text => "text",
            ChannelType::Voice => "voice",
        }
    }
}

pub async fn list(client: &ApiClient, server_id: &str) -> Result<Vec<Channel>, ApiError> {
    let response = client
        .get(&format!("/api/servers/{}/channels", server_id))
        .send()
        .await?;
    let status = response.status();
    let raw = response.text().await?;
    tracing::debug!("channels {} → {}", status, raw);

    if status.is_success() {
        let mut channels = serde_json::from_str::<Vec<Channel>>(&raw)
            .map_err(|e| ApiError::Server(format!("JSON: {e} — body: {raw}")))?;
        channels.sort_by_key(|c| c.position);
        Ok(channels)
    } else {
        Err(ApiError::Server(format!("HTTP {status}")))
    }
}
