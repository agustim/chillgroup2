//! Endpoints de canals.

#![allow(dead_code)]

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing::{get, post, put},
    Router, extract::Path,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    middleware::{AppState, AuthClaims},
    error::AppError,
    models::{Channel, ChannelType, EncryptionType},
};
use tracing::info;

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

#[derive(Debug, Deserialize)]
pub struct InviteRequest {
    pub username: String,
    #[allow(dead_code)]
    pub encrypted_keys: Vec<EncryptedKey>,
}

#[derive(Debug, Deserialize)]
pub struct EncryptedKey {
    #[allow(dead_code)]
    pub device_id: Uuid,
    #[allow(dead_code)]
    pub encrypted_key: String,
    #[allow(dead_code)]
    pub kem_ciphertext: String,
}

#[derive(Debug, Serialize)]
pub struct InviteResponse {
    pub invited_user: String,
    #[allow(dead_code)]
    pub devices_invited: u32,
}

pub async fn list_channels(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<Vec<Channel>>, AppError> {
    info!("Endpoint list_channels cridat: server_id={}, user_id={}", server_id, claims.user_id);

    // Verificar que l'usuari és membre del servidor
    let role = state.db.is_server_member(server_id, claims.user_id).await
        .map_err(|e| AppError::DatabaseError(e))?;
    if role.is_none() {
        return Err(AppError::Forbidden);
    }

    let channels = state.db.list_channels_for_server(server_id).await.map_err(|e| AppError::DatabaseError(e))?;
    Ok(Json(channels))
}

pub async fn create_channel(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<Channel>), AppError> {
    info!("Endpoint create_channel cridat: server_id={}, name={}, user_id={}", server_id, req.name, claims.user_id);

    // Verificar que l'usuari és membre del servidor
    let role = state.db.is_server_member(server_id, claims.user_id).await
        .map_err(|e| AppError::DatabaseError(e))?;
    if role.is_none() {
        return Err(AppError::Forbidden);
    }

    let channel_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // Guardar a la base de dades
    let channel_type_str = match req.channel_type {
        ChannelType::Text => "text",
        ChannelType::Voice => "voice",
    };
    let encryption_str = match req.encryption_type {
        EncryptionType::None => "none",
        EncryptionType::Symmetric => "symmetric",
        EncryptionType::Asymmetric => "asymmetric",
    };

    state.db.create_channel(
        channel_id,
        server_id,
        &req.name,
        channel_type_str,
        encryption_str,
        req.message_ttl,
        req.is_private,
    ).await.map_err(|e| AppError::DatabaseError(e))?;

    info!("Canal creat i desat a DB: channel_id={}", channel_id);
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
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<Vec<crate::models::ChannelKey>>, AppError> {
    info!("Endpoint get_channel_keys cridat: channel_id={}, user_id={}", channel_id, claims.user_id);
    Ok(Json(vec![]))
}

pub async fn invite_to_channel(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<InviteRequest>,
) -> Result<(StatusCode, Json<InviteResponse>), AppError> {
    info!("Endpoint invite_to_channel cridat: channel_id={}, username={}, user_id={}", channel_id, req.username, claims.user_id);
    Ok((
        StatusCode::CREATED,
        Json(InviteResponse {
            invited_user: req.username,
            devices_invited: 0,
        }),
    ))
}

pub async fn update_channel(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<UpdateChannelRequest>,
) -> Result<Json<Channel>, AppError> {
    info!("Endpoint update_channel cridat: channel_id={}, user_id={}", channel_id, claims.user_id);
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
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    info!("Endpoint delete_channel cridat: channel_id={}, user_id={}", channel_id, claims.user_id);
    Ok(StatusCode::OK)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/servers/{server_id}/channels", get(list_channels).post(create_channel))
        .route("/api/channels/{channel_id}/keys", get(get_channel_keys))
        .route("/api/channels/{channel_id}/invite", post(invite_to_channel))
        .route("/api/channels/{channel_id}", put(update_channel))
        .with_state(state)
}