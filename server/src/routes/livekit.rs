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
use std::time::{Duration as StdDuration, Instant};
use crate::middleware::{AppState, AuthClaims};
use crate::error::AppError;
use crate::db::CHANNEL_PERMISSION_WRITE;
use tracing::info;

/// Minimum gap between quota charges for the same user+room.
/// Prevents repeated token requests (reconnects, refreshes) from draining quota.
const LIVEKIT_CHARGE_COOLDOWN: StdDuration = StdDuration::from_secs(55 * 60);

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

    let year_month = Utc::now().format("%Y-%m").to_string();
    let max_hours = state
        .db
        .get_user_streaming_quota(claims.user_id)
        .await
        .map_err(|e| AppError::DatabaseError(sqlx::Error::Protocol(e)))?;
    if max_hours >= 0 {
        let used_seconds = state
            .db
            .get_user_streaming_usage(claims.user_id, &year_month)
            .await
            .map_err(|e| AppError::DatabaseError(sqlx::Error::Protocol(e)))?;
        let max_seconds = i64::from(max_hours) * 3600;
        if used_seconds >= max_seconds {
            return Err(AppError::StreamingQuotaExceeded);
        }
        // Only charge if this user+room hasn't been charged within the cooldown window.
        // Prevents repeated token requests (reconnects, tab refreshes) from draining quota.
        let cache_key = (claims.user_id, req.room.clone());
        let should_charge = {
            let mut cache = state.livekit_token_cache.lock().await;
            let now = Instant::now();
            match cache.get(&cache_key) {
                Some(last) if now.duration_since(*last) < LIVEKIT_CHARGE_COOLDOWN => false,
                _ => {
                    cache.insert(cache_key, now);
                    true
                }
            }
        };
        if should_charge {
            let _ = state
                .db
                .increment_streaming_seconds(claims.user_id, &year_month, 3600)
                .await;
        }
    }

    let mut livekit_host = state.config.livekit_host.clone();
    let mut livekit_api_key = state.config.livekit_api_key.clone();
    let mut livekit_api_secret = state.config.livekit_api_secret.clone();

    let channel_id = req
        .room
        .strip_prefix("chillgroup-")
        .and_then(|value| Uuid::parse_str(value).ok());

    let resolved_server_id = match req.server_id {
        Some(server_id) => Some(server_id),
        None => match channel_id {
            Some(channel_id) => state
                .db
                .get_channel(channel_id)
                .await
                .map_err(AppError::DatabaseError)?
                .map(|channel| channel.server_id),
            None => None,
        },
    };

    let can_publish = match channel_id {
        Some(channel_id) => {
            let level = state
                .db
                .get_channel_permission_level(channel_id, claims.user_id)
                .await
                .map_err(AppError::DatabaseError)?
                .unwrap_or(0);
            level >= CHANNEL_PERMISSION_WRITE
        }
        None => true,
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

    let participant_identity = claims.user_id.to_string();
    let token = mint_livekit_token(
        &livekit_api_key,
        &livekit_api_secret,
        &req.room,
        &participant_identity,
        &claims.username,
        can_publish,
        true,
    )?;

    info!("Token LiveKit generat amb èxit per a room={}", req.room);
    Ok(Json(LiveKitTokenResponse {
        token,
        url: livekit_host,
    }))
}

