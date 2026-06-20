use serde::Deserialize;
use super::{ApiClient, ApiError};

// Matches shared::types::ServerInfo (snake_case, array returned directly)
#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    pub server_id: String,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner_id: String,
    pub member_count: Option<u32>,
    pub my_role: Option<String>,
}

pub async fn list(client: &ApiClient) -> Result<Vec<Server>, ApiError> {
    let response = client.get("/api/servers").send().await?;
    let status = response.status();
    let raw = response.text().await?;
    tracing::debug!("servers {} → {}", status, raw);

    if status.is_success() {
        serde_json::from_str::<Vec<Server>>(&raw)
            .map_err(|e| ApiError::Server(format!("JSON: {e} — body: {raw}")))
    } else {
        Err(ApiError::Server(format!("HTTP {status}")))
    }
}
