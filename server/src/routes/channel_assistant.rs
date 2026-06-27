//! Endpoints de control de l'assistent de veu d'un canal (start/stop).
//!
//! La lògica de captura/transcripció viu a `services::channel_assistant`. Aquí
//! només validem permisos i deleguem. Cal permís d'escriptura al canal (o admin).

#![allow(dead_code)]

use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::db::CHANNEL_PERMISSION_WRITE;
use crate::error::AppError;
use crate::middleware::{AppState, AuthClaims};
use crate::services::channel_assistant;

#[derive(Debug, Default, Deserialize)]
pub struct StartAssistantRequest {
    /// Clau de canal en base64. Obligatòria per canals asimètrics (el server no
    /// la custodia), ignorada per simètrics.
    #[serde(default, alias = "channelKey")]
    pub channel_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AssistantResponse {
    pub success: bool,
    /// URL del fitxer Markdown exportat (només a `stop`, si hi ha transcripció).
    #[serde(rename = "fileUrl", skip_serializing_if = "Option::is_none")]
    pub file_url: Option<String>,
}

/// Comprova que l'usuari té permís d'escriptura al canal (o és admin).
async fn ensure_write_permission(
    state: &AppState,
    channel_id: Uuid,
    claims: &AuthClaims,
) -> Result<(), AppError> {
    if claims.is_admin {
        return Ok(());
    }
    let level = state
        .db
        .get_channel_permission_level(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .unwrap_or(0);
    if level < CHANNEL_PERMISSION_WRITE {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

/// `POST /api/channels/{channel_id}/assistant/start`
pub async fn start_assistant(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    body: Option<Json<StartAssistantRequest>>,
) -> Result<Json<AssistantResponse>, AppError> {
    info!("start_assistant: channel_id={}, user_id={}", channel_id, claims.user_id);
    ensure_write_permission(&state, channel_id, &claims).await?;

    let channel_key = body.and_then(|Json(req)| req.channel_key);
    channel_assistant::start_session(state.db.clone(), state.config.clone(), channel_id, channel_key)
        .await?;

    Ok(Json(AssistantResponse { success: true, file_url: None }))
}

/// `POST /api/channels/{channel_id}/assistant/stop`
pub async fn stop_assistant(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<AssistantResponse>, AppError> {
    info!("stop_assistant: channel_id={}, user_id={}", channel_id, claims.user_id);
    ensure_write_permission(&state, channel_id, &claims).await?;

    let file_url = channel_assistant::stop_session(channel_id).await?;
    Ok(Json(AssistantResponse { success: true, file_url }))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/channels/{channel_id}/assistant/start", post(start_assistant))
        .route("/api/channels/{channel_id}/assistant/stop", post(stop_assistant))
        .with_state(state)
}
