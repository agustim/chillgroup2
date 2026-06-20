use serde::Deserialize;
use super::{ApiClient, ApiError, ApiResponse};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub server_id: String,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner_id: String,
}

pub async fn list(client: &ApiClient) -> Result<Vec<Server>, ApiError> {
    let resp: ApiResponse<Vec<Server>> = client
        .get("/api/servers")
        .send()
        .await?
        .json()
        .await?;

    if resp.success {
        Ok(resp.data.unwrap_or_default())
    } else {
        Err(ApiError::Server(
            resp.error.map(|e| e.message).unwrap_or_else(|| "Error fetching servers".into()),
        ))
    }
}
