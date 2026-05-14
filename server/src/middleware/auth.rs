//! Middleware d'autenticació JWT.

use axum::{
    http::{Request, HeaderValue},
    middleware::Next,
    response::Response,
    body::Body,
    Extension,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::Pool;
use sqlx::Postgres;
use uuid::Uuid;
use crate::config::Config;
use crate::error::AppError;

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

#[derive(Clone)]
pub struct AppState {
    pub db: Pool<Postgres>,
    pub config: Config,
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

/// Middleware que extreu els claims del JWT del header Authorization
/// i inyecta els claims a l'extensions de la request.
pub async fn extract_claims(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // Obtenir Authorization header
    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(AppError::TokenMissing)?;

    // Parsejar "Bearer <token>"
    let parts: Vec<&str> = auth_header.split_whitespace().collect();
    if parts.len() != 2 || parts[0] != "Bearer" {
        return Err(AppError::TokenMissing);
    }

    let token_str = parts[1];

    // Per obtenir el secret, necessitem AppState
    // Com que no tenim State aquí, farem una suposició:
    // El secret es pot passar com a env var o des d'AppState
    // Utilitzarem Extension<AppState> si està disponible

    let claims = if let Some(app_state) = req.extensions().get::<AppState>() {
        let secret = app_state.config.jwt_secret.as_bytes();
        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.leeway = 5;

        decode::<AuthClaims>(
            token_str,
            &DecodingKey::from_secret(secret),
            &validation,
        ).map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AppError::TokenExpired,
            jsonwebtoken::errors::ErrorKind::InvalidSignature => AppError::TokenInvalid,
            _ => AppError::TokenInvalid,
        })?.claims
    } else {
        // Fallback: utilitzar JWT_SECRET com a env var
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

    // Insertar claims a l'extensions de la request
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}