//! Endpoints de missatges.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing::{get, post},
    Router, extract::Path, extract::Query,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    middleware::{AppState, AuthClaims},
    error::AppError,
    models::Message,
};
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub encrypted_payload: String,
    pub iv: String,
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
        Some(message) => Ok(Json(message)),
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

    // Persist to DB
    state.db.create_message(
        message_id,
        channel_id,
        claims.user_id,
        &claims.username,
        claims.device_id,
        &req.encrypted_payload,
        &req.iv,
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

    // Persist to DB
    state.db.create_message(
        message_id,
        Uuid::nil(), // DM: no channel
        claims.user_id,
        &claims.username,
        claims.device_id,
        &req.encrypted_payload,
        &req.iv,
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

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/channels/{channel_id}/messages", get(list_messages).post(send_message))
        .route("/api/messages/{message_id}", get(get_message).put(edit_message).delete(delete_message))
        .route("/api/channels/{channel_id}/messages/check-new", get(check_new_messages))
        .route("/api/direct-messages", post(send_direct_message))
        .route("/api/direct-messages/list", get(list_direct_messages))
        .route("/api/conversations", get(list_conversations))
        .with_state(state)
}
