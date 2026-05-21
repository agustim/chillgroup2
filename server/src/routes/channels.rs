//! Endpoints de canals.

#![allow(dead_code)]

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing::{delete, get, post, put},
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

#[derive(Debug, Default, Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub message_ttl: Option<Option<i32>>,
    #[serde(default)]
    pub channel_type: Option<ChannelType>,
    #[serde(default)]
    pub encryption_type: Option<EncryptionType>,
    #[serde(default)]
    pub is_private: Option<bool>,
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

#[derive(Debug, Deserialize)]
pub struct MarkChannelReadRequest {
    #[serde(default)]
    pub last_read_message_id: Option<Uuid>,
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

    let channels = state.db.list_channels_for_server(server_id, claims.user_id).await.map_err(|e| AppError::DatabaseError(e))?;
    Ok(Json(channels))
}

pub async fn mark_channel_read(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<MarkChannelReadRequest>,
) -> Result<StatusCode, AppError> {
    let channel = state.db.get_channel(channel_id).await.map_err(AppError::DatabaseError)?;
    let channel = channel.ok_or(AppError::ChannelNotFound)?;

    let role = state.db.is_server_member(channel.server_id, claims.user_id).await
        .map_err(AppError::DatabaseError)?;
    if role.is_none() {
        return Err(AppError::Forbidden);
    }

    state
        .db
        .mark_channel_read(claims.user_id, channel_id, req.last_read_message_id)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(StatusCode::NO_CONTENT)
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
            unread_count: 0,
            created_at: now,
        }),
    ))
}

pub async fn get_channel_keys(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    info!("Endpoint get_channel_keys cridat: channel_id={}, device_id={}", channel_id, claims.device_id);

    // Verificar que el canal existeix i l'usuari és membre
    let channel = state.db.get_channel(channel_id).await.map_err(AppError::DatabaseError)?;
    let channel = channel.ok_or(AppError::ChannelNotFound)?;
    let role = state.db.is_server_member(channel.server_id, claims.user_id).await
        .map_err(AppError::DatabaseError)?;
    if role.is_none() {
        return Err(AppError::Forbidden);
    }

    // Buscar la clau de canal encriptada per a aquest dispositiu
    let key_entry = state.db
        .get_channel_key_for_device(channel_id, claims.device_id)
        .await
        .map_err(AppError::DatabaseError)?;

    match key_entry {
        Some((encrypted_key, kem_ciphertext)) => Ok(Json(serde_json::json!({
            "success": true,
            "data": {
                "deviceId": claims.device_id,
                "encryptedKey": encrypted_key,
                "kemCiphertext": kem_ciphertext,
            }
        }))),
        None => Err(AppError::ChannelKeyNotFound),
    }
}

#[derive(Debug, Deserialize)]
pub struct ChannelKeyBundle {
    pub device_id: Uuid,
    pub encrypted_key: String,
    pub kem_ciphertext: String,
}

