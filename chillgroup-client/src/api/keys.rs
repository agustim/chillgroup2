use serde::{Deserialize, Serialize};
use super::{ApiClient, ApiError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelKeyBundle {
    pub encrypted_key: String,
    pub kem_ciphertext: String,
    pub key_version_id: Option<String>,
    pub key_version: Option<i32>,
    pub signature: Option<String>,
    pub signed_by_device_id: Option<String>,
    pub device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllKeyBundle {
    pub device_id: String,
    pub key_version_id: Option<String>,
    pub key_version: Option<i32>,
    pub encrypted_key: String,
    pub kem_ciphertext: String,
    pub signature: Option<String>,
    pub signed_by_device_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RotateKeyResult {
    pub key_version_id: String,
    pub key_version: i32,
}

// Server wraps all responses: { "success": true, "data": { ... } }
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    data: T,
}

pub async fn get_channel_key(client: &ApiClient, channel_id: &str) -> Result<ChannelKeyBundle, ApiError> {
    let response = client
        .get(&format!("/api/channels/{}/keys", channel_id))
        .send()
        .await?;
    let status = response.status();
    let raw = response.text().await?;
    tracing::debug!("channel_key {} → {}", status, &raw[..raw.len().min(400)]);
    if status.is_success() {
        serde_json::from_str::<ApiResponse<ChannelKeyBundle>>(&raw)
            .map(|r| r.data)
            .map_err(|e| ApiError::Server(format!("JSON: {e} — body: {raw}")))
    } else {
        Err(ApiError::Server(format!("HTTP {status}: {raw}")))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberDevice {
    pub device_id: String,
    #[serde(default)]
    pub kem_public_key: String,
    #[serde(default)]
    pub dsa_public_key: String,
}

pub async fn get_member_devices(client: &ApiClient, channel_id: &str) -> Result<Vec<MemberDevice>, ApiError> {
    let response = client
        .get(&format!("/api/channels/{}/member-devices", channel_id))
        .send()
        .await?;
    let status = response.status();
    let raw = response.text().await?;
    if status.is_success() {
        serde_json::from_str::<ApiResponse<Vec<MemberDevice>>>(&raw)
            .map(|r| r.data)
            .map_err(|e| ApiError::Server(format!("JSON: {e} — body: {raw}")))
    } else {
        Err(ApiError::Server(format!("HTTP {status}: {raw}")))
    }
}

#[derive(Debug, Serialize)]
pub struct KeyBundle {
    pub device_id: String,
    pub encrypted_key: String,
    pub kem_ciphertext: String,
    pub key_version: Option<i32>,
    pub signature: Option<String>,
    pub signed_by_device_id: Option<String>,
}

pub async fn upload_key_bundles(client: &ApiClient, channel_id: &str, bundles: &[KeyBundle]) -> Result<(), ApiError> {
    let response = client
        .post(&format!("/api/channels/{}/keys", channel_id))
        .json(bundles)
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

pub async fn get_all_key_bundles(client: &ApiClient, channel_id: &str) -> Result<Vec<AllKeyBundle>, ApiError> {
    let response = client
        .get(&format!("/api/channels/{}/keys/all", channel_id))
        .send()
        .await?;
    let status = response.status();
    let raw = response.text().await?;
    if status.is_success() {
        serde_json::from_str::<Vec<AllKeyBundle>>(&raw)
            .map_err(|e| ApiError::Server(format!("JSON: {e} — body: {raw}")))
    } else {
        Err(ApiError::Server(format!("HTTP {status}: {raw}")))
    }
}

pub async fn rotate_channel_key(client: &ApiClient, channel_id: &str) -> Result<RotateKeyResult, ApiError> {
    let response = client
        .post(&format!("/api/channels/{}/keys/rotate", channel_id))
        .json(&serde_json::json!({}))
        .send()
        .await?;
    let status = response.status();
    let raw = response.text().await?;
    if status.is_success() {
        serde_json::from_str::<RotateKeyResult>(&raw)
            .map_err(|e| ApiError::Server(format!("JSON: {e} — body: {raw}")))
    } else {
        Err(ApiError::Server(format!("HTTP {status}: {raw}")))
    }
}

pub async fn update_device_public_key(client: &ApiClient, kem_pk_b64: &str, dsa_pk_b64: &str) -> Result<(), ApiError> {
    let response = client
        .put("/api/user/me/device/publickey")
        .json(&serde_json::json!({
            "kem_public_key": kem_pk_b64,
            "dsa_public_key": dsa_pk_b64
        }))
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
