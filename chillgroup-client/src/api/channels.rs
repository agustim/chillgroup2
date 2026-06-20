use serde::Deserialize;
use super::{ApiClient, ApiError, ApiResponse};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub channel_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub channel_type: String,
    pub position: Option<i32>,
    pub encryption_type: Option<String>,
    pub unread_count: Option<i32>,
}

pub async fn list(client: &ApiClient, server_id: &str) -> Result<Vec<Channel>, ApiError> {
    let resp: ApiResponse<Vec<Channel>> = client
        .get(&format!("/api/servers/{}/channels", server_id))
        .send()
        .await?
        .json()
        .await?;

    if resp.success {
        let mut channels = resp.data.unwrap_or_default();
        channels.sort_by_key(|c| c.position.unwrap_or(0));
        Ok(channels)
    } else {
        Err(ApiError::Server(
            resp.error.map(|e| e.message).unwrap_or_else(|| "Error fetching channels".into()),
        ))
    }
}
