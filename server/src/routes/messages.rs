//! Endpoints de missatges.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing::{get, post, put, delete},
    Router, extract::Path, extract::Query,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    middleware::{AppState, AuthClaims},
    error::AppError,
    models::Message,
};

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub encrypted_payload: String,
    pub iv: String,
    pub expires_at: Option<String>,
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
    pub before: Option<Uuid>,
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
    pub next_cursor: Uuid,
}

pub async fn list_messages(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<PaginatedResponse>, AppError> {
    // TODO: Query DB amb paginació
    // SELECT * FROM messages WHERE channel_id = $1 AND deleted_at IS NULL
    // ORDER BY timestamp DESC LIMIT $2
    // (si before es proporciona, afegir AND id < $3)

    Ok(Json(PaginatedResponse {
        data: vec![],
        pagination: PaginationMeta {
            has_more: false,
            next_cursor: Uuid::nil(),
        },
    }))
}

pub async fn send_message(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<Message>), AppError> {
    // TODO: Verificar permisos del canal
    // TODO: INSERT message
    // TODO: Emitir via Socket.IO

    let message_id = Uuid::new_v4();
    let expires_at = req.expires_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok()).map(|d| d.with_timezone(&chrono::Utc));

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

pub async fn edit_message(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(message_id): Path<Uuid>,
    Json(req): Json<EditMessageRequest>,
) -> Result<Json<Message>, AppError> {
    // TODO: Verificar que és el remitent
    // TODO: Verificar temps màxim (5 minuts)
    // TODO: UPDATE message SET encrypted_payload = $1, iv = $2, edited_at = now() WHERE id = $3

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

pub async fn delete_message(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(message_id): Path<Uuid>,
) -> Result<Json<Message>, AppError> {
    // TODO: Verificar permisos
    // TODO: UPDATE message SET deleted_at = now() WHERE id = $1

    Ok(Json(Message {
        id: message_id,
        channel_id: Uuid::nil(),
        sender_user_id: claims.user_id,
        sender_device_id: claims.device_id,
        encrypted_payload: String::new(),
        iv: String::new(),
        timestamp: chrono::Utc::now(),
        expires_at: None,
        edited_at: None,
        deleted_at: Some(chrono::Utc::now()),
    }))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/channels/:channel_id/messages", get(list_messages).post(send_message))
        .route("/api/messages/:message_id", put(edit_message).delete(delete_message))
        .with_state(state)
}