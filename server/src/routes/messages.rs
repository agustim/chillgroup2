//! Endpoints de missatges.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing::{get, post, put},
    Router, extract::Path, extract::Query,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    db::{CHANNEL_PERMISSION_READ, CHANNEL_PERMISSION_WRITE},
    middleware::{AppState, AuthClaims},
    error::AppError,
    models::{EncryptionType, Message},
};
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub encrypted_payload: String,
    pub iv: String,
    #[serde(default)]
    pub key_version: Option<i32>,
    pub expires_at: Option<String>,
    #[serde(default)]
    pub is_direct: Option<bool>,
    #[serde(default)]
    pub recipient_user_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct EditMessageRequest {
    pub encrypted_payload: String,
    pub iv: String,
}

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Cursor per paginació endavant (missatges més nous)
    #[serde(default)]
    pub after: Option<Uuid>,
    /// Cursor per paginació enrere (missatges més antics)
    #[serde(default)]
    pub before: Option<Uuid>,
    /// Timestamp opcional per filtrar missatges des d'un moment concret
    #[serde(default)]
    pub since: Option<String>,
    /// Si és true, retorna només els missatges nous des del timestamp
    #[serde(default)]
    pub new_only: Option<bool>,
}

fn default_limit() -> usize { 50 }

#[derive(Debug, Serialize)]
pub struct PaginatedResponse {
    pub data: Vec<Message>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub has_more: bool,
    pub next_cursor: Option<Uuid>,
    pub prev_cursor: Option<Uuid>,
    pub total_new: Option<usize>,
}

/// Recuperar un missatge concret pel seu ID.
pub async fn get_message(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(message_id): Path<Uuid>,
) -> Result<Json<Message>, AppError> {
    info!("Endpoint get_message cridat: message_id={}, user_id={}", message_id, claims.user_id);

    let msg = state.db.get_message(message_id).await
        .map_err(|e| {
            tracing::error!("Error querying message {}: {}", message_id, e);
            AppError::InternalError
        })?;

    match msg {
        Some(message) => {
            if message.channel_id != Uuid::nil() {
                let permission_level = state
                    .db
                    .get_channel_permission_level(message.channel_id, claims.user_id)
                    .await
                    .map_err(|_| AppError::InternalError)?
                    .unwrap_or(0);
                if permission_level < CHANNEL_PERMISSION_READ {
                    return Err(AppError::Forbidden);
                }
            }
            Ok(Json(message))
        }
        None => Err(AppError::MessageNotFound),
    }
}

/// Llistar missatges d'un canal amb paginació.
pub async fn list_messages(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<PaginatedResponse>, AppError> {
    info!(
        "Endpoint list_messages cridat: channel_id={}, user_id={}, limit={}, after={:?}, before={:?}, since={:?}",
        channel_id, claims.user_id, query.limit, query.after, query.before, query.since
    );

    let permission_level = state
        .db
        .get_channel_permission_level(channel_id, claims.user_id)
        .await
        .map_err(|_| AppError::InternalError)?
        .unwrap_or(0);
    if permission_level < CHANNEL_PERMISSION_READ {
        return Err(AppError::Forbidden);
    }

    // Parse 'since' parameter if provided
    let since_dt = query.since
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Utc));

    // Query DB with cursor-based pagination
    let messages = state.db.list_messages(channel_id, query.limit, query.after, query.before, since_dt)
        .await
        .map_err(|e| {
            tracing::error!("Error listing messages for channel {}: {}", channel_id, e);
            AppError::InternalError
        })?;

    // Determine pagination cursors
    let has_more = messages.len() == query.limit; // truncated means more
    let next_cursor = if has_more {
        messages.last().map(|m| m.id)
    } else {
        query.after // No more means the next cursor would be after this page's last item
    };

    let prev_cursor = if query.after.is_some() || query.before.is_some() {
        messages.first().map(|m| m.id)
    } else {
        None
    };

    // If new_only is requested, we also compute the total new count
    let total_new = if query.new_only == Some(true) {
        let since = since_dt.unwrap_or_else(|| chrono::Utc::now() - chrono::Duration::hours(24));
        let new_since = since.to_rfc3339();
        match state.db.count_new_messages(channel_id, claims.user_id, &new_since).await {
            Ok((count, _)) => Some(count),
            Err(e) => {
                tracing::error!("Error counting new messages: {}", e);
                None
            }
        }
    } else {
        query.new_only.and_then(|b| if b { Some(0) } else { None })
    };

    Ok(Json(PaginatedResponse {
        data: messages,
        pagination: PaginationMeta {
            has_more,
            next_cursor,
            prev_cursor,
            total_new,
        },
    }))
}

