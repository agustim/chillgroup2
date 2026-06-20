use serde::{Deserialize, Serialize};
use super::{ApiClient, ApiError};

#[derive(Debug, Serialize)]
struct LoginRequest<'a> {
    username: &'a str,
    password: &'a str,
}

// Servidor retorna snake_case sense wrapper {success, data}
#[derive(Debug, Clone, Deserialize)]
pub struct LoginData {
    pub user_id: String,
    pub username: String,
    pub token: String,
    pub device_id: String,
    pub device_label: Option<String>,
    pub is_admin: bool,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    error: Option<ErrorDetail>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    message: Option<String>,
}

pub async fn login(
    client: &ApiClient,
    username: &str,
    password: &str,
) -> Result<LoginData, ApiError> {
    let response = client
        .post("/api/auth/login")
        .json(&LoginRequest { username, password })
        .send()
        .await?;

    let status = response.status();
    let raw = response.text().await?;
    tracing::debug!("login {} → {}", status, raw);

    if status.is_success() {
        serde_json::from_str::<LoginData>(&raw)
            .map_err(|e| ApiError::Server(format!("JSON: {e} — body: {raw}")))
    } else if status.as_u16() == 401 {
        Err(ApiError::Unauthorized)
    } else {
        let msg = serde_json::from_str::<ErrorBody>(&raw)
            .ok()
            .and_then(|b| b.error.and_then(|e| e.message).or(b.message))
            .unwrap_or_else(|| format!("HTTP {status}"));
        Err(ApiError::Server(msg))
    }
}
