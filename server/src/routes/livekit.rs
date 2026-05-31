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
use uuid::Uuid;
use crate::middleware::{AppState, AuthClaims};
use crate::error::AppError;
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct LiveKitTokenRequest {
    pub room: String,
    pub participant: Option<String>,
    pub server_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct LiveKitTokenResponse {
    pub token: String,
    pub url: String,
}

/// Generar un token de LiveKit per a una sala.
pub async fn generate_token(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<LiveKitTokenRequest>,
) -> Result<Json<LiveKitTokenResponse>, AppError> {
    info!("Endpoint generate_token cridat: room={}, user_id={}", req.room, claims.user_id);

    let mut livekit_host = state.config.livekit_host.clone();
    let mut livekit_api_key = state.config.livekit_api_key.clone();
    let mut livekit_api_secret = state.config.livekit_api_secret.clone();

    let resolved_server_id = match req.server_id {
        Some(server_id) => Some(server_id),
        None => {
            let channel_id = req
                .room
                .strip_prefix("chillgroup-")
                .and_then(|value| Uuid::parse_str(value).ok());

            match channel_id {
                Some(channel_id) => state
                    .db
                    .get_channel(channel_id)
                    .await
                    .map_err(AppError::DatabaseError)?
                    .map(|channel| channel.server_id),
                None => None,
            }
        }
    };

    if let Some(server_id) = resolved_server_id {
        if let Some(override_config) = state
            .db
            .get_server_livekit_override(server_id)
            .await
            .map_err(AppError::DatabaseError)?
        {
            livekit_host = override_config.host;
            livekit_api_key = override_config.api_key;
            livekit_api_secret = override_config.api_secret;
        }
    }

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
        iss: &livekit_api_key,
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
        &EncodingKey::from_secret(livekit_api_secret.as_bytes()),
    )
    .map_err(|e| {
        info!("Error generant token LiveKit: {}", e);
        AppError::LiveKitTokenError
    })?;

    info!("Token LiveKit generat amb èxit per a room={}", req.room);
    Ok(Json(LiveKitTokenResponse {
        token,
        url: livekit_host,
    }))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/livekit/token", post(generate_token))
        .with_state(state)
}
