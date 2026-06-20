use serde::Deserialize;
use super::{ApiClient, ApiError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelKeyBundle {
    pub encrypted_key: String,
    pub kem_ciphertext: String,
    pub key_version_id: Option<String>,
    pub key_version: Option<i32>,
}

pub async fn get_channel_key(client: &ApiClient, channel_id: &str) -> Result<ChannelKeyBundle, ApiError> {
    let response = client
        .get(&format!("/api/channels/{}/keys", channel_id))
        .send()
        .await?;
    let status = response.status();
    let raw = response.text().await?;
    tracing::debug!("channel_key {} → {}", status, &raw[..raw.len().min(300)]);
    if status.is_success() {
        serde_json::from_str::<ChannelKeyBundle>(&raw)
            .map_err(|e| ApiError::Server(format!("JSON: {e} — body: {raw}")))
    } else {
        Err(ApiError::Server(format!("HTTP {status}: {raw}")))
    }
}

pub async fn update_device_public_key(client: &ApiClient, kem_pk_b64: &str) -> Result<(), ApiError> {
    let response = client
        .put("/api/user/me/device/publickey")
        .json(&serde_json::json!({
            "kem_public_key": kem_pk_b64,
            "dsa_public_key": ""
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
