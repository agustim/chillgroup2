//! Endpoints d'autenticació: register, login.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing,
    Router,
};
use shared::types::{AuthResponse, RefreshResponse};
use serde::Deserialize;
use uuid::Uuid;
use tracing::{info, error, warn};

use crate::{
    crypto::hash,
    error::AppError,
    middleware::{AppState, generate_token, generate_claims},
};
use shared::constants::{MIN_USERNAME_LENGTH, MAX_USERNAME_LENGTH, MIN_PASSWORD_LENGTH};

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

#[axum::debug_handler]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    info!("📝 Endpoint de register cridat per username: {}", req.username);

    // Validar username
    if req.username.len() < MIN_USERNAME_LENGTH || req.username.len() > MAX_USERNAME_LENGTH {
        error!("❌ Register fallat: username amb longitud invàlida ({})", req.username);
        return Err(AppError::UsernameExists);
    }
    if !req.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        error!("❌ Register fallat: username amb caràcters invàlids");
        return Err(AppError::UsernameExists);
    }

    // Validar password
    if req.password.len() < MIN_PASSWORD_LENGTH {
        error!("❌ Register fallat: password massa curt ({} < {})", req.password.len(), MIN_PASSWORD_LENGTH);
        return Err(AppError::WeakPassword { min: MIN_PASSWORD_LENGTH });
    }

    // Comprovar si l'usuari ja existeix
    let exists = state.db.user_exists(&req.username).await;
    match exists {
        Ok(true) => {
            error!("❌ Register fallat: usuari ja existeix: {}", req.username);
            return Err(AppError::UsernameExists);
        }
        Err(e) => {
            error!("❌ Error consultant DB: {}", e);
            return Err(AppError::DatabaseUnavailable);
        }
        Ok(false) => {}
    }

    info!("✅ Usuari no existeix, creant nou usuari: {}", req.username);

    // Generar hash de password
    let password_hash = match hash::hash_password(&req.password) {
        Ok(h) => h,
        Err(e) => {
            error!("❌ Error generant hash: {}", e);
            return Err(AppError::InternalError);
        }
    };
    info!("✅ Password hash generat");

    // Crear usuari a DB
    let user_id = match state.db.create_user(&req.username, &password_hash).await {
        Ok(id) => id,
        Err(e) => {
            error!("❌ Error creant usuari a DB: {}", e);
            return Err(AppError::InternalError);
        }
    };
    info!("✅ Usuari creat a DB amb user_id={}", user_id);

    // Crear dispositiu persistent per a l'usuari
    let device_label = format!("Dispositiu principal");
    let device_id = match state.db.upsert_device_for_user(user_id, &device_label).await {
        Ok(id) => id,
        Err(e) => {
            error!("❌ Error creant dispositiu a DB: {}", e);
            return Err(AppError::InternalError);
        }
    };

    // Generar token JWT
    let claims = generate_claims(user_id, &req.username, device_id, false, &state.config);
    let token = match generate_token(&claims, &state.config) {
        Ok(t) => t,
        Err(e) => {
            error!("❌ Error generant token: {}", e);
            return Err(AppError::InternalError);
        }
    };
    info!("✅ Token JWT generat");

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            user_id,
            username: req.username,
            token,
            device_id,
            device_label: Some("Dispositiu principal".to_string()),
            is_admin: false,
        }),
    ))
}

// ── Login ────────────────────────────────────────────────────────

#[axum::debug_handler]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    info!("🔑 Endpoint de login cridat per username: {}", req.username);

    // Buscar usuari a DB
    let user = state.db.find_user_by_username(&req.username).await;
    match user {
        Ok(Some((_user_id, _username, password_hash))) => {
            info!("✅ Usuari trobat a DB, verificant password...");

            // Verificar password amb hash
            let is_valid = match hash::verify_password(&req.password, &password_hash) {
                Ok(v) => v,
                Err(e) => {
                    error!("❌ Error verificant password: {}", e);
                    return Err(AppError::InternalError);
                }
            };

            if !is_valid {
                warn!("❌ Login fallat: password incorrecte per username: {}", req.username);
                return Err(AppError::UnauthorizedCredentials);
            }

            info!("✅ Password correcte, generant token...");

            let user_id = _user_id;

            // Recuperar o crear dispositiu persistent per a l'usuari
            let device_id = match state.db.upsert_device_for_user(user_id, "Dispositiu principal").await {
                Ok(id) => id,
                Err(e) => {
                    error!("❌ Error obtenint dispositiu a DB: {}", e);
                    return Err(AppError::InternalError);
                }
            };

            // Generar token JWT
            let claims = generate_claims(user_id, &_username, device_id, false, &state.config);
            let token = match generate_token(&claims, &state.config) {
                Ok(t) => t,
                Err(e) => {
                    error!("❌ Error generant token: {}", e);
                    return Err(AppError::InternalError);
                }
            };

            info!("✅ Login exitós: user_id={}, device_id={}, username={}", user_id, device_id, _username);

            Ok((
                StatusCode::OK,
                Json(AuthResponse {
                    user_id,
                    username: _username,
                    token,
                    device_id,
                    device_label: Some("Dispositiu principal".to_string()),
                    is_admin: false,
                }),
            ))
        }
        Ok(None) => {
            warn!("❌ Login fallat: usuari no trobat: {}", req.username);
            Err(AppError::UnauthorizedCredentials)
        }
        Err(e) => {
            error!("❌ Error consultant DB: {}", e);
            Err(AppError::DatabaseUnavailable)
        }
    }
}

// ── Refresh ──────────────────────────────────────────────────────

#[axum::debug_handler]
pub async fn refresh(
    State(_state): State<AppState>,
) -> Result<Json<RefreshResponse>, AppError> {
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();
    let claims = generate_claims(user_id, "user", device_id, false, &_state.config);
    let token = generate_token(&claims, &_state.config)?;
    Ok(Json(RefreshResponse { token }))
}

// ── Router ───────────────────────────────────────────────────────

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/register", routing::post(register))
        .route("/api/auth/login", routing::post(login))
        .route("/api/auth/refresh", routing::post(refresh))
        .with_state(state)
}