//! Middleware d'autenticació JWT.

#![allow(dead_code)]

use axum::{
    http::Request,
    middleware::Next,
    response::Response,
    body::Body,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;
use crate::config::Config;
use crate::error::AppError;
use tracing::{info, warn};

// ── Claims JWT ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthClaims {
    pub user_id: Uuid,
    pub username: String,
    pub device_id: Uuid,
    pub is_admin: bool,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
}

// ── Estat de l'aplicació compartit ──────────────────────────────

use crate::db::DatabasePool;
use socketioxide::SocketIo;

#[derive(Debug, Default)]
pub struct UserPresenceState {
    pub online_sockets: HashMap<Uuid, HashSet<String>>,
}

/// Tracks recent LiveKit token issuances to prevent duplicate quota charges.
/// Key: (user_id, room_name). Value: Instant of last token issued.
pub type LiveKitTokenCache = Arc<Mutex<HashMap<(Uuid, String), Instant>>>;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabasePool,
    pub config: Config,
    pub io: SocketIo,
    pub user_presence: Arc<RwLock<UserPresenceState>>,
    pub livekit_token_cache: LiveKitTokenCache,
}

// ── Generació de tokens ─────────────────────────────────────────

pub fn generate_token(claims: &AuthClaims, config: &Config) -> Result<String, AppError> {
    let header = Header::default();
    let secret = config.jwt_secret.as_bytes();
    encode(&header, claims, &EncodingKey::from_secret(secret))
        .map_err(|_| AppError::InternalError)
}

pub fn generate_claims(
    user_id: Uuid,
    username: &str,
    device_id: Uuid,
    is_admin: bool,
    config: &Config,
) -> AuthClaims {
    let now = Utc::now();
    let expiration = (now + Duration::days(config.jwt_expiration_days as i64))
        .timestamp();
    info!("Claims generats per user_id={}, username={}", user_id, username);
    AuthClaims {
        user_id,
        username: username.to_string(),
        device_id,
        is_admin,
        exp: expiration,
        iat: now.timestamp(),
        jti: Uuid::new_v4().to_string(),
    }
}

// ── Extracció i validació de claims ─────────────────────────────

