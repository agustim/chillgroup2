//! Endpoints de canals.

#![allow(dead_code)]

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
    routing::{get, post, put},
    Router,
};
use aes_gcm::{Aes256Gcm, Nonce, aead::{Aead, KeyInit}};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ml_kem::{Encapsulate, ml_kem_1024};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    middleware::{AppState, AuthClaims},
    error::AppError,
    models::{Channel, ChannelType, EncryptionType},
};
use tracing::info;

const AES_GCM_NONCE_SIZE: usize = 12;

#[derive(Debug, Default, Deserialize)]
pub struct GetChannelKeysQuery {
    #[serde(default)]
    pub version: Option<i32>,
}

fn encrypt_with_aes_gcm(key: &[u8; 32], plaintext: &[u8]) -> Result<(Vec<u8>, [u8; AES_GCM_NONCE_SIZE]), AppError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| AppError::EncapsulationFailed)?;
    let mut nonce_bytes = [0u8; AES_GCM_NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let encrypted = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| AppError::EncapsulationFailed)?;
    Ok((encrypted, nonce_bytes))
}

fn decrypt_with_aes_gcm(key: &[u8; 32], ciphertext: &[u8], nonce: &[u8]) -> Result<Vec<u8>, AppError> {
    if nonce.len() != AES_GCM_NONCE_SIZE {
        return Err(AppError::DecryptionFailed);
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| AppError::DecryptionFailed)?;
    let nonce = Nonce::from_slice(nonce);
    cipher.decrypt(nonce, ciphertext).map_err(|_| AppError::DecryptionFailed)
}

fn wrap_channel_key_for_device(channel_key: &[u8], device_public_key_b64: &str) -> Result<(String, String), AppError> {
    let public_key_bytes = STANDARD
        .decode(device_public_key_b64)
        .map_err(|_| AppError::PublicKeyNotFound)?;

    let key: ml_kem::kem::Key<ml_kem_1024::EncapsulationKey> = public_key_bytes
        .as_slice()
        .try_into()
        .map_err(|_| AppError::PublicKeyNotFound)?;

    let encapsulation_key = ml_kem_1024::EncapsulationKey::new(&key)
        .map_err(|_| AppError::PublicKeyNotFound)?;

    let (kem_ciphertext, shared_secret) = encapsulation_key.encapsulate();
    let mut wrapping_key = [0u8; 32];
    wrapping_key.copy_from_slice(shared_secret.as_slice());

    let (encrypted_key, nonce) = encrypt_with_aes_gcm(&wrapping_key, channel_key)?;

    let mut wrapped = Vec::with_capacity(AES_GCM_NONCE_SIZE + encrypted_key.len());
    wrapped.extend_from_slice(&nonce);
    wrapped.extend_from_slice(&encrypted_key);

    Ok((STANDARD.encode(wrapped), STANDARD.encode(kem_ciphertext.as_slice())))
}

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
    #[serde(default)]
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
    let _channel = channel.ok_or(AppError::ChannelNotFound)?;

    let can_access = state.db
        .user_can_access_channel(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;
    if !can_access {
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

    if req.is_private {
        state
            .db
            .add_channel_member(channel_id, claims.user_id)
            .await
            .map_err(AppError::DatabaseError)?;
    }

    let mut initial_key_version_id = None;
    let mut initial_key_version = None;

    if req.encryption_type == EncryptionType::Symmetric {
        let mut channel_key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut channel_key);

        let (encrypted_key_bytes, nonce) = encrypt_with_aes_gcm(&state.config.server_master_key, &channel_key)?;
        let encrypted_key_b64 = STANDARD.encode(encrypted_key_bytes);
        let nonce_b64 = STANDARD.encode(nonce);

        let key_version_id = state
            .db
            .create_channel_key_version(channel_id, 1, &encrypted_key_b64, &nonce_b64, claims.user_id)
            .await
            .map_err(AppError::DatabaseError)?;
        initial_key_version_id = Some(key_version_id);
        initial_key_version = Some(1);
    } else if req.encryption_type == EncryptionType::Asymmetric {
        let key_version_id = state
            .db
            .create_channel_key_version(channel_id, 1, "", "", claims.user_id)
            .await
            .map_err(AppError::DatabaseError)?;
        initial_key_version_id = Some(key_version_id);
        initial_key_version = Some(1);
    }

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
            key_version_id: initial_key_version_id,
            key_version: initial_key_version,
            created_at: now,
        }),
    ))
}

