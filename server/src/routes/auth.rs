//! Endpoints d'autenticació: register, login.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing::{post},
    Router,
};
use shared::types::{AuthResponse, RefreshResponse};
use uuid::Uuid;
use crate::{
    config::Config,
    crypto::{hash, kyber},
    error::AppError,
    middleware::{AppState, AuthClaims, generate_token, generate_claims},
};
use shared::constants::{MIN_USERNAME_LENGTH, MAX_USERNAME_LENGTH, MIN_PASSWORD_LENGTH, MAX_PASSWORD_LENGTH};
use serde::{Deserialize, Serialize};

// ── Requests ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

// ── Register ─────────────────────────────────────────────────────

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    // Validar username
    if req.username.len() < MIN_USERNAME_LENGTH || req.username.len() > MAX_USERNAME_LENGTH {
        return Err(AppError::UsernameExists);
    }
    if !req.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(AppError::UsernameExists);
    }

    // Validar password
    if req.password.len() < MIN_PASSWORD_LENGTH {
        return Err(AppError::WeakPassword { min: MIN_PASSWORD_LENGTH });
    }

    // Comprovar si l'usuari ja existeix (simplificat — en producció faríem query real)
    // TODO: verificar a DB

    // Generar hash de password
    let password_hash = hash::hash_password(&req.password)?;

    // Generar keypair Kyber-1024 per al dispositiu (placeholder)
    let _public_key = kyber::generate_keypair_placeholder();
    let _password_hash = password_hash;

    // TODO: INSERT a DB aquí
    // let user_id = ...
    // let device_id = ...

    // Per ara, generar IDs de test
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    // Generar token JWT
    let claims = generate_claims(user_id, &req.username, device_id, false, &state.config);
    let token = generate_token(&claims, &state.config)?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            user_id,
            username: req.username,
            token,
            device_id,
            device_label: Some("Generated Device".to_string()),
            is_admin: false,
        }),
    ))
}

// ── Login ────────────────────────────────────────────────────────

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    // TODO: Buscar usuari a DB, verificar password amb hash::verify_password

    // Per ara, generar resposta de test
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    let claims = generate_claims(user_id, &req.username, device_id, false, &state.config);
    let token = generate_token(&claims, &state.config)?;

    Ok((
        StatusCode::OK,
        Json(AuthResponse {
            user_id,
            username: req.username,
            token,
            device_id,
            device_label: Some("Login Device".to_string()),
            is_admin: false,
        }),
    ))
}

// ── Refresh ──────────────────────────────────────────────────────

pub async fn refresh(
    State(state): State<AppState>,
) -> Result<Json<RefreshResponse>, AppError> {
    // TODO: Implementar refresh token amb cookie HttpOnly
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();

    let claims = generate_claims(user_id, "user", device_id, false, &state.config);
    let token = generate_token(&claims, &state.config)?;

    Ok(Json(RefreshResponse { token }))
}

// ── Router ───────────────────────────────────────────────────────

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        .route("/api/auth/refresh", post(refresh))
        .with_state(state)
}