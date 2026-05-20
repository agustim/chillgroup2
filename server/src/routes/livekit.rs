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
use crate::middleware::{AppState, AuthClaims};
use crate::error::AppError;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct LiveKitTokenRequest {
    pub room: String,
    pub participant: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LiveKitTokenResponse {
    pub token: String,
}

/// Generar un token de LiveKit per a una sala.
pub async fn generate_token(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<LiveKitTokenRequest>,
) -> Result<Json<LiveKitTokenResponse>, AppError> {
    info!("Endpoint generate_token cridat: room={}, user_id={}", req.room, claims.user_id);

    #[derive(Serialize)]
    struct VideoGrant<'a> {
        room: &'a str,
        #[serde(rename = "roomJoin")]
        room_join: bool,
        #[serde(rename = "canPublish")]
        can_publish: bool,
        #[serde(rename = "canSubscribe")]
        can_subscribe: bool,
    }

    #[derive(Serialize)]
    struct LiveKitClaims<'a> {
        iss: &'a str,
        sub: &'a str,
        exp: i64,
        nbf: i64,
        iat: i64,
        name: &'a str,
        video: VideoGrant<'a>,
    }

    let now = Utc::now();
    let expiration = (now + Duration::hours(23)).timestamp(); // 23 hores de validesa
    let participant_identity = claims.user_id.to_string();

    let claims = LiveKitClaims {
        iss: &state.config.livekit_api_key,
        // Per LiveKit, sub ha de ser la identitat del participant, no l'API secret.
        sub: &participant_identity,
        exp: expiration,
        nbf: now.timestamp(),
        iat: now.timestamp(),
        name: &claims.username,
        video: VideoGrant {
            room: &req.room,
            room_join: true,
            can_publish: true,
            can_subscribe: true,
        },
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
