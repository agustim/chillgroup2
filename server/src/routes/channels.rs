//! Endpoints de canals.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing::{get, post, put, delete},
    Router, extract::Path,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    middleware::{AppState, AuthClaims},
    error::AppError,
    models::{Channel, ChannelType, EncryptionType},
};

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    #[serde(default = "default_channel_type")]
    pub channel_type: ChannelType,
    #[serde(default = "default_encryption")]
    pub encryption_type: EncryptionType,
    #[serde(default)]
    pub message_ttl: Option<i32>,
    #[serde(default)]
    pub is_private: bool,
}

fn default_channel_type() -> ChannelType { ChannelType::Text }
fn default_encryption() -> EncryptionType { EncryptionType::None }

#[derive(Debug, Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub message_ttl: Option<Option<i32>>,
}

pub async fn list_channels(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<Vec<Channel>>, AppError> {
    // TODO: Query DB per obtenir canals del servidor
    // SELECT * FROM channels WHERE server_id = $1 AND deleted_at IS NULL

    Ok(Json(vec![]))
}

pub async fn create_channel(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<Channel>), AppError> {
    // TODO: Validar límit de canals
    // TODO: INSERT canal
    // TODO: Si encryption_type != None, generar/claus

    let channel_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    Ok((
        StatusCode::CREATED,
        Json(Channel {
            id: channel_id,
            server_id,
            name: req.name,
            channel_type: req.channel_type,
            encryption_type: req.encryption_type,
            message_ttl: req.message_ttl,
            is_private: req.is_private,
            created_at: now,
        }),
    ))
}

pub async fn get_channel_keys(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<Vec<crate::models::ChannelKey>>, AppError> {
    // TODO: Query DB per obtenir claus del canal per al dispositiu actual
    // SELECT * FROM channel_keys WHERE channel_id = $1 AND device_id = $2

    Ok(Json(vec![]))
}

pub async fn invite_to_channel(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(_req): Json<InviteRequest>,
) -> Result<(StatusCode, Json<InviteResponse>), AppError> {
    // TODO: Buscar usuari per username
    // TODO: Obtenir device_ids de l'usuari
    // TODO: Per cada device, KEM encrypt channel key
    // TODO: INSERT channel_keys

    Ok((
        StatusCode::CREATED,
        Json(InviteResponse {
            invited_user: "target_user".to_string(),
            devices_invited: 0,
        }),
    ))
}

pub async fn update_channel(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<UpdateChannelRequest>,
) -> Result<Json<Channel>, AppError> {
    // TODO: UPDATE canal

    Ok(Json(Channel {
        id: channel_id,
        server_id: Uuid::nil(),
        name: req.name.unwrap_or_else(|| "updated".to_string()),
        channel_type: ChannelType::Text,
        encryption_type: EncryptionType::None,
        message_ttl: req.message_ttl.flatten(),
        is_private: false,
        created_at: chrono::Utc::now(),
    }))
}

pub async fn delete_channel(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    // TODO: DELETE canal (soft delete amb deleted_at)

    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub username: String,
    pub encrypted_keys: Vec<EncryptedKey>,
}

#[derive(Debug, Deserialize)]
pub struct EncryptedKey {
    pub device_id: Uuid,
    pub encrypted_key: String,
    pub kem_ciphertext: String,
}

#[derive(Debug, Serialize)]
pub struct InviteResponse {
    pub invited_user: String,
    pub devices_invited: u32,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/servers/:server_id/channels", get(list_channels).post(create_channel))
        .route("/api/channels/:channel_id/keys", get(get_channel_keys))
        .route("/api/channels/:channel_id/invite", post(invite_to_channel))
        .route("/api/channels/:channel_id", put(update_channel).delete(delete_channel))
        .with_state(state)
}