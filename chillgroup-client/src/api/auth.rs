use serde::{Deserialize, Serialize};
use super::{ApiClient, ApiError, ApiResponse};

#[derive(Debug, Serialize)]
struct LoginRequest<'a> {
    username: &'a str,
    password: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    pub user_id: String,
    pub username: String,
    pub token: String,
    pub device_id: String,
    pub is_admin: bool,
}

pub async fn login(
    client: &ApiClient,
    username: &str,
    password: &str,
) -> Result<LoginData, ApiError> {
    let resp: ApiResponse<LoginData> = client
        .post("/api/auth/login")
        .json(&LoginRequest { username, password })
        .send()
        .await?
        .json()
        .await?;

    if resp.success {
        resp.data.ok_or_else(|| ApiError::Server("Empty data".into()))
    } else {
        let msg = resp.error.map(|e| e.message).unwrap_or_else(|| "Login error".into());
        if msg.to_lowercase().contains("unauthorized") || msg.to_lowercase().contains("invalid") {
            Err(ApiError::Unauthorized)
        } else {
            Err(ApiError::Server(msg))
        }
    }
}