/// Consultar si hi ha missatges nous des d'un timestamp concret.
/// Aquest endpoint és útil per saber si cal descarregar nous missatges
/// quan l'usuari torna a entrar a un canal.
pub async fn check_new_messages(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<CheckNewMessagesQuery>,
) -> Result<Json<NewMessagesResponse>, AppError> {
    info!(
        "Endpoint check_new_messages cridat: channel_id={}, user_id={}, since={}",
        channel_id, claims.user_id, query.last_seen
    );

    let permission_level = state
        .db
        .get_channel_permission_level(channel_id, claims.user_id)
        .await
        .map_err(|_| AppError::InternalError)?
        .unwrap_or(0);
    if permission_level < CHANNEL_PERMISSION_READ {
        return Err(AppError::Forbidden);
    }

    let (count, first_id) = state.db.count_new_messages(channel_id, claims.user_id, &query.last_seen)
        .await
        .map_err(|e| {
            tracing::error!("Error counting new messages: {}", e);
            AppError::InternalError
        })?;

    Ok(Json(NewMessagesResponse {
        channel_id,
        has_new: count > 0,
        new_count: count,
        first_new_message_id: first_id,
        last_seen: query.last_seen,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CheckNewMessagesQuery {
    /// Timestamp de l'ultima visita de l'usuari al canal
    pub last_seen: String,
}

#[derive(Debug, Serialize)]
pub struct NewMessagesResponse {
    pub channel_id: Uuid,
    pub has_new: bool,
    pub new_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_new_message_id: Option<Uuid>,
    pub last_seen: String,
}

/// Enviar un missatge a un canal.
pub async fn send_message(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<Message>), AppError> {
    info!("Endpoint send_message cridat: channel_id={}, user_id={}", channel_id, claims.user_id);

    let channel = state.db.get_channel(channel_id).await
        .map_err(|e| {
            tracing::error!("Error fetching channel for message send: {}", e);
            AppError::InternalError
        })?
        .ok_or(AppError::ChannelNotFound)?;

    let permission_level = state
        .db
        .get_channel_permission_level(channel_id, claims.user_id)
        .await
        .map_err(|_| AppError::InternalError)?
        .unwrap_or(0);
    if permission_level < CHANNEL_PERMISSION_WRITE {
        return Err(AppError::Forbidden);
    }

    let message_id = Uuid::new_v4();
    let timestamp = chrono::Utc::now();
    let request_expires_at = req
        .expires_at
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&chrono::Utc));
    let channel_expires_at = channel
        .message_ttl
        .map(|ttl| timestamp + Duration::seconds(i64::from(ttl)));
    let expires_at = channel_expires_at.or(request_expires_at);
    let key_version = match channel.encryption_type {
        EncryptionType::None => None,
        EncryptionType::Symmetric | EncryptionType::Asymmetric => {
            let resolved = match req.key_version {
                Some(version) => Some(version),
                None => state
                    .db
                    .get_channel_key_version_metadata(channel_id)
                    .await
                    .map_err(AppError::DatabaseError)?
                    .map(|(_, version)| version),
            };

            if channel.encryption_type == EncryptionType::Asymmetric && resolved.is_none() {
                return Err(AppError::BadRequest);
            }

            resolved
        }
    };

    // Persist to DB
    state.db.create_message(
        message_id,
        channel_id,
        claims.user_id,
        &claims.username,
        claims.device_id,
        &req.encrypted_payload,
        &req.iv,
        key_version,
        expires_at,
        timestamp,
    ).await.map_err(|e| {
        tracing::error!("Error saving message to DB: {}", e);
        AppError::InternalError
    })?;

    info!("Missatge enviat amb èxit: message_id={}", message_id);

    let message = Message {
        id: message_id,
        channel_id,
        sender_user_id: claims.user_id,
        sender_username: Some(claims.username.clone()),
        sender_device_id: claims.device_id,
        encrypted_payload: req.encrypted_payload,
        iv: req.iv,
        key_version,
        timestamp,
        expires_at,
        edited_at: None,
        deleted_at: None,
    };

    // Broadcast via Socket.IO a tots els clients del canal
    let room = format!("channel:{}", channel_id);
    let socket_event = serde_json::json!({
        "messageId": message.id,
        "channelId": message.channel_id,
        "senderUserId": message.sender_user_id,
        "senderUsername": message.sender_username,
        "senderDeviceId": message.sender_device_id,
        "encryptedPayload": message.encrypted_payload,
        "iv": message.iv,
        "keyVersion": message.key_version,
        "timestamp": message.timestamp,
        "editedAt": message.edited_at,
        "deletedAt": message.deleted_at,
    });
    if let Err(e) = state.io.to(room).emit("message", &socket_event).await {
        tracing::warn!("Error fent broadcast del missatge via socket: {:?}", e);
    }

    // Actualitzar comptadors unread per membres del servidor (excepte remitent)
    if let Ok(member_ids) = state.db.list_server_member_ids(channel.server_id).await {
        for member_id in member_ids.into_iter().filter(|id| *id != claims.user_id) {
            match state.db.count_unread_messages_for_user(channel_id, member_id).await {
                Ok(unread_count) => {
                    let unread_event = serde_json::json!({
                        "channelId": channel_id,
                        "unreadCount": unread_count,
                    });
                    let user_room = format!("user:{}", member_id);
                    if let Err(e) = state.io.to(user_room).emit("unread-updated", &unread_event).await {
                        tracing::warn!("Error enviant unread-updated: {:?}", e);
                    }
                }
                Err(e) => tracing::warn!("Error calculant unread per usuari {}: {}", member_id, e),
            }
        }
    }

    Ok((StatusCode::CREATED, Json(message)))
}

/// Editar un missatge existent.
pub async fn edit_message(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(message_id): Path<Uuid>,
    Json(req): Json<EditMessageRequest>,
) -> Result<Json<Message>, AppError> {
    info!("Endpoint edit_message cridat: message_id={}, user_id={}", message_id, claims.user_id);

    // Check the message exists and is not already deleted
    let existing = state.db.get_message(message_id).await
        .map_err(|e| {
            tracing::error!("Error fetching message for edit: {}", e);
            AppError::InternalError
        })?;

    let message = match existing {
        Some(msg) => msg,
        None => return Err(AppError::MessageNotFound),
    };

    // Verify the sender is the current user
    if message.sender_user_id != claims.user_id {
        return Err(AppError::NotMessageSender);
    }

    let edited_at = chrono::Utc::now();

    let updated = state.db.update_message(message_id, &req.encrypted_payload, &req.iv, edited_at)
        .await
        .map_err(|e| {
            tracing::error!("Error updating message: {}", e);
            AppError::InternalError
        })?;

    Ok(Json(updated))
}

/// Eliminar un missatge (soft delete).
pub async fn delete_message(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(message_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    info!("Endpoint delete_message cridat: message_id={}, user_id={}", message_id, claims.user_id);

    // Check message exists
    let existing = state.db.get_message(message_id).await
        .map_err(|e| {
            tracing::error!("Error fetching message for delete: {}", e);
            AppError::InternalError
        })?;

    match existing {
        Some(msg) => {
            // Verify the sender is the current user
            if msg.sender_user_id != claims.user_id {
                return Err(AppError::NotMessageSender);
            }
        }
        None => return Err(AppError::MessageNotFound),
    }

    state.db.delete_message(message_id).await
        .map_err(|e| {
            tracing::error!("Error deleting message: {}", e);
            AppError::InternalError
        })?;

    Ok(StatusCode::OK)
}

/// Enviar un missatge directe (DM) a un altre usuari.
pub async fn send_direct_message(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<Message>), AppError> {
    info!(
        "Endpoint send_direct_message cridat: from={}, to={}, is_direct={}",
        claims.user_id, req.recipient_user_id.unwrap_or(Uuid::nil()), req.is_direct.unwrap_or(false)
    );

    // For now, create a DM with channel_id = Uuid::nil() (not associated to a real channel)
    // In a full implementation, DMs would use a dedicated DM channel table

    let message_id = Uuid::new_v4();
    let expires_at = req.expires_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&chrono::Utc));
    let timestamp = chrono::Utc::now();
    let key_version = req.key_version;

    // Persist to DB
    state.db.create_message(
        message_id,
        Uuid::nil(), // DM: no channel
        claims.user_id,
        &claims.username,
        claims.device_id,
        &req.encrypted_payload,
        &req.iv,
        key_version,
        expires_at,
        timestamp,
    ).await.map_err(|e| {
        tracing::error!("Error saving DM to DB: {}", e);
        AppError::InternalError
    })?;

    info!("Missatge directe enviat: message_id={}", message_id);

    let message = Message {
        id: message_id,
        channel_id: Uuid::nil(),
        sender_user_id: claims.user_id,
        sender_username: Some(claims.username.clone()),
        sender_device_id: claims.device_id,
        encrypted_payload: req.encrypted_payload,
        iv: req.iv,
        key_version,
        timestamp,
        expires_at,
        edited_at: None,
        deleted_at: None,
    };

    Ok((StatusCode::CREATED, Json(message)))
}

/// Llistar missatges directes entre dos usuaris.
pub async fn list_direct_messages(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Query(query): Query<ListDirectMessagesQuery>,
) -> Result<Json<PaginatedResponse>, AppError> {
    info!(
        "Endpoint list_direct_messages cridat: user1={}, user2={}, limit={}",
        claims.user_id, query.with_user, query.limit
    );

    // DM messages are stored with channel_id = nil, and we filter by sender/receiver
    // For now, use list_messages with no channel_id filter via a custom approach
    // In a full implementation, DMs would have dedicated channel records

    // Simplified: query all messages for this user and filter by recipient in the app layer
    let messages = state.db.list_messages(Uuid::nil(), query.limit, query.after, query.before, None)
        .await
        .map_err(|e| {
            tracing::error!("Error listing DMs: {}", e);
            AppError::InternalError
        })?;

    // Filter to only DM messages (channel_id is nil) matching the conversation
    let filtered: Vec<Message> = messages.into_iter()
        .filter(|m| {
            m.channel_id == Uuid::nil()
        })
        .take(query.limit)
        .collect();

    let has_more = filtered.len() == query.limit;
    let next_cursor = if has_more { filtered.last().map(|m| m.id) } else { None };

    Ok(Json(PaginatedResponse {
        data: filtered,
        pagination: PaginationMeta {
            has_more,
            next_cursor,
            prev_cursor: None,
            total_new: None,
        },
    }))
}

#[derive(Debug, Deserialize)]
pub struct ListDirectMessagesQuery {
    /// ID de l'altre usuari en la conversa
    pub with_user: Uuid,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub after: Option<Uuid>,
    #[serde(default)]
    pub before: Option<Uuid>,
}

/// Llistar converses directes de l'usuari.
pub async fn list_conversations(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<Vec<DirectMessageConversation>>, AppError> {
    info!("Endpoint list_conversations cridat: user_id={}", claims.user_id);

    // TODO: Full implementation with proper DM channel support
    Ok(Json(vec![]))
}

#[derive(Debug, Serialize)]
pub struct DirectMessageConversation {
    pub other_user_id: Uuid,
    pub other_user_username: String,
    pub other_user_avatar: Option<String>,
    pub last_message_at: DateTime<Utc>,
    pub unread_count: usize,
    pub last_message_preview: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenDmChannelRequest {
    pub target_user_id: Uuid,
    #[serde(default)]
    pub message_ttl: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct OpenDmChannelResponse {
    pub dm_channel_id: Uuid,
    pub peer_user_id: Uuid,
    pub peer_username: String,
    pub encryption_type: &'static str,
    pub message_ttl: Option<i32>,
    pub key_version_id: Option<Uuid>,
    pub key_version: Option<i32>,
    pub created: bool,
}

#[derive(Debug, Serialize)]
pub struct DmChannelListItem {
    pub dm_channel_id: Uuid,
    pub peer_user_id: Uuid,
    pub peer_username: String,
    pub message_ttl: Option<i32>,
    pub unread_count: usize,
    pub last_message_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDmSettingsRequest {
    #[serde(default)]
    pub message_ttl: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct UpdateDmSettingsResponse {
    pub dm_channel_id: Uuid,
    pub message_ttl: Option<i32>,
}

#[derive(Debug, Serialize)]
pub struct RotateDmKeyResponse {
    pub dm_channel_id: Uuid,
    pub key_version_id: Uuid,
    pub key_version: i32,
}

pub async fn open_dm_channel(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<OpenDmChannelRequest>,
) -> Result<Json<OpenDmChannelResponse>, AppError> {
    if req.target_user_id == claims.user_id {
        return Err(AppError::BadRequest);
    }

    let peer_username = state
        .db
        .find_username_by_user_id(req.target_user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::UserNotFound)?;

    if let Some(channel_id) = state
        .db
        .find_dm_channel_by_users(claims.user_id, req.target_user_id)
        .await
        .map_err(AppError::DatabaseError)?
    {
        let current_ttl = state
            .db
            .get_dm_channel_ttl_for_member(channel_id, claims.user_id)
            .await
            .map_err(AppError::DatabaseError)?
            .flatten();

        let (key_version_id, key_version) = state
            .db
            .get_channel_key_version_metadata(channel_id)
            .await
            .map_err(AppError::DatabaseError)?
            .map(|(id, version)| (Some(id), Some(version)))
            .unwrap_or((None, None));

        return Ok(Json(OpenDmChannelResponse {
            dm_channel_id: channel_id,
            peer_user_id: req.target_user_id,
            peer_username,
            encryption_type: "asymmetric",
            message_ttl: current_ttl,
            key_version_id,
            key_version,
            created: false,
        }));
    }

    let channel_id = Uuid::new_v4();
    state
        .db
        .create_dm_channel(channel_id, claims.user_id, req.target_user_id, req.message_ttl)
        .await
        .map_err(AppError::DatabaseError)?;

    let key_version_id = state
        .db
        .create_channel_key_version(channel_id, 1, "", "", claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(Json(OpenDmChannelResponse {
        dm_channel_id: channel_id,
        peer_user_id: req.target_user_id,
        peer_username,
        encryption_type: "asymmetric",
        message_ttl: req.message_ttl,
        key_version_id: Some(key_version_id),
        key_version: Some(1),
        created: true,
    }))
}

pub async fn list_dm_channels(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<Vec<DmChannelListItem>>, AppError> {
    let rows = state
        .db
        .list_dm_channels_for_user(claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let unread_count = state
            .db
            .count_unread_messages_for_user(row.channel_id, claims.user_id)
            .await
            .unwrap_or(0);
        out.push(DmChannelListItem {
            dm_channel_id: row.channel_id,
            peer_user_id: row.peer_user_id,
            peer_username: row.peer_username,
            message_ttl: row.message_ttl,
            unread_count,
            last_message_at: row.last_message_at,
        });
    }

    Ok(Json(out))
}

pub async fn list_dm_channel_messages(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<PaginatedResponse>, AppError> {
    let maybe_ttl = state
        .db
        .get_dm_channel_ttl_for_member(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    if maybe_ttl.is_none() {
        return Err(AppError::Forbidden);
    }

    let since_dt = query
        .since
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Utc));

    let messages = state
        .db
        .list_messages(channel_id, query.limit, query.after, query.before, since_dt)
        .await
        .map_err(AppError::DatabaseError)?;

    let has_more = messages.len() == query.limit;
    let next_cursor = if has_more { messages.last().map(|m| m.id) } else { None };

    Ok(Json(PaginatedResponse {
        data: messages,
        pagination: PaginationMeta {
            has_more,
            next_cursor,
            prev_cursor: None,
            total_new: None,
        },
    }))
}

pub async fn send_dm_channel_message(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<Message>), AppError> {
    let dm_ttl = state
        .db
        .get_dm_channel_ttl_for_member(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let dm_ttl = match dm_ttl {
        Some(v) => v,
        None => return Err(AppError::Forbidden),
    };

    let message_id = Uuid::new_v4();
    let timestamp = chrono::Utc::now();
    let request_expires_at = req
        .expires_at
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&chrono::Utc));
    let channel_expires_at = dm_ttl.map(|ttl| timestamp + Duration::seconds(i64::from(ttl)));
    let expires_at = channel_expires_at.or(request_expires_at);
    let key_version = match req.key_version {
        Some(version) => Some(version),
        None => state
            .db
            .get_channel_key_version_metadata(channel_id)
            .await
            .map_err(AppError::DatabaseError)?
            .map(|(_, version)| version),
    };
    if key_version.is_none() {
        return Err(AppError::BadRequest);
    }

    state
        .db
        .create_message(
            message_id,
            channel_id,
            claims.user_id,
            &claims.username,
            claims.device_id,
            &req.encrypted_payload,
            &req.iv,
            key_version,
            expires_at,
            timestamp,
        )
        .await
        .map_err(AppError::DatabaseError)?;

    let message = Message {
        id: message_id,
        channel_id,
        sender_user_id: claims.user_id,
        sender_username: Some(claims.username.clone()),
        sender_device_id: claims.device_id,
        encrypted_payload: req.encrypted_payload,
        iv: req.iv,
        key_version,
        timestamp,
        expires_at,
        edited_at: None,
        deleted_at: None,
    };

    let room = format!("channel:{}", channel_id);
    let socket_event = serde_json::json!({
        "messageId": message.id,
        "channelId": message.channel_id,
        "senderUserId": message.sender_user_id,
        "senderUsername": message.sender_username,
        "senderDeviceId": message.sender_device_id,
        "encryptedPayload": message.encrypted_payload,
        "iv": message.iv,
        "keyVersion": message.key_version,
        "timestamp": message.timestamp,
        "editedAt": message.edited_at,
        "deletedAt": message.deleted_at,
    });

    if let Err(e) = state.io.to(room).emit("message", &socket_event).await {
        tracing::warn!("Error fent broadcast del missatge DM via socket: {:?}", e);
    }

    if let Ok(member_ids) = state.db.list_channel_member_ids(channel_id).await {
        for member_id in member_ids.into_iter().filter(|id| *id != claims.user_id) {
            match state.db.count_unread_messages_for_user(channel_id, member_id).await {
                Ok(unread_count) => {
                    let unread_event = serde_json::json!({
                        "channelId": channel_id,
                        "unreadCount": unread_count,
                    });
                    let user_room = format!("user:{}", member_id);
                    if let Err(e) = state.io.to(user_room).emit("unread-updated", &unread_event).await {
                        tracing::warn!("Error enviant unread-updated DM: {:?}", e);
                    }
                }
                Err(e) => tracing::warn!("Error calculant unread DM per usuari {}: {}", member_id, e),
            }
        }
    }

    Ok((StatusCode::CREATED, Json(message)))
}

pub async fn update_dm_channel_settings(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<UpdateDmSettingsRequest>,
) -> Result<Json<UpdateDmSettingsResponse>, AppError> {
    let member = state
        .db
        .get_dm_channel_ttl_for_member(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    if member.is_none() {
        return Err(AppError::Forbidden);
    }

    state
        .db
        .update_dm_channel_ttl(channel_id, req.message_ttl)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(Json(UpdateDmSettingsResponse {
        dm_channel_id: channel_id,
        message_ttl: req.message_ttl,
    }))
}

pub async fn rotate_dm_channel_key(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<RotateDmKeyResponse>, AppError> {
    let member = state
        .db
        .get_dm_channel_ttl_for_member(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    if member.is_none() {
        return Err(AppError::Forbidden);
    }

    let next_version = state
        .db
        .get_latest_channel_key_version(channel_id)
        .await
        .map_err(AppError::DatabaseError)?
        .map(|(_, version, _, _)| version + 1)
        .unwrap_or(1);

    let key_version_id = state
        .db
        .create_channel_key_version(channel_id, next_version, "", "", claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(Json(RotateDmKeyResponse {
        dm_channel_id: channel_id,
        key_version_id,
        key_version: next_version,
    }))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/channels/{channel_id}/messages", get(list_messages).post(send_message))
        .route("/api/messages/{message_id}", get(get_message).put(edit_message).delete(delete_message))
        .route("/api/channels/{channel_id}/messages/check-new", get(check_new_messages))
    .route("/api/dm/channels/open", post(open_dm_channel))
    .route("/api/dm/channels", get(list_dm_channels))
    .route("/api/dm/channels/{channel_id}/messages", get(list_dm_channel_messages).post(send_dm_channel_message))
    .route("/api/dm/channels/{channel_id}/settings", put(update_dm_channel_settings))
    .route("/api/dm/channels/{channel_id}/keys/rotate", post(rotate_dm_channel_key))
        .route("/api/direct-messages", post(send_direct_message))
        .route("/api/direct-messages/list", get(list_direct_messages))
        .route("/api/conversations", get(list_conversations))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, LogLevel},
        db::connect_db,
    };
    use axum::Extension;
    use socketioxide::extract::{Data, SocketRef};
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn test_config() -> Config {
        Config {
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
        }
    }

    async fn make_state() -> AppState {
        let config = test_config();
        let db = connect_db(&config).await.expect("sqlite test db should initialize");
        let (_layer, io) = socketioxide::SocketIo::new_layer();
        io.ns("/", |_socket: SocketRef, Data(_auth): Data<serde_json::Value>| async move {});
        AppState {
            db,
            config,
            io,
            user_presence: Arc::new(RwLock::new(crate::middleware::auth::UserPresenceState {
                online_sockets: HashMap::<Uuid, HashSet<String>>::new(),
            })),
        }
    }

    fn make_claims(user_id: Uuid, username: &str) -> AuthClaims {
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
    async fn send_message_to_channel_succeeds_for_member() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user("msg_send_owner", "hash")
            .await
            .expect("create owner");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "msg-send-server", None, owner_id)
            .await
            .expect("create server");

        let channel_id = Uuid::new_v4();
        state
            .db
            .create_channel(channel_id, server_id, "general", "text", "none", None, false)
            .await
            .expect("create channel");

        let result = send_message(
            State(state),
            Extension(make_claims(owner_id, "msg_send_owner")),
            Path(channel_id),
            Json(SendMessageRequest {
                encrypted_payload: "ciphertext-hello".to_string(),
                iv: "iv-hello".to_string(),
                key_version: None,
                expires_at: None,
                is_direct: None,
                recipient_user_id: None,
            }),
        )
        .await;

        assert!(result.is_ok(), "server owner should be able to send a message");
        let (status, Json(msg)) = result.expect("should return message");
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(msg.channel_id, channel_id);
        assert_eq!(msg.sender_user_id, owner_id);
        assert_eq!(msg.encrypted_payload, "ciphertext-hello");
    }

    #[tokio::test]
    async fn send_message_forbidden_for_non_member() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user("msg_forbidden_owner", "hash")
            .await
            .expect("create owner");
        let outsider_id = state
            .db
            .create_user("msg_forbidden_outsider", "hash")
            .await
            .expect("create outsider");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "msg-forbidden-server", None, owner_id)
            .await
            .expect("create server");

        let channel_id = Uuid::new_v4();
        state
            .db
            .create_channel(channel_id, server_id, "general", "text", "none", None, false)
            .await
            .expect("create channel");

        let result = send_message(
            State(state),
            Extension(make_claims(outsider_id, "msg_forbidden_outsider")),
            Path(channel_id),
            Json(SendMessageRequest {
                encrypted_payload: "unauthorized".to_string(),
                iv: "iv".to_string(),
                key_version: None,
                expires_at: None,
                is_direct: None,
                recipient_user_id: None,
            }),
        )
        .await;

        assert!(matches!(result, Err(AppError::Forbidden)), "non-member should be forbidden");
    }

    #[tokio::test]
    async fn send_message_to_nonexistent_channel_returns_not_found() {
        let state = make_state().await;
        let user_id = state
            .db
            .create_user("msg_notfound_user", "hash")
            .await
            .expect("create user");

        let result = send_message(
            State(state),
            Extension(make_claims(user_id, "msg_notfound_user")),
            Path(Uuid::new_v4()),
            Json(SendMessageRequest {
                encrypted_payload: "ciphertext".to_string(),
                iv: "iv".to_string(),
                key_version: None,
                expires_at: None,
                is_direct: None,
                recipient_user_id: None,
            }),
        )
        .await;

        assert!(matches!(result, Err(AppError::ChannelNotFound)), "missing channel should return ChannelNotFound");
    }

    #[tokio::test]
    async fn edit_message_forbidden_for_non_sender() {
        let state = make_state().await;
        let sender_id = state
            .db
            .create_user("msg_edit_sender", "hash")
            .await
            .expect("create sender");
        let other_id = state
            .db
            .create_user("msg_edit_other", "hash")
            .await
            .expect("create other user");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "msg-edit-server", None, sender_id)
            .await
            .expect("create server");

        let channel_id = Uuid::new_v4();
        state
            .db
            .create_channel(channel_id, server_id, "general", "text", "none", None, false)
            .await
            .expect("create channel");

        let message_id = Uuid::new_v4();
        let now = Utc::now();
        state
            .db
            .create_message(
                message_id,
                channel_id,
                sender_id,
                "msg_edit_sender",
                Uuid::new_v4(),
                "original",
                "iv",
                None,
                None,
                now,
            )
            .await
            .expect("create message");

        let result = edit_message(
            State(state),
            Extension(make_claims(other_id, "msg_edit_other")),
            Path(message_id),
            Json(EditMessageRequest {
                encrypted_payload: "tampered".to_string(),
                iv: "iv2".to_string(),
            }),
        )
        .await;

        assert!(
            matches!(result, Err(AppError::NotMessageSender)),
            "non-sender should not be able to edit a message"
        );
    }

    #[tokio::test]
    async fn delete_message_forbidden_for_non_sender() {
        let state = make_state().await;
        let sender_id = state
            .db
            .create_user("msg_del_sender", "hash")
            .await
            .expect("create sender");
        let other_id = state
            .db
            .create_user("msg_del_other", "hash")
            .await
            .expect("create other user");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "msg-del-server", None, sender_id)
            .await
            .expect("create server");

        let channel_id = Uuid::new_v4();
        state
            .db
            .create_channel(channel_id, server_id, "general", "text", "none", None, false)
            .await
            .expect("create channel");

        let message_id = Uuid::new_v4();
        let now = Utc::now();
        state
            .db
            .create_message(
                message_id,
                channel_id,
                sender_id,
                "msg_del_sender",
                Uuid::new_v4(),
                "content",
                "iv",
                None,
                None,
                now,
            )
            .await
            .expect("create message");

        let result = delete_message(
            State(state),
            Extension(make_claims(other_id, "msg_del_other")),
            Path(message_id),
        )
        .await;

        assert!(
            matches!(result, Err(AppError::NotMessageSender)),
            "non-sender should not be able to delete a message"
        );
    }

    #[tokio::test]
    async fn delete_message_succeeds_for_sender() {
        let state = make_state().await;
        let sender_id = state
            .db
            .create_user("msg_del_ok_sender", "hash")
            .await
            .expect("create sender");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "msg-del-ok-server", None, sender_id)
            .await
            .expect("create server");

        let channel_id = Uuid::new_v4();
        state
            .db
            .create_channel(channel_id, server_id, "general", "text", "none", None, false)
            .await
            .expect("create channel");

        let message_id = Uuid::new_v4();
        let now = Utc::now();
        state
            .db
            .create_message(
                message_id,
                channel_id,
                sender_id,
                "msg_del_ok_sender",
                Uuid::new_v4(),
                "content",
                "iv",
                None,
                None,
                now,
            )
            .await
            .expect("create message");

        let result = delete_message(
            State(state),
            Extension(make_claims(sender_id, "msg_del_ok_sender")),
            Path(message_id),
        )
        .await;

        assert!(result.is_ok(), "sender should be able to delete their own message");
        assert_eq!(result.expect("should return status"), StatusCode::OK);
    }

    #[tokio::test]
    async fn dm_open_is_idempotent() {
        let state = make_state().await;
        let user_a = state
            .db
            .create_user("dm_idempotent_a", "hash")
            .await
            .expect("create user a");
        let user_b = state
            .db
            .create_user("dm_idempotent_b", "hash")
            .await
            .expect("create user b");

        let claims = make_claims(user_a, "dm_idempotent_a");
        let first = open_dm_channel(
            State(state.clone()),
            Extension(claims.clone()),
            Json(OpenDmChannelRequest {
                target_user_id: user_b,
                message_ttl: Some(3600),
            }),
        )
        .await
        .expect("first open should work")
        .0;

        let second = open_dm_channel(
            State(state.clone()),
            Extension(claims),
            Json(OpenDmChannelRequest {
                target_user_id: user_b,
                message_ttl: Some(120),
            }),
        )
        .await
        .expect("second open should work")
        .0;

        assert!(first.created);
        assert!(!second.created);
        assert_eq!(first.dm_channel_id, second.dm_channel_id);
    }

    #[tokio::test]
    async fn dm_access_is_restricted_to_members() {
        let state = make_state().await;
        let user_a = state
            .db
            .create_user("dm_access_a", "hash")
            .await
            .expect("create user a");
        let user_b = state
            .db
            .create_user("dm_access_b", "hash")
            .await
            .expect("create user b");
        let user_c = state
            .db
            .create_user("dm_access_c", "hash")
            .await
            .expect("create user c");

        let open = open_dm_channel(
            State(state.clone()),
            Extension(make_claims(user_a, "dm_access_a")),
            Json(OpenDmChannelRequest {
                target_user_id: user_b,
                message_ttl: None,
            }),
        )
        .await
        .expect("open dm")
        .0;

        let list_res = list_dm_channel_messages(
            State(state.clone()),
            Extension(make_claims(user_c, "dm_access_c")),
            Path(open.dm_channel_id),
            Query(ListMessagesQuery {
                limit: 50,
                after: None,
                before: None,
                since: None,
                new_only: None,
            }),
        )
        .await;

        assert!(matches!(list_res, Err(AppError::Forbidden)));
    }

    #[tokio::test]
    async fn dm_send_applies_channel_ttl_by_default() {
        let state = make_state().await;
        let user_a = state
            .db
            .create_user("dm_ttl_a", "hash")
            .await
            .expect("create user a");
        let user_b = state
            .db
            .create_user("dm_ttl_b", "hash")
            .await
            .expect("create user b");

        let open = open_dm_channel(
            State(state.clone()),
            Extension(make_claims(user_a, "dm_ttl_a")),
            Json(OpenDmChannelRequest {
                target_user_id: user_b,
                message_ttl: Some(60),
            }),
        )
        .await
        .expect("open dm")
        .0;

        let sent = send_dm_channel_message(
            State(state.clone()),
            Extension(make_claims(user_a, "dm_ttl_a")),
            Path(open.dm_channel_id),
            Json(SendMessageRequest {
                encrypted_payload: "ciphertext".to_string(),
                iv: "iv".to_string(),
                key_version: None,
                expires_at: None,
                is_direct: None,
                recipient_user_id: None,
            }),
        )
        .await
        .expect("send dm should work")
        .1
        .0;

        let expires_at = sent.expires_at.expect("expires_at should be set from dm ttl");
        assert!(expires_at > sent.timestamp);
    }

    #[tokio::test]
    async fn dm_rotate_key_increments_version() {
        let state = make_state().await;
        let user_a = state
            .db
            .create_user("dm_rotate_a", "hash")
            .await
            .expect("create user a");
        let user_b = state
            .db
            .create_user("dm_rotate_b", "hash")
            .await
            .expect("create user b");

        let open = open_dm_channel(
            State(state.clone()),
            Extension(make_claims(user_a, "dm_rotate_a")),
            Json(OpenDmChannelRequest {
                target_user_id: user_b,
                message_ttl: None,
            }),
        )
        .await
        .expect("open dm")
        .0;

        let rotate = rotate_dm_channel_key(
            State(state.clone()),
            Extension(make_claims(user_a, "dm_rotate_a")),
            Path(open.dm_channel_id),
        )
        .await
        .expect("rotate should work")
        .0;

        assert_eq!(rotate.key_version, 2);
    }

    #[tokio::test]
    async fn dm_rotate_key_forbidden_for_non_member() {
        let state = make_state().await;
        let user_a = state
            .db
            .create_user("dm_rotate_forbidden_a", "hash")
            .await
            .expect("create user a");
        let user_b = state
            .db
            .create_user("dm_rotate_forbidden_b", "hash")
            .await
            .expect("create user b");
        let user_c = state
            .db
            .create_user("dm_rotate_forbidden_c", "hash")
            .await
            .expect("create user c");

        let open = open_dm_channel(
            State(state.clone()),
            Extension(make_claims(user_a, "dm_rotate_forbidden_a")),
            Json(OpenDmChannelRequest {
                target_user_id: user_b,
                message_ttl: None,
            }),
        )
        .await
        .expect("open dm")
        .0;

        let rotate = rotate_dm_channel_key(
            State(state.clone()),
            Extension(make_claims(user_c, "dm_rotate_forbidden_c")),
            Path(open.dm_channel_id),
        )
        .await;

        assert!(matches!(rotate, Err(AppError::Forbidden)));
    }
}
