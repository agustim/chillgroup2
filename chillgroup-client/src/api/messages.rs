use serde::{Deserialize, Serialize};
use super::{ApiClient, ApiError, ApiResponse};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub message_id: String,
    pub channel_id: String,
    pub author_id: String,
    pub author_username: String,
    pub content: String,
    pub iv: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
struct SendMessageRequest<'a> {
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    iv: Option<&'a str>,
}

pub async fn list(
    client: &ApiClient,
    channel_id: &str,
    limit: u32,
) -> Result<Vec<Message>, ApiError> {
    let resp: ApiResponse<Vec<Message>> = client
        .get(&format!("/api/channels/{}/messages?limit={}", channel_id, limit))
        .send()
        .await?
        .json()
        .await?;

    if resp.success {
        Ok(resp.data.unwrap_or_default())
    } else {
        Err(ApiError::Server(
            resp.error.map(|e| e.message).unwrap_or_else(|| "Error fetching messages".into()),
        ))
    }
}

pub async fn send(
    client: &ApiClient,
    channel_id: &str,
    content: &str,
) -> Result<Message, ApiError> {
    let resp: ApiResponse<Message> = client
        .post(&format!("/api/channels/{}/messages", channel_id))
        .json(&SendMessageRequest { content, iv: None })
        .send()
        .await?
        .json()
        .await?;

    if resp.success {
        resp.data.ok_or_else(|| ApiError::Server("Empty data".into()))
    } else {
        Err(ApiError::Server(
            resp.error.map(|e| e.message).unwrap_or_else(|| "Error sending message".into()),
        ))
    }
}