/// Genera un JWT de LiveKit signat amb l'API secret per a un participant donat.
///
/// Compartit entre l'endpoint d'usuari i l'assistent de canal (server-integrat),
/// que necessita encunyar el seu propi token amb `can_publish: false`.
pub fn mint_livekit_token(
    api_key: &str,
    api_secret: &str,
    room: &str,
    identity: &str,
    name: &str,
    can_publish: bool,
    can_subscribe: bool,
) -> Result<String, AppError> {
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

    let claims = LiveKitClaims {
        iss: api_key,
        // Per LiveKit, sub ha de ser la identitat del participant, no l'API secret.
        sub: identity,
        exp: expiration,
        nbf: now.timestamp(),
        iat: now.timestamp(),
        name,
        video: VideoGrant {
            room,
            room_join: true,
            can_publish,
            can_subscribe,
        },
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(api_secret.as_bytes()),
    )
    .map_err(|e| {
        info!("Error generant token LiveKit: {}", e);
        AppError::LiveKitTokenError
    })
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/livekit/token", post(generate_token))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, LogLevel},
        db::connect_db,
        middleware::auth::{AuthClaims, UserPresenceState},
    };
    use axum::Extension;
    use std::{collections::{HashMap, HashSet}, sync::Arc};
    use tokio::sync::RwLock;

    async fn make_state() -> crate::middleware::AppState {
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
            livekit_api_secret: "test-secret-32-bytes-padding-xx".to_string(),
            jwt_secret: "test-secret".to_string(),
            jwt_expiration_days: 7,
            backend_debug: LogLevel::Info,
            server_master_key: [9u8; 32],
            static_dir: None,
            max_file_size_bytes: 0,
            allowed_origins: vec![],
            assistant_openai_base_url: "https://api.openai.com/v1".to_string(),
            assistant_openai_api_key: None,
            assistant_stt_model: "whisper-1".to_string(),
            assistant_summary_model: "gpt-4o-mini".to_string(),
            assistant_language: None,
        };
        let db = connect_db(&config).await.expect("sqlite test db");
        let (_layer, io) = socketioxide::SocketIo::new_layer();
        crate::middleware::AppState {
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
    async fn livekit_token_blocked_when_streaming_quota_exhausted() {
        let state = make_state().await;
        let user_id = state
            .db
            .create_user("lk_quota_user", "hash")
            .await
            .expect("create user");

        let year_month = Utc::now().format("%Y-%m").to_string();

        // Free plan: 10 h = 36_000 s. Pre-fill to the limit.
        state
            .db
            .increment_streaming_seconds(user_id, &year_month, 36_000)
            .await
            .expect("pre-fill streaming usage");

        let result = generate_token(
            State(state),
            Extension(claims_for(user_id, "lk_quota_user")),
            Json(LiveKitTokenRequest {
                room: "test-room".to_string(),
                participant: None,
                server_id: None,
            }),
        )
        .await;

        assert!(
            matches!(result, Err(AppError::StreamingQuotaExceeded)),
            "token should be blocked when streaming quota is exhausted"
        );
    }

    #[tokio::test]
    async fn livekit_token_allowed_when_quota_not_exhausted() {
        let state = make_state().await;
        let user_id = state
            .db
            .create_user("lk_ok_user", "hash")
            .await
            .expect("create user");

        let result = generate_token(
            State(state),
            Extension(claims_for(user_id, "lk_ok_user")),
            Json(LiveKitTokenRequest {
                room: "test-room".to_string(),
                participant: None,
                server_id: None,
            }),
        )
        .await;

        assert!(
            !matches!(result, Err(AppError::StreamingQuotaExceeded)),
            "token should not be blocked when quota is available"
        );
    }

    #[tokio::test]
    async fn livekit_token_enterprise_unlimited_never_blocked() {
        let state = make_state().await;
        let user_id = state
            .db
            .create_user("lk_enterprise_user", "hash")
            .await
            .expect("create user");
        state
            .db
            .set_user_plan_by_name(user_id, "enterprise")
            .await
            .expect("set enterprise plan");

        let year_month = Utc::now().format("%Y-%m").to_string();
        state
            .db
            .increment_streaming_seconds(user_id, &year_month, i64::MAX / 2)
            .await
            .expect("pre-fill enormous usage");

        let result = generate_token(
            State(state),
            Extension(claims_for(user_id, "lk_enterprise_user")),
            Json(LiveKitTokenRequest {
                room: "test-room".to_string(),
                participant: None,
                server_id: None,
            }),
        )
        .await;

        assert!(
            !matches!(result, Err(AppError::StreamingQuotaExceeded)),
            "enterprise plan should never block streaming tokens"
        );
    }

    #[tokio::test]
    async fn streaming_usage_accumulates_per_token() {
        let state = make_state().await;
        let user_id = state
            .db
            .create_user("lk_accum_user", "hash")
            .await
            .expect("create user");

        let year_month = Utc::now().format("%Y-%m").to_string();
        let before = state
            .db
            .get_user_streaming_usage(user_id, &year_month)
            .await
            .expect("usage before");

        let _ = generate_token(
            State(state.clone()),
            Extension(claims_for(user_id, "lk_accum_user")),
            Json(LiveKitTokenRequest {
                room: "room-a".to_string(),
                participant: None,
                server_id: None,
            }),
        )
        .await;

        let after = state
            .db
            .get_user_streaming_usage(user_id, &year_month)
            .await
            .expect("usage after");

        assert_eq!(after, before + 3600, "each token should add 3600s of usage");
    }
}
