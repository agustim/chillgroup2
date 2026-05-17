//! Endpoints de missatges.

#![allow(dead_code)]

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing::{get, post},
    Router, extract::Path, extract::Query,
};
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
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(message_id): Path<Uuid>,
) -> Result<Json<Message>, AppError> {
    info!("Endpoint get_message cridat: message_id={}, user_id={}", message_id, claims.user_id);
    // TODO: Query DB: SELECT * FROM messages WHERE id = $1

    Ok(Json(Message {
        id: message_id,
        channel_id: Uuid::nil(),
        sender_user_id: claims.user_id,
        sender_device_id: claims.device_id,
        encrypted_payload: "placeholder".to_string(),
        iv: "placeholder".to_string(),
        timestamp: chrono::Utc::now(),
        expires_at: None,
        edited_at: None,
        deleted_at: None,
    }))
}

/// Llistar missatges d'un canal amb paginació.
pub async fn list_messages(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<PaginatedResponse>, AppError> {
    info!(
        "Endpoint list_messages cridat: channel_id={}, user_id={}, limit={}, after={:?}, before={:?}, since={:?}",
        channel_id, claims.user_id, query.limit, query.after, query.before, query.since
    );

    // TODO: Query DB amb paginació:
    // SELECT * FROM messages
    // WHERE channel_id = $1 AND deleted_at IS NULL
    // AND (before IS NULL OR id < before)
    // AND (after IS NULL OR id > after)
    // AND (since IS NULL OR timestamp > since::timestamp)
    // ORDER BY timestamp ASC
    // LIMIT $2

    Ok(Json(PaginatedResponse {
        data: vec![],
        pagination: PaginationMeta {
            has_more: false,
            next_cursor: None,
            prev_cursor: None,
            total_new: query.new_only.and_then(|b| if b { Some(0) } else { None }),
        },
    }))
}

/// Consultar si hi ha missatges nous des d'un timestamp concret.
/// Aquest endpoint és útil per saber si cal descarregar nous missatges
/// quan l'usuari torna a entrar a un canal.
pub async fn check_new_messages(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<CheckNewMessagesQuery>,
) -> Result<Json<NewMessagesResponse>, AppError> {
    info!(
        "Endpoint check_new_messages cridat: channel_id={}, user_id={}, since={}",
        channel_id, claims.user_id, query.last_seen
    );

    // TODO: Query DB:
    // SELECT COUNT(*) FROM messages
    // WHERE channel_id = $1 AND sender_user_id != $2 AND deleted_at IS NULL
    // AND timestamp > $3

    Ok(Json(NewMessagesResponse {
        channel_id,
        has_new: true,
        new_count: 0,
        first_new_message_id: None,
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
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<Message>), AppError> {
    info!("Endpoint send_message cridat: channel_id={}, user_id={}", channel_id, claims.user_id);

    let message_id = Uuid::new_v4();
    let expires_at = req.expires_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&chrono::Utc));
    info!("Missatge enviat amb èxit: message_id={}", message_id);

    Ok((
        StatusCode::CREATED,
        Json(Message {
            id: message_id,
            channel_id,
            sender_user_id: claims.user_id,
            sender_device_id: claims.device_id,
            encrypted_payload: req.encrypted_payload,
            iv: req.iv,
            timestamp: chrono::Utc::now(),
            expires_at,
            edited_at: None,
            deleted_at: None,
        }),
    ))
}

/// Editar un missatge existent.
pub async fn edit_message(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(message_id): Path<Uuid>,
    Json(req): Json<EditMessageRequest>,
) -> Result<Json<Message>, AppError> {
    info!("Endpoint edit_message cridat: message_id={}, user_id={}", message_id, claims.user_id);

    Ok(Json(Message {
        id: message_id,
        channel_id: Uuid::nil(),
        sender_user_id: claims.user_id,
        sender_device_id: claims.device_id,
        encrypted_payload: req.encrypted_payload,
        iv: req.iv,
        timestamp: chrono::Utc::now(),
        expires_at: None,
        edited_at: Some(chrono::Utc::now()),
        deleted_at: None,
    }))
}

/// Eliminar un missatge (soft delete).
pub async fn delete_message(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(message_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    info!("Endpoint delete_message cridat: message_id={}, user_id={}", message_id, claims.user_id);

    // TODO: UPDATE messages SET deleted_at = now() WHERE id = $1

    Ok(StatusCode::OK)
}

/// Enviar un missatge directe (DM) a un altre usuari.
pub async fn send_direct_message(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<Message>), AppError> {
    info!(
        "Endpoint send_direct_message cridat: from={}, to={}, is_direct={}",
        claims.user_id, req.recipient_user_id.unwrap_or(Uuid::nil()), req.is_direct.unwrap_or(false)
    );

    // TODO: Validar que el missatge és directe
    // TODO: Verificar que el recipient existeix
    // TODO: INSERT message amb recipient_user_id

    let message_id = Uuid::new_v4();
    let expires_at = req.expires_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&chrono::Utc));

    info!("Missatge directe enviat: message_id={}", message_id);

    Ok((
        StatusCode::CREATED,
        Json(Message {
            id: message_id,
            channel_id: Uuid::nil(), // DM no té canal
            sender_user_id: claims.user_id,
            sender_device_id: claims.device_id,
            encrypted_payload: req.encrypted_payload,
            iv: req.iv,
            timestamp: chrono::Utc::now(),
            expires_at,
            edited_at: None,
            deleted_at: None,
        }),
    ))
}

/// Llistar missatges directes entre dos usuaris.
pub async fn list_direct_messages(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Query(query): Query<ListDirectMessagesQuery>,
) -> Result<Json<PaginatedResponse>, AppError> {
    info!(
        "Endpoint list_direct_messages cridat: user1={}, user2={}, limit={}",
        claims.user_id, query.with_user, query.limit
    );

    // TODO: Query DB:
    // SELECT * FROM messages
    // WHERE (sender_user_id = $1 AND recipient_user_id = $2)
    //    OR (sender_user_id = $2 AND recipient_user_id = $1)
    // AND is_direct = true AND deleted_at IS NULL
    // ORDER BY timestamp ASC LIMIT $3

    Ok(Json(PaginatedResponse {
        data: vec![],
        pagination: PaginationMeta {
            has_more: false,
            next_cursor: None,
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

    // TODO: Query DB:
    // SELECT DISTINCT
    //     CASE WHEN sender_user_id = $1 THEN recipient_user_id ELSE sender_user_id END as other_user,
    //     MAX(timestamp) as last_message_at,
    //     COUNT(CASE WHEN recipient_user_id = $1 AND read_at IS NULL THEN 1 END) as unread_count
    // FROM messages
    // WHERE (sender_user_id = $1 OR recipient_user_id = $1)
    // AND is_direct = true AND deleted_at IS NULL
    // GROUP BY other_user
    // ORDER BY last_message_at DESC

    Ok(Json(vec![]))
}

#[derive(Debug, Serialize)]
pub struct DirectMessageConversation {
    pub other_user_id: Uuid,
    pub other_user_username: String,
    pub other_user_avatar: Option<String>,
    pub last_message_at: chrono::DateTime<chrono::Utc>,
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