pub async fn upload_channel_keys(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(bundles): Json<Vec<ChannelKeyBundle>>,
) -> Result<StatusCode, AppError> {
    info!("Endpoint upload_channel_keys cridat: channel_id={}, bundles={}", channel_id, bundles.len());

    let channel = state.db.get_channel(channel_id).await.map_err(AppError::DatabaseError)?;
    let channel = channel.ok_or(AppError::ChannelNotFound)?;
    let role = state.db.is_server_member(channel.server_id, claims.user_id).await
        .map_err(AppError::DatabaseError)?;
    if role.is_none() {
        return Err(AppError::Forbidden);
    }

    for bundle in &bundles {
        state.db
            .store_channel_key_for_device(channel_id, bundle.device_id, &bundle.encrypted_key, &bundle.kem_ciphertext)
            .await
            .map_err(AppError::DatabaseError)?;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_channel_member_devices(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    info!("Endpoint get_channel_member_devices cridat: channel_id={}", channel_id);

    let channel = state.db.get_channel(channel_id).await.map_err(AppError::DatabaseError)?;
    let channel = channel.ok_or(AppError::ChannelNotFound)?;
    let role = state.db.is_server_member(channel.server_id, claims.user_id).await
        .map_err(AppError::DatabaseError)?;
    if role.is_none() {
        return Err(AppError::Forbidden);
    }

    let devices = state.db
        .get_member_devices_for_channel(channel_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let data: Vec<serde_json::Value> = devices.iter().map(|(device_id, public_key)| serde_json::json!({
        "deviceId": device_id,
        "publicKey": public_key,
    })).collect();

    Ok(Json(serde_json::json!({ "success": true, "data": data })))
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
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<UpdateChannelRequest>,
) -> Result<Json<Channel>, AppError> {
    info!("Endpoint update_channel cridat: channel_id={}, user_id={}", channel_id, claims.user_id);

    // First, get the existing channel to find its server_id
    let channel = state.db.get_channel(channel_id).await.map_err(|e| AppError::DatabaseError(e))?;
    let channel = channel.ok_or(AppError::ChannelNotFound)?;

    // Verificar que l'usuari és membre del servidor
    let role = state.db.is_server_member(channel.server_id, claims.user_id).await
        .map_err(|e| AppError::DatabaseError(e))?;
    if role.is_none() {
        return Err(AppError::Forbidden);
    }

    // Build partial update
    let name = req.name.as_deref();
    let message_ttl = req.message_ttl.flatten();
    let channel_type_str = match req.channel_type {
        Some(ct) => match ct {
            ChannelType::Text => "text",
            ChannelType::Voice => "voice",
        },
        None => match channel.channel_type {
            ChannelType::Text => "text",
            ChannelType::Voice => "voice",
        },
    };
    let encryption_str = match req.encryption_type {
        Some(et) => match et {
            EncryptionType::None => "none",
            EncryptionType::Symmetric => "symmetric",
            EncryptionType::Asymmetric => "asymmetric",
        },
        None => match channel.encryption_type {
            EncryptionType::None => "none",
            EncryptionType::Symmetric => "symmetric",
            EncryptionType::Asymmetric => "asymmetric",
        },
    };
    let is_private = req.is_private.unwrap_or(channel.is_private);

    state.db.update_channel(
        channel_id,
        channel.server_id,
        name,
        channel_type_str,
        encryption_str,
        message_ttl,
        is_private,
    ).await.map_err(|e| AppError::DatabaseError(e))?;

    // Read back the updated channel
    let updated = state.db.get_channel(channel_id).await
        .map_err(|e| AppError::DatabaseError(e))?
        .ok_or(AppError::ChannelNotFound)?;

    Ok(Json(updated))
}

pub async fn delete_channel(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    info!("Endpoint delete_channel cridat: channel_id={}, user_id={}", channel_id, claims.user_id);

    // Get the channel to verify server membership
    let channel = state.db.get_channel(channel_id).await.map_err(|e| AppError::DatabaseError(e))?;
    let channel = channel.ok_or(AppError::ChannelNotFound)?;

    // Verificar que l'usuari és membre del servidor
    let role = state.db.is_server_member(channel.server_id, claims.user_id).await
        .map_err(|e| AppError::DatabaseError(e))?;
    if role.is_none() {
        return Err(AppError::Forbidden);
    }

    // Delete the channel from DB
    state.db.delete_channel(channel_id).await.map_err(|e| AppError::DatabaseError(e))?;

    info!("Canal eliminat de la DB: channel_id={}", channel_id);
    Ok(StatusCode::OK)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/servers/{server_id}/channels", get(list_channels).post(create_channel))
        .route("/api/channels/{channel_id}/read", post(mark_channel_read))
        .route("/api/channels/{channel_id}/keys", get(get_channel_keys).post(upload_channel_keys))
        .route("/api/channels/{channel_id}/member-devices", get(get_channel_member_devices))
        .route("/api/channels/{channel_id}/invite", post(invite_to_channel))
        .route("/api/channels/{channel_id}", put(update_channel).delete(delete_channel))
        .with_state(state)
}