pub async fn extract_claims(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::TokenMissing)?;

    let parts: Vec<&str> = auth_header.split_whitespace().collect();
    if parts.len() != 2 || parts[0] != "Bearer" {
        warn!("Token missing o format incorrecte");
        return Err(AppError::TokenMissing);
    }

    let token_str = parts[1];

    let app_state = req.extensions().get::<AppState>().cloned();

    let claims = if let Some(ref app_state) = app_state {
        let secret = app_state.config.jwt_secret.as_bytes();
        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.leeway = 5;

        decode::<AuthClaims>(
            token_str,
            &DecodingKey::from_secret(secret),
            &validation,
        ).map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                warn!("Token JWT expirat");
                AppError::TokenExpired
            },
            jsonwebtoken::errors::ErrorKind::InvalidSignature => {
                warn!("Token JWT invàlid");
                AppError::TokenInvalid
            },
            _ => {
                warn!("Error verificant token JWT: {:?}", e);
                AppError::TokenInvalid
            },
        })?.claims
    } else {
        warn!("No s'ha trobat AppState, utilitzant JWT_SECRET de variable d'entorn");
        let secret = std::env::var("JWT_SECRET").map_err(|_| AppError::TokenMissing)?;
        let secret_bytes = secret.as_bytes();
        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.leeway = 5;

        decode::<AuthClaims>(
            token_str,
            &DecodingKey::from_secret(secret_bytes),
            &validation,
        ).map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
            jsonwebtoken::errors::ErrorKind::InvalidSignature => AppError::TokenInvalid,
            _ => AppError::TokenInvalid,
        })?.claims
    };

    if let Some(app_state) = app_state {
        let active = app_state.db
            .is_device_active(claims.device_id, claims.user_id)
            .await
            .map_err(|_| AppError::InternalError)?;
        if !active {
            warn!("Device {} revocat o inexistent per user_id={}", claims.device_id, claims.user_id);
            return Err(AppError::TokenInvalid);
        }
    }

    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use crate::config::LogLevel;

    fn test_config() -> Config {
        env::set_var("JWT_SECRET", "test-secret-key-for-unit-tests-only");
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
            jwt_secret: "test-secret-key-for-unit-tests-only".to_string(),
            jwt_expiration_days: 7,
            backend_debug: LogLevel::Info,
            server_master_key: [7u8; 32],
            static_dir: None,
            max_file_size_bytes: 100 * 1024 * 1024,
            allowed_origins: vec![],
            assistant_openai_base_url: "https://api.openai.com/v1".to_string(),
            assistant_openai_api_key: None,
            assistant_stt_model: "whisper-1".to_string(),
            assistant_summary_model: "gpt-4o-mini".to_string(),
            assistant_language: None,
        }
    }

    #[test]
    fn test_generate_claims_creates_valid_claims() {
        let config = test_config();
        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();
        let username = "testuser".to_string();

        let claims = generate_claims(user_id, &username, device_id, false, &config);

        assert_eq!(claims.user_id, user_id);
        assert_eq!(claims.username, username);
        assert_eq!(claims.device_id, device_id);
        assert!(!claims.is_admin);
        assert!(claims.exp > claims.iat);
    }

    #[test]
    fn test_generate_claims_admin() {
        let config = test_config();
        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();

        let claims = generate_claims(user_id, "admin", device_id, true, &config);

        assert!(claims.is_admin);
    }

    #[test]
    fn test_generate_and_decode_token() {
        let config = test_config();
        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();

        let claims = generate_claims(user_id, "testuser", device_id, false, &config);
        let token = generate_token(&claims, &config).expect("Ha de generar token");

        assert!(!token.is_empty());
        assert!(token.starts_with("eyJ")); // JWT base64 header
    }

    #[test]
    fn test_token_contains_user_id() {
        let config = test_config();
        let user_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();

        let claims = generate_claims(user_id, "testuser", device_id, false, &config);
        let token = generate_token(&claims, &config).expect("Ha de generar token");

        // Decodificar el token per verificar que conté el user_id
        let decoded = decode::<AuthClaims>(
            &token,
            &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
            &Validation::new(jsonwebtoken::Algorithm::HS256),
        ).expect("Ha de decodificar");

        assert_eq!(decoded.claims.user_id, user_id);
        assert_eq!(decoded.claims.username, "testuser");
    }

    #[test]
    fn test_token_has_valid_structure() {
        let config = test_config();
        let claims = generate_claims(Uuid::new_v4(), "user", Uuid::new_v4(), false, &config);
        let token = generate_token(&claims, &config).expect("Ha de generar token");

        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3); // header.payload.signature
    }

    #[test]
    fn test_claims_serialization() {
        let config = test_config();
        let claims = generate_claims(Uuid::new_v4(), "user", Uuid::new_v4(), false, &config);

        let json = serde_json::to_string(&claims).expect("Ha de serialitzar");
        assert!(json.contains("user_id"));
        assert!(json.contains("username"));
        assert!(json.contains("user"));
    }

    #[test]
    fn test_claims_deserialization() {
        let config = test_config();
        let original = generate_claims(Uuid::new_v4(), "user", Uuid::new_v4(), false, &config);

        let json = serde_json::to_string(&original).expect("Ha de serialitzar");
        let deserialized: AuthClaims = serde_json::from_str(&json).expect("Ha de deserialitzar");

        assert_eq!(deserialized.user_id, original.user_id);
        assert_eq!(deserialized.username, original.username);
    }

    #[test]
    fn test_different_users_different_tokens() {
        let config = test_config();
        let claims1 = generate_claims(Uuid::new_v4(), "user1", Uuid::new_v4(), false, &config);
        let claims2 = generate_claims(Uuid::new_v4(), "user2", Uuid::new_v4(), false, &config);

        let token1 = generate_token(&claims1, &config).expect("Ha de generar token");
        let token2 = generate_token(&claims2, &config).expect("Ha de generar token");

        assert_ne!(token1, token2);
    }
}