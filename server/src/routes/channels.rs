//! Endpoints de canals.

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
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;
use crate::{
    db::{
        ChannelKeyBundleWriteResult,
        CHANNEL_PERMISSION_MANAGE,
        CHANNEL_PERMISSION_READ,
        CHANNEL_PERMISSION_WRITE,
        SERVER_PERMISSION_MANAGE_MEMBERS,
    },
    middleware::{AppState, AuthClaims},
    error::AppError,
    models::{Channel, ChannelType, EncryptionType},
};
use tracing::info;

const AES_GCM_NONCE_SIZE: usize = 12;

fn is_duplicate_channel_name_error(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db_err) => {
            let code = db_err.code().map(|c| c.to_string());
            let constraint = db_err.constraint();
            let message = db_err.message();

            // PostgreSQL unique_violation = 23505. SQLite constraint unique = 2067.
            // L'índex únic és per (server_id, tipus, name): noms únics dins de cada tipus.
            code.as_deref() == Some("23505")
                || code.as_deref() == Some("2067")
                || constraint == Some("idx_channels_server_type_name")
                || message.contains("idx_channels_server_type_name")
                || message.contains("UNIQUE constraint failed: channels.server_id, channels.type, channels.name")
        }
        _ => false,
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct GetChannelKeysQuery {
    #[serde(default)]
    pub version: Option<i32>,
}

async fn ensure_channel_permission(
    state: &AppState,
    channel_id: Uuid,
    user_id: Uuid,
    is_admin: bool,
    min_level: i32,
) -> Result<i32, AppError> {
    if is_admin {
        return Ok(CHANNEL_PERMISSION_MANAGE);
    }

    let level = state
        .db
        .get_channel_permission_level(channel_id, user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .unwrap_or(0);

    if level < min_level {
        return Err(AppError::Forbidden);
    }

    Ok(level)
}

fn encrypt_with_aes_gcm(key: &[u8; 32], plaintext: &[u8]) -> Result<(Vec<u8>, [u8; AES_GCM_NONCE_SIZE]), AppError> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| AppError::EncapsulationFailed)?;
    let mut nonce_bytes = [0u8; AES_GCM_NONCE_SIZE];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
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

