//! Endpoints de LiveKit per a generació de tokens.

#![allow(dead_code)]

use axum::{
    extract::State,
    Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, Header, EncodingKey};
use crate::middleware::AppState;
use crate::error::AppError;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct LiveKitTokenRequest {
    pub room: String,
    pub participant: String,
}

#[derive(Debug, Serialize)]
pub struct LiveKitTokenResponse {
    pub token: String,
}

/// Generar un token de LiveKit per a una sala.
pub async fn generate_token(
    State(state): State<AppState>,
    Json(req): Json<LiveKitTokenRequest>,
) -> Result<Json<LiveKitTokenResponse>, AppError> {
    info!("Endpoint generate_token cridat: room={}, participant={}", req.room, req.participant);

    #[derive(Serialize)]
    struct LiveKitClaims {
        iss: String,
        sub: String,
        exp: i64,
        nbf: i64,
        iat: i64,
        name: String,
        room_join: bool,
        room: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        can_subscribe: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        can_publish: Option<bool>,
    }

    let now = Utc::now();
    let expiration = (now + Duration::minutes(5)).timestamp();

    let claims = LiveKitClaims {
        iss: state.config.livekit_api_key.clone(),
        sub: state.config.livekit_api_key.clone(),
        exp: expiration,
        nbf: now.timestamp(),
        iat: now.timestamp(),
        name: req.participant.clone(),
        room_join: true,
        room: req.room.clone(),
        can_subscribe: Some(true),
        can_publish: Some(true),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.livekit_api_secret.as_bytes()),
    )
    .map_err(|e| {
        info!("Error generant token LiveKit: {}", e);
        AppError::LiveKitTokenError
    })?;

    info!("Token LiveKit generat amb èxit per a room={}", req.room);
    Ok(Json(LiveKitTokenResponse { token }))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/livekit/token", post(generate_token))
        .with_state(state)
}