pub async fn get_channel_keys(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<GetChannelKeysQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    info!("Endpoint get_channel_keys cridat: channel_id={}, device_id={}", channel_id, claims.device_id);

    // Verificar que el canal existeix i l'usuari és membre
    let channel = state.db.get_channel(channel_id).await.map_err(AppError::DatabaseError)?;
    let channel = channel.ok_or(AppError::ChannelNotFound)?;
    let can_access = state.db
        .user_can_access_channel(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;
    if !can_access {
        return Err(AppError::Forbidden);
    }

    if channel.encryption_type == EncryptionType::Symmetric {
        let (device_public_key, _) = state
            .db
            .get_device_public_keys_for_user(claims.device_id, claims.user_id)
            .await
            .map_err(AppError::DatabaseError)?
            .filter(|(kem_public_key, _)| !kem_public_key.trim().is_empty())
            .ok_or(AppError::DeviceNoPublicKey)?;

        let key_version_row = if let Some(version) = query.version {
            state
                .db
                .get_channel_key_version(channel_id, version)
                .await
                .map_err(AppError::DatabaseError)?
        } else {
            state
                .db
                .get_latest_channel_key_version(channel_id)
                .await
                .map_err(AppError::DatabaseError)?
        }
        .ok_or(AppError::ChannelKeyNotFound)?;

        let (_, version, encrypted_key_b64, nonce_b64) = key_version_row;
        let encrypted_key = STANDARD.decode(encrypted_key_b64).map_err(|_| AppError::DecryptionFailed)?;
        let nonce = STANDARD.decode(nonce_b64).map_err(|_| AppError::DecryptionFailed)?;
        let channel_key = decrypt_with_aes_gcm(&state.config.server_master_key, &encrypted_key, &nonce)?;

        let (wrapped_key, kem_ciphertext) = wrap_channel_key_for_device(&channel_key, &device_public_key)?;

        Ok(Json(serde_json::json!({
            "success": true,
            "data": {
                "deviceId": claims.device_id,
                "encryptedKey": wrapped_key,
                "kemCiphertext": kem_ciphertext,
                "keyVersion": version,
            }
        })))
    } else {
        // Nivell 2: recuperar bundle pujat pels clients (zero-knowledge del servidor)
        let key_entry = state.db
            .get_latest_channel_key_bundle_for_device(channel_id, claims.device_id)
            .await
            .map_err(AppError::DatabaseError)?;

        match key_entry {
            Some((key_version_id, key_version, encrypted_key, kem_ciphertext, signature, signed_by_device_id)) => Ok(Json(serde_json::json!({
                "success": true,
                "data": {
                    "deviceId": claims.device_id,
                    "keyVersionId": key_version_id,
                    "keyVersion": key_version,
                    "encryptedKey": encrypted_key,
                    "kemCiphertext": kem_ciphertext,
                    "signature": signature,
                    "signedByDeviceId": signed_by_device_id,
                }
            }))),
            None => Err(AppError::ChannelKeyNotFound),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ChannelKeyBundle {
    pub device_id: Uuid,
    pub encrypted_key: String,
    pub kem_ciphertext: String,
    pub key_version: Option<i32>,
    pub signature: Option<String>,
    pub signed_by_device_id: Option<Uuid>,
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
    let can_access = state.db
        .user_can_access_channel(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;
    if !can_access {
        return Err(AppError::Forbidden);
    }

    let member_devices = state
        .db
        .get_member_devices_for_channel(channel_id)
        .await
        .map_err(AppError::DatabaseError)?;
    let member_device_ids: std::collections::HashSet<Uuid> = member_devices
        .iter()
        .map(|(device_id, _, _)| *device_id)
        .collect();

    for bundle in &bundles {
        if !member_device_ids.contains(&bundle.device_id) {
            return Err(AppError::Forbidden);
        }

        if channel.encryption_type == EncryptionType::Asymmetric {
            if bundle.signed_by_device_id != Some(claims.device_id) {
                return Err(AppError::Forbidden);
            }
        }

        let requested_version = bundle.key_version.unwrap_or(1);
        let key_version_id = if let Some((key_version_id, _, _, _)) = state
            .db
            .get_channel_key_version(channel_id, requested_version)
            .await
            .map_err(AppError::DatabaseError)?
        {
            key_version_id
        } else if channel.encryption_type == EncryptionType::Asymmetric {
            state
                .db
                .create_channel_key_version(channel_id, requested_version, "", "", claims.user_id)
                .await
                .map_err(AppError::DatabaseError)?
        } else {
            return Err(AppError::ChannelKeyNotFound);
        };

        state.db
            .store_channel_key_bundle_for_device(
                key_version_id,
                bundle.device_id,
                &bundle.encrypted_key,
                &bundle.kem_ciphertext,
                bundle.signature.as_deref(),
                bundle.signed_by_device_id,
            )
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
    let _channel = channel.ok_or(AppError::ChannelNotFound)?;
    let can_access = state.db
        .user_can_access_channel(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;
    if !can_access {
        return Err(AppError::Forbidden);
    }

    let devices = state.db
        .get_member_devices_for_channel(channel_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let data: Vec<serde_json::Value> = devices.iter().map(|(device_id, kem_public_key, dsa_public_key)| serde_json::json!({
        "deviceId": device_id,
        "kemPublicKey": kem_public_key,
        "dsaPublicKey": dsa_public_key,
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
    let channel = _state.db.get_channel(channel_id).await.map_err(AppError::DatabaseError)?;
    let channel = channel.ok_or(AppError::ChannelNotFound)?;

    let current_role = _state
        .db
        .is_server_member(channel.server_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::NotServerMember)?;

    if current_role != "owner" && current_role != "admin" && current_role != "member" {
        return Err(AppError::Forbidden);
    }

    let invited_user = _state
        .db
        .find_user_by_username(&req.username)
        .await
        .map_err(|_| AppError::InternalError)?
        .ok_or(AppError::UserNotFound)?;

    if _state
        .db
        .is_server_member(channel.server_id, invited_user.0)
        .await
        .map_err(AppError::DatabaseError)?
        .is_none()
    {
        _state
            .db
            .add_server_member(channel.server_id, invited_user.0, "member")
            .await
            .map_err(AppError::DatabaseError)?;
    }

    if channel.is_private {
        _state
            .db
            .add_channel_member(channel_id, invited_user.0)
            .await
            .map_err(AppError::DatabaseError)?;
    }

    let invited_devices = _state
        .db
        .list_devices_for_user(invited_user.0)
        .await
        .map_err(AppError::DatabaseError)?;

    let devices_invited = invited_devices
        .iter()
        .filter(|(_, _, kem_public_key, _, _, _, revoked)| !revoked && !kem_public_key.trim().is_empty())
        .count() as u32;

    Ok((
        StatusCode::CREATED,
        Json(InviteResponse {
            invited_user: req.username,
            devices_invited,
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