// Distingeix entre camp absent (None) i camp present amb null (Some(None))
fn deserialize_double_option<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    Ok(Some(Option::deserialize(d)?))
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    #[serde(default, deserialize_with = "deserialize_double_option")]
    pub message_ttl: Option<Option<i32>>,
    #[serde(default)]
    pub channel_type: Option<ChannelType>,
    #[serde(default)]
    pub encryption_type: Option<EncryptionType>,
    #[serde(default)]
    pub is_private: Option<bool>,
    #[serde(default)]
    pub position: Option<i32>,
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
    if !claims.is_admin {
        let role = state.db.is_server_member(server_id, claims.user_id).await
            .map_err(|e| AppError::DatabaseError(e))?;
        if role.is_none() {
            return Err(AppError::Forbidden);
        }
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

    ensure_channel_permission(&state, channel_id, claims.user_id, claims.is_admin, CHANNEL_PERMISSION_READ).await?;

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

    let trimmed_name = req.name.trim();
    if trimmed_name.is_empty() || trimmed_name.len() > 100 {
        return Err(AppError::BadRequest);
    }

    // Verificar que l'usuari és membre del servidor
    if !claims.is_admin {
        let role = state.db.is_server_member(server_id, claims.user_id).await
            .map_err(|e| AppError::DatabaseError(e))?;
        let role = role.ok_or(AppError::Forbidden)?;
        if role != "owner" && role != "admin" {
            return Err(AppError::Forbidden);
        }
    }

    let (max_text_channels, max_voice_channels) = state
        .db
        .get_user_channel_limits(claims.user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    match req.channel_type {
        ChannelType::Text => {
            let current = state
                .db
                .count_channels_by_type_in_server(server_id, "text")
                .await
                .map_err(|_| AppError::DatabaseUnavailable)?;
            if max_text_channels != -1 && current >= i64::from(max_text_channels) {
                return Err(AppError::ChannelLimitExceeded);
            }
        }
        ChannelType::Voice => {
            let current = state
                .db
                .count_channels_by_type_in_server(server_id, "voice")
                .await
                .map_err(|_| AppError::DatabaseUnavailable)?;
            if max_voice_channels != -1 && current >= i64::from(max_voice_channels) {
                return Err(AppError::ChannelLimitExceeded);
            }
        }
    }

    // El nom és únic per tipus: text i veu poden compartir nom, però no dos de text ni dos de veu.
    let channel_type_str = match req.channel_type {
        ChannelType::Text => "text",
        ChannelType::Voice => "voice",
    };

    let channel_name_exists = state
        .db
        .channel_name_exists_in_server(server_id, &req.name, channel_type_str)
        .await
        .map_err(AppError::DatabaseError)?;
    if channel_name_exists {
        return Err(AppError::ChannelNameExists);
    }

    let channel_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    // Guardar a la base de dades
    let encryption_str = match req.encryption_type {
        EncryptionType::None => "none",
        EncryptionType::Symmetric => "symmetric",
        EncryptionType::Asymmetric => "asymmetric",
    };

    state
        .db
        .create_channel(
            channel_id,
            server_id,
            &req.name,
            channel_type_str,
            encryption_str,
            req.message_ttl,
            req.is_private,
        )
        .await
        .map_err(|e| {
            if is_duplicate_channel_name_error(&e) {
                AppError::ChannelNameExists
            } else {
                AppError::DatabaseError(e)
            }
        })?;

    if req.is_private {
        state
            .db
            .add_channel_member_with_permission(channel_id, claims.user_id, CHANNEL_PERMISSION_MANAGE)
            .await
            .map_err(AppError::DatabaseError)?;
    }

    let mut initial_key_version_id = None;
    let mut initial_key_version = None;

    if req.encryption_type == EncryptionType::Symmetric {
        let mut channel_key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut channel_key);

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

    let channels_updated_event = serde_json::json!({
        "serverId": server_id,
        "reason": "channel-created",
        "channelId": channel_id,
    });
    let server_room = format!("server:{}", server_id);
    if let Err(e) = state
        .io
        .to(server_room)
        .emit("server-channels-updated", &channels_updated_event)
        .await
    {
        tracing::warn!("Error enviant server-channels-updated: {:?}", e);
    }

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
            permission_level: Some(CHANNEL_PERMISSION_MANAGE),
            unread_count: 0,
            key_version_id: initial_key_version_id,
            key_version: initial_key_version,
            last_read_message_id: None,
            created_at: now,
            position: 0,
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
    ensure_channel_permission(&state, channel_id, claims.user_id, claims.is_admin, CHANNEL_PERMISSION_READ).await?;

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

        let (key_version_id, version, encrypted_key_b64, nonce_b64) = key_version_row;
        let encrypted_key = STANDARD.decode(encrypted_key_b64).map_err(|_| AppError::DecryptionFailed)?;
        let nonce = STANDARD.decode(nonce_b64).map_err(|_| AppError::DecryptionFailed)?;
        let channel_key = decrypt_with_aes_gcm(&state.config.server_master_key, &encrypted_key, &nonce)?;

        let (wrapped_key, kem_ciphertext) = wrap_channel_key_for_device(&channel_key, &device_public_key)?;

        Ok(Json(serde_json::json!({
            "success": true,
            "data": {
                "deviceId": claims.device_id,
                "keyVersionId": key_version_id,
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

#[derive(Debug, Serialize)]
pub struct RotateChannelKeyResponse {
    pub channel_id: Uuid,
    pub key_version_id: Uuid,
    pub key_version: i32,
}

#[derive(Debug, Serialize)]
pub struct ChannelPermissionEntry {
    pub user_id: Uuid,
    pub username: String,
    pub permission_level: i32,
    pub permission: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateChannelExplicitPermissionRequest {
    #[serde(default)]
    pub permission_level: Option<i32>,
}

pub async fn get_all_channel_key_bundles(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    ensure_channel_permission(&state, channel_id, claims.user_id, claims.is_admin, CHANNEL_PERMISSION_READ).await?;

    let bundles = state
        .db
        .get_all_channel_key_bundles(channel_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let result: Vec<serde_json::Value> = bundles
        .into_iter()
        .map(|(device_id, key_version_id, key_version, encrypted_key, kem_ciphertext, signature, signed_by_device_id)| {
            serde_json::json!({
                "deviceId": device_id,
                "keyVersionId": key_version_id,
                "keyVersion": key_version,
                "encryptedKey": encrypted_key,
                "kemCiphertext": kem_ciphertext,
                "signature": signature,
                "signedByDeviceId": signed_by_device_id,
            })
        })
        .collect();

    Ok(Json(serde_json::json!(result)))
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
    let permission_level = ensure_channel_permission(
        &state,
        channel_id,
        claims.user_id,
        claims.is_admin,
        CHANNEL_PERMISSION_READ,
    )
    .await?;

    if channel.encryption_type == EncryptionType::Symmetric && permission_level < CHANNEL_PERMISSION_MANAGE {
        return Err(AppError::Forbidden);
    }

    if channel.encryption_type == EncryptionType::Asymmetric && permission_level < CHANNEL_PERMISSION_WRITE {
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

        let write_result = state.db
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

        if write_result == ChannelKeyBundleWriteResult::Conflict {
            return Err(AppError::ChannelKeyBundleConflict);
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn rotate_channel_key(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<RotateChannelKeyResponse>, AppError> {
    let channel = state
        .db
        .get_channel(channel_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ChannelNotFound)?;

    let required_permission = if channel.encryption_type == EncryptionType::Asymmetric {
        CHANNEL_PERMISSION_WRITE
    } else {
        CHANNEL_PERMISSION_MANAGE
    };
    ensure_channel_permission(&state, channel_id, claims.user_id, claims.is_admin, required_permission).await?;

    let next_version = state
        .db
        .get_latest_channel_key_version(channel_id)
        .await
        .map_err(AppError::DatabaseError)?
        .map(|(_, version, _, _)| version + 1)
        .unwrap_or(1);

    let key_version_id = if channel.encryption_type == EncryptionType::Symmetric {
        let mut channel_key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut channel_key);

        let (encrypted_key_bytes, nonce) = encrypt_with_aes_gcm(&state.config.server_master_key, &channel_key)?;
        let encrypted_key_b64 = STANDARD.encode(encrypted_key_bytes);
        let nonce_b64 = STANDARD.encode(nonce);

        state
            .db
            .create_channel_key_version(channel_id, next_version, &encrypted_key_b64, &nonce_b64, claims.user_id)
            .await
            .map_err(AppError::DatabaseError)?
    } else {
        state
            .db
            .create_channel_key_version(channel_id, next_version, "", "", claims.user_id)
            .await
            .map_err(AppError::DatabaseError)?
    };

    Ok(Json(RotateChannelKeyResponse {
        channel_id,
        key_version_id,
        key_version: next_version,
    }))
}

pub async fn get_channel_member_devices(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    info!("Endpoint get_channel_member_devices cridat: channel_id={}", channel_id);

    let channel = state.db.get_channel(channel_id).await.map_err(AppError::DatabaseError)?;
    let _channel = channel.ok_or(AppError::ChannelNotFound)?;
    ensure_channel_permission(&state, channel_id, claims.user_id, claims.is_admin, CHANNEL_PERMISSION_WRITE).await?;

    let devices = state.db
        .get_member_devices_for_channel(channel_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let data: Vec<serde_json::Value> = devices.iter().map(|(device_id, kem_public_key, dsa_public_key)| serde_json::json!({
        "deviceId": device_id,
        "kemPublicKey": kem_public_key,
        "dsaPublicKey": dsa_public_key,
        "hasKemPublicKey": !kem_public_key.trim().is_empty(),
        "hasDsaPublicKey": !dsa_public_key.trim().is_empty(),
    })).collect();

    Ok(Json(serde_json::json!({ "success": true, "data": data })))
}

pub async fn get_channel_permissions(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let channel = state
        .db
        .get_channel(channel_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ChannelNotFound)?;

    if channel.server_id == Uuid::nil() {
        return Err(AppError::Forbidden);
    }

    let server_permission = if claims.is_admin {
        SERVER_PERMISSION_MANAGE_MEMBERS
    } else {
        state
            .db
            .get_server_permission_level(channel.server_id, claims.user_id)
            .await
            .map_err(AppError::DatabaseError)?
            .unwrap_or(0)
    };

    if server_permission < SERVER_PERMISSION_MANAGE_MEMBERS {
        ensure_channel_permission(&state, channel_id, claims.user_id, claims.is_admin, CHANNEL_PERMISSION_MANAGE).await?;
    }

    let rows = state
        .db
        .list_channel_permission_levels(channel_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let entries: Vec<ChannelPermissionEntry> = rows
        .into_iter()
        .map(|(user_id, username, permission_level)| ChannelPermissionEntry {
            user_id,
            username,
            permission_level,
            permission: match permission_level {
                3 => "manage",
                2 => "write",
                1 => "read",
                _ => "none",
            }
            .to_string(),
        })
        .collect();

    Ok(Json(serde_json::json!({ "success": true, "data": entries })))
}

pub async fn update_channel_explicit_permission(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path((channel_id, user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateChannelExplicitPermissionRequest>,
) -> Result<StatusCode, AppError> {
    let channel = state
        .db
        .get_channel(channel_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ChannelNotFound)?;

    if channel.server_id == Uuid::nil() {
        return Err(AppError::Forbidden);
    }

    ensure_channel_permission(&state, channel_id, claims.user_id, claims.is_admin, CHANNEL_PERMISSION_MANAGE).await?;

    let is_server_member = state
        .db
        .is_server_member(channel.server_id, user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .is_some();

    if !is_server_member {
        return Err(AppError::UserNotFound);
    }

    if let Some(level) = req.permission_level {
        state
            .db
            .set_explicit_channel_permission(channel_id, user_id, level)
            .await
            .map_err(AppError::DatabaseError)?;
    } else {
        state
            .db
            .remove_explicit_channel_permission(channel_id, user_id)
            .await
            .map_err(AppError::DatabaseError)?;
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_channel_explicit_permissions(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .db
        .get_channel(channel_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ChannelNotFound)?;

    ensure_channel_permission(&state, channel_id, claims.user_id, claims.is_admin, CHANNEL_PERMISSION_MANAGE).await?;

    let rows = state
        .db
        .list_explicit_channel_permissions(channel_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let entries: Vec<ChannelPermissionEntry> = rows
        .into_iter()
        .map(|(user_id, username, permission_level)| ChannelPermissionEntry {
            user_id,
            username,
            permission_level,
            permission: match permission_level {
                3 => "manage",
                2 => "write",
                1 => "read",
                _ => "none",
            }
            .to_string(),
        })
        .collect();

    Ok(Json(serde_json::json!({ "success": true, "data": entries })))
}

pub async fn invite_to_channel(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<InviteRequest>,
) -> Result<(StatusCode, Json<InviteResponse>), AppError> {
    info!("Endpoint invite_to_channel cridat: channel_id={}, username={}, user_id={}", channel_id, req.username, claims.user_id);
    let channel = state.db.get_channel(channel_id).await.map_err(AppError::DatabaseError)?;
    let channel = channel.ok_or(AppError::ChannelNotFound)?;

    let required_permission = if channel.encryption_type == EncryptionType::Asymmetric {
        CHANNEL_PERMISSION_WRITE
    } else {
        CHANNEL_PERMISSION_MANAGE
    };

    if ensure_channel_permission(&state, channel_id, claims.user_id, claims.is_admin, required_permission)
        .await
        .is_err()
    {
        return Err(AppError::Forbidden);
    }

    let invited_user = state
        .db
        .find_user_by_username(&req.username)
        .await
        .map_err(|_| AppError::InternalError)?
        .ok_or(AppError::UserNotFound)?;

    if state
        .db
        .is_server_member(channel.server_id, invited_user.0)
        .await
        .map_err(AppError::DatabaseError)?
        .is_none()
    {
        state
            .db
            .add_server_member(channel.server_id, invited_user.0, "member")
            .await
            .map_err(AppError::DatabaseError)?;
    }

    if channel.is_private {
        state
            .db
            .add_channel_member_with_permission(channel_id, invited_user.0, CHANNEL_PERMISSION_WRITE)
            .await
            .map_err(AppError::DatabaseError)?;
    }

    let invited_devices = state
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

    ensure_channel_permission(&state, channel_id, claims.user_id, claims.is_admin, CHANNEL_PERMISSION_MANAGE).await?;

    // Build partial update
    let name = req.name.as_deref();
    // None = camp absent (conserva valor actual); Some(v) = valor explícit (inclòs null)
    let message_ttl = match req.message_ttl {
        None => channel.message_ttl,
        Some(v) => v,
    };
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

    // Nom únic per tipus: si canvia el parell (nom, tipus), assegurar que no col·lisiona
    // amb un altre canal del mateix tipus al servidor.
    let current_type_str = match channel.channel_type {
        ChannelType::Text => "text",
        ChannelType::Voice => "voice",
    };
    if let Some(new_name) = name {
        if new_name != channel.name || channel_type_str != current_type_str {
            let exists = state
                .db
                .channel_name_exists_in_server(channel.server_id, new_name, channel_type_str)
                .await
                .map_err(AppError::DatabaseError)?;
            if exists {
                return Err(AppError::ChannelNameExists);
            }
        }
    }

    state.db.update_channel(
        channel_id,
        channel.server_id,
        name,
        channel_type_str,
        encryption_str,
        message_ttl,
        is_private,
    ).await.map_err(|e| {
        if is_duplicate_channel_name_error(&e) {
            AppError::ChannelNameExists
        } else {
            AppError::DatabaseError(e)
        }
    })?;

    if let Some(position) = req.position {
        state.db.update_channel_position(channel_id, position).await
            .map_err(|e| AppError::DatabaseError(e))?;
    }

    // Read back the updated channel
    let updated = state.db.get_channel(channel_id).await
        .map_err(|e| AppError::DatabaseError(e))?
        .ok_or(AppError::ChannelNotFound)?;

    let server_room = format!("server:{}", channel.server_id);
    let event = serde_json::json!({
        "serverId": channel.server_id,
        "reason": "channel-updated",
        "channelId": channel_id,
    });
    if let Err(e) = state.io.to(server_room).emit("server-channels-updated", &event).await {
        tracing::warn!("Error enviant server-channels-updated: {:?}", e);
    }

    Ok(Json(updated))
}

pub async fn delete_channel(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    info!("Endpoint delete_channel cridat: channel_id={}, user_id={}", channel_id, claims.user_id);

    // Validate channel exists before permission checks.
    let _channel = state.db.get_channel(channel_id).await.map_err(|e| AppError::DatabaseError(e))?;
    let _channel = _channel.ok_or(AppError::ChannelNotFound)?;

    ensure_channel_permission(&state, channel_id, claims.user_id, claims.is_admin, CHANNEL_PERMISSION_MANAGE).await?;

    // Delete the channel from DB
    state.db.delete_channel(channel_id).await.map_err(|e| AppError::DatabaseError(e))?;

    info!("Canal eliminat de la DB: channel_id={}", channel_id);
    Ok(StatusCode::OK)
}

#[derive(Debug, Deserialize)]
pub struct ReorderChannelRequest {
    pub channels: Vec<ReorderChannelItem>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderChannelItem {
    pub channel_id: Uuid,
    pub position: i32,
}

pub async fn reorder_channels(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
    Json(req): Json<ReorderChannelRequest>,
) -> Result<StatusCode, AppError> {
    info!("Endpoint reorder_channels cridat: server_id={}, user_id={}", server_id, claims.user_id);

    if !claims.is_admin {
        let role = state.db.is_server_member(server_id, claims.user_id).await
            .map_err(|e| AppError::DatabaseError(e))?;
        let role = role.ok_or(AppError::Forbidden)?;
        if role != "owner" && role != "admin" {
            return Err(AppError::Forbidden);
        }
    }

    for item in &req.channels {
        state.db.update_channel_position(item.channel_id, item.position).await
            .map_err(AppError::DatabaseError)?;
    }

    Ok(StatusCode::OK)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/servers/{server_id}/channels", get(list_channels).post(create_channel))
        .route("/api/servers/{server_id}/channels/reorder", put(reorder_channels))
        .route("/api/channels/{channel_id}/read", post(mark_channel_read))
        .route("/api/channels/{channel_id}/keys", get(get_channel_keys).post(upload_channel_keys))
        .route("/api/channels/{channel_id}/keys/rotate", post(rotate_channel_key))
        .route("/api/channels/{channel_id}/member-devices", get(get_channel_member_devices))
        .route("/api/channels/{channel_id}/permissions", get(get_channel_permissions))
        .route("/api/channels/{channel_id}/permissions/explicit", get(get_channel_explicit_permissions))
        .route("/api/channels/{channel_id}/permissions/explicit/{user_id}", put(update_channel_explicit_permission))
            .route("/api/channels/{channel_id}/keys/all", get(get_all_channel_key_bundles))
        .route("/api/channels/{channel_id}/invite", post(invite_to_channel))
        .route("/api/channels/{channel_id}", put(update_channel).delete(delete_channel))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, LogLevel},
        db::connect_db,
        middleware::auth::UserPresenceState,
    };
    use axum::response::IntoResponse;
    use socketioxide::extract::{Data, SocketRef};
    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    };
    use tokio::sync::RwLock;

    async fn make_state() -> AppState {
        let config = Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 8080,
            database_url: "sqlite::memory:".to_string(),
            open_register: true,
            admin_user: None,
            admin_password: None,
            ttl_cleanup_interval_minutes: 5,
            livekit_host: "http://localhost:7880".to_string(),
            livekit_api_key: "test-key".to_string(),
            livekit_api_secret: "test-secret".to_string(),
            jwt_secret: "test-secret".to_string(),
            jwt_expiration_days: 7,
            backend_debug: LogLevel::Info,
            server_master_key: [7u8; 32],
            static_dir: None,
            max_file_size_bytes: 100 * 1024 * 1024,
            allowed_origins: vec![],
        };

        let db = connect_db(&config).await.expect("sqlite test db should initialize");
        let (_layer, io) = socketioxide::SocketIo::new_layer();
        io.ns("/", |_socket: SocketRef, Data(_auth): Data<serde_json::Value>| async move {});

        AppState {
            db,
            config,
            io,
            user_presence: Arc::new(RwLock::new(UserPresenceState {
                online_sockets: HashMap::<Uuid, HashSet<String>>::new(),
            })),
            livekit_token_cache: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
        }
    }

    fn claims_for(user_id: Uuid, username: &str) -> AuthClaims {
        AuthClaims {
            user_id,
            username: username.to_string(),
            device_id: Uuid::new_v4(),
            is_admin: false,
            exp: 0,
            iat: 0,
            jti: Uuid::new_v4().to_string(),
        }
    }

    #[tokio::test]
    async fn create_text_channel_returns_429_when_free_limit_reached() {
        let state = make_state().await;
        let user_id = state
            .db
            .create_user_with_role("channel_free_user", "hash", "user")
            .await
            .expect("user creation should work");
        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "channels-free-server", None, user_id)
            .await
            .expect("server should be created");

        for i in 0..3 {
            state
                .db
                .create_channel(
                    Uuid::new_v4(),
                    server_id,
                    &format!("text-{}", i),
                    "text",
                    "none",
                    None,
                    false,
                )
                .await
                .expect("seed channel should be created");
        }

        let result = create_channel(
            State(state),
            axum::Extension(claims_for(user_id, "channel_free_user")),
            Path(server_id),
            Json(CreateChannelRequest {
                name: "text-over-limit".to_string(),
                channel_type: ChannelType::Text,
                encryption_type: EncryptionType::None,
                message_ttl: None,
                is_private: false,
            }),
        )
        .await;

        let err = result.expect_err("free plan should block 4th text channel");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn create_text_channel_allows_above_free_limit_on_pro_plan() {
        let state = make_state().await;
        let user_id = state
            .db
            .create_user_with_role("channel_pro_user", "hash", "user")
            .await
            .expect("user creation should work");
        state
            .db
            .set_user_plan_by_name(user_id, "pro")
            .await
            .expect("plan assignment should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "channels-pro-server", None, user_id)
            .await
            .expect("server should be created");

        for i in 0..3 {
            state
                .db
                .create_channel(
                    Uuid::new_v4(),
                    server_id,
                    &format!("text-pro-{}", i),
                    "text",
                    "none",
                    None,
                    false,
                )
                .await
                .expect("seed channel should be created");
        }

        let result = create_channel(
            State(state),
            axum::Extension(claims_for(user_id, "channel_pro_user")),
            Path(server_id),
            Json(CreateChannelRequest {
                name: "text-pro-4".to_string(),
                channel_type: ChannelType::Text,
                encryption_type: EncryptionType::None,
                message_ttl: None,
                is_private: false,
            }),
        )
        .await;

        assert!(result.is_ok(), "pro plan should allow creating 4th text channel");
        let (status, _) = result.expect("request should succeed");
        assert_eq!(status, StatusCode::CREATED);
    }

    #[tokio::test]
    async fn update_channel_allows_public_explicit_manage_override() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("channel_owner", "hash", "user")
            .await
            .expect("owner creation should work");
        let manager_id = state
            .db
            .create_user_with_role("channel_manager", "hash", "user")
            .await
            .expect("manager creation should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "channels-manage-server", None, owner_id)
            .await
            .expect("server should be created");
        state
            .db
            .add_server_member(server_id, manager_id, "member")
            .await
            .expect("manager should join server");

        let channel_id = Uuid::new_v4();
        state
            .db
            .create_channel(channel_id, server_id, "general", "text", "none", None, false)
            .await
            .expect("public channel should be created");
        state
            .db
            .set_explicit_channel_permission(channel_id, manager_id, CHANNEL_PERMISSION_MANAGE)
            .await
            .expect("explicit manage override should be stored");

        let result = update_channel(
            State(state),
            axum::Extension(claims_for(manager_id, "channel_manager")),
            Path(channel_id),
            Json(UpdateChannelRequest {
                name: Some("general-updated".to_string()),
                message_ttl: None,
                channel_type: None,
                encryption_type: None,
                is_private: None,
                position: None,
            }),
        )
        .await;

        assert!(result.is_ok(), "explicit manage override should allow channel updates");
        let Json(updated_channel) = result.expect("request should succeed");
        assert_eq!(updated_channel.name, "general-updated");
    }

    #[tokio::test]
    async fn create_channel_succeeds_for_server_owner() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("chan_owner_basic", "hash", "user")
            .await
            .expect("owner creation should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "test-server-basic", None, owner_id)
            .await
            .expect("server should be created");

        let result = create_channel(
            State(state),
            axum::Extension(claims_for(owner_id, "chan_owner_basic")),
            Path(server_id),
            Json(CreateChannelRequest {
                name: "general".to_string(),
                channel_type: ChannelType::Text,
                encryption_type: EncryptionType::None,
                message_ttl: None,
                is_private: false,
            }),
        )
        .await;

        assert!(result.is_ok(), "server owner should be able to create a channel");
        let (status, Json(channel)) = result.expect("should return channel");
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(channel.name, "general");
        assert_eq!(channel.server_id, server_id);
    }

    #[tokio::test]
    async fn create_channel_returns_conflict_when_name_exists_in_same_server() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("chan_owner_dup", "hash", "user")
            .await
            .expect("owner creation should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "test-server-dup", None, owner_id)
            .await
            .expect("server should be created");

        state
            .db
            .create_channel(Uuid::new_v4(), server_id, "general", "text", "none", None, false)
            .await
            .expect("seed channel should be created");

        let result = create_channel(
            State(state),
            axum::Extension(claims_for(owner_id, "chan_owner_dup")),
            Path(server_id),
            Json(CreateChannelRequest {
                name: "general".to_string(),
                channel_type: ChannelType::Text,
                encryption_type: EncryptionType::None,
                message_ttl: None,
                is_private: false,
            }),
        )
        .await;

        let err = result.expect_err("duplicate channel name should fail");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn create_channel_allows_same_name_for_different_type() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("chan_owner_xtype", "hash", "user")
            .await
            .expect("owner creation should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "test-server-xtype", None, owner_id)
            .await
            .expect("server should be created");

        // Canal de text "general".
        state
            .db
            .create_channel(Uuid::new_v4(), server_id, "general", "text", "none", None, false)
            .await
            .expect("text channel should be created");

        // Canal de veu amb el mateix nom: permès.
        let result = create_channel(
            State(state),
            axum::Extension(claims_for(owner_id, "chan_owner_xtype")),
            Path(server_id),
            Json(CreateChannelRequest {
                name: "general".to_string(),
                channel_type: ChannelType::Voice,
                encryption_type: EncryptionType::None,
                message_ttl: None,
                is_private: false,
            }),
        )
        .await;

        assert!(result.is_ok(), "voice channel may share name with a text channel");
        let (status, Json(channel)) = result.expect("should return channel");
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(channel.name, "general");
        assert_eq!(channel.channel_type, ChannelType::Voice);
    }

    #[tokio::test]
    async fn create_channel_returns_conflict_for_duplicate_voice_name() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("chan_owner_dupvoice", "hash", "user")
            .await
            .expect("owner creation should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "test-server-dupvoice", None, owner_id)
            .await
            .expect("server should be created");

        state
            .db
            .create_channel(Uuid::new_v4(), server_id, "veu", "voice", "none", None, false)
            .await
            .expect("seed voice channel should be created");

        let result = create_channel(
            State(state),
            axum::Extension(claims_for(owner_id, "chan_owner_dupvoice")),
            Path(server_id),
            Json(CreateChannelRequest {
                name: "veu".to_string(),
                channel_type: ChannelType::Voice,
                encryption_type: EncryptionType::None,
                message_ttl: None,
                is_private: false,
            }),
        )
        .await;

        let err = result.expect_err("duplicate voice channel name should fail");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn create_channel_name_is_case_and_accent_sensitive() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("chan_owner_case", "hash", "user")
            .await
            .expect("owner creation should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "test-server-case", None, owner_id)
            .await
            .expect("server should be created");

        state
            .db
            .create_channel(Uuid::new_v4(), server_id, "general", "text", "none", None, false)
            .await
            .expect("seed channel should be created");

        // "General" (majúscula) i "generàl" (accent) són noms diferents: permesos.
        for name in ["General", "generàl"] {
            let result = create_channel(
                State(state.clone()),
                axum::Extension(claims_for(owner_id, "chan_owner_case")),
                Path(server_id),
                Json(CreateChannelRequest {
                    name: name.to_string(),
                    channel_type: ChannelType::Text,
                    encryption_type: EncryptionType::None,
                    message_ttl: None,
                    is_private: false,
                }),
            )
            .await;
            assert!(result.is_ok(), "name '{}' should differ from 'general'", name);
            let (status, _) = result.expect("should return channel");
            assert_eq!(status, StatusCode::CREATED);
        }
    }

    #[tokio::test]
    async fn create_channel_forbidden_for_non_member() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("chan_owner_nm", "hash", "user")
            .await
            .expect("owner creation should work");
        let outsider_id = state
            .db
            .create_user_with_role("chan_outsider_nm", "hash", "user")
            .await
            .expect("outsider creation should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "test-server-nm", None, owner_id)
            .await
            .expect("server should be created");

        let result = create_channel(
            State(state),
            axum::Extension(claims_for(outsider_id, "chan_outsider_nm")),
            Path(server_id),
            Json(CreateChannelRequest {
                name: "unauthorized-channel".to_string(),
                channel_type: ChannelType::Text,
                encryption_type: EncryptionType::None,
                message_ttl: None,
                is_private: false,
            }),
        )
        .await;

        let err = result.expect_err("non-member should be forbidden");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn list_channels_forbidden_for_non_member() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("chan_list_owner", "hash", "user")
            .await
            .expect("owner creation should work");
        let outsider_id = state
            .db
            .create_user_with_role("chan_list_outsider", "hash", "user")
            .await
            .expect("outsider creation should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "test-list-server", None, owner_id)
            .await
            .expect("server should be created");

        let result = list_channels(
            State(state),
            axum::Extension(claims_for(outsider_id, "chan_list_outsider")),
            Path(server_id),
        )
        .await;

        let err = result.expect_err("non-member should be forbidden to list channels");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn list_channels_returns_created_channel() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("chan_list_ok_owner", "hash", "user")
            .await
            .expect("owner creation should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "test-list-ok-server", None, owner_id)
            .await
            .expect("server should be created");

        let channel_id = Uuid::new_v4();
        state
            .db
            .create_channel(channel_id, server_id, "my-channel", "text", "none", None, false)
            .await
            .expect("channel should be created");

        let result = list_channels(
            State(state),
            axum::Extension(claims_for(owner_id, "chan_list_ok_owner")),
            Path(server_id),
        )
        .await;

        assert!(result.is_ok(), "owner should be able to list channels");
        let Json(channels) = result.expect("should return channels");
        assert!(
            channels.iter().any(|c| c.id == channel_id && c.name == "my-channel"),
            "created channel should appear in list"
        );
    }

    #[tokio::test]
    async fn delete_channel_succeeds_for_owner() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("chan_del_owner", "hash", "user")
            .await
            .expect("owner creation should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "test-del-server", None, owner_id)
            .await
            .expect("server should be created");

        let channel_id = Uuid::new_v4();
        state
            .db
            .create_channel(channel_id, server_id, "to-delete", "text", "none", None, false)
            .await
            .expect("channel should be created");

        let result = delete_channel(
            State(state),
            axum::Extension(claims_for(owner_id, "chan_del_owner")),
            Path(channel_id),
        )
        .await;

        assert!(result.is_ok(), "owner should be able to delete a channel");
        assert_eq!(result.expect("should return status"), StatusCode::OK);
    }

    #[tokio::test]
    async fn delete_channel_forbidden_for_regular_member() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("chan_del_owner2", "hash", "user")
            .await
            .expect("owner creation should work");
        let member_id = state
            .db
            .create_user_with_role("chan_del_member", "hash", "user")
            .await
            .expect("member creation should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "test-del-server2", None, owner_id)
            .await
            .expect("server should be created");
        state
            .db
            .add_server_member(server_id, member_id, "member")
            .await
            .expect("member should join");

        let channel_id = Uuid::new_v4();
        state
            .db
            .create_channel(channel_id, server_id, "protected", "text", "none", None, false)
            .await
            .expect("channel should be created");

        let result = delete_channel(
            State(state),
            axum::Extension(claims_for(member_id, "chan_del_member")),
            Path(channel_id),
        )
        .await;

        let err = result.expect_err("regular member should be forbidden to delete");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn update_channel_blocks_public_explicit_read_override() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("channel_owner_read", "hash", "user")
            .await
            .expect("owner creation should work");
        let reader_id = state
            .db
            .create_user_with_role("channel_reader", "hash", "user")
            .await
            .expect("reader creation should work");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "channels-read-server", None, owner_id)
            .await
            .expect("server should be created");
        state
            .db
            .add_server_member(server_id, reader_id, "member")
            .await
            .expect("reader should join server");

        let channel_id = Uuid::new_v4();
        state
            .db
            .create_channel(channel_id, server_id, "general-read", "text", "none", None, false)
            .await
            .expect("public channel should be created");
        state
            .db
            .set_explicit_channel_permission(channel_id, reader_id, CHANNEL_PERMISSION_READ)
            .await
            .expect("explicit read override should be stored");

        let result = update_channel(
            State(state),
            axum::Extension(claims_for(reader_id, "channel_reader")),
            Path(channel_id),
            Json(UpdateChannelRequest {
                name: Some("general-forbidden".to_string()),
                message_ttl: None,
                channel_type: None,
                encryption_type: None,
                is_private: None,
                position: None,
            }),
        )
        .await;

        let err = result.expect_err("read-only override should forbid updates");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}