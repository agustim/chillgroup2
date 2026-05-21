//! Rutes d'usuari — `/api/user/me`
//!
//! Retorna la informació de l'usuari autenticat extreta del JWT.

use axum::{
    extract::State,
    Json,
    Router,
    http::StatusCode,
};
use serde::Deserialize;
use tracing::info;

use crate::{
    middleware::{AppState, AuthClaims},
    error::AppError,
};

/// Obtenir informació de l'usuari autenticat
///
/// Extreu user_id i username del JWT i verifica que existeix a la DB.
#[axum::debug_handler]
pub async fn get_user_me(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<serde_json::Value>, AppError> {
    info!("📋 Endpoint /api/user/me cridat per user_id={}, username={}", claims.user_id, claims.username);

    // Verificar que l'usuari existeix a la DB
    let user_exists = match state.db.user_exists(&claims.username).await {
        Ok(exists) => exists,
        Err(e) => {
            tracing::error!("❌ Error verificant usuari a DB: {}", e);
            return Err(AppError::DatabaseUnavailable);
        }
    };

    if !user_exists {
        tracing::warn!("⚠️ Usuari {} del token JWT no trobat a la DB", claims.username);
        return Err(AppError::UserNotFound);
    }

    info!("✅ Usuari verificat correctament a la DB");

    // Retornar informació de l'usuari
    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "userId": claims.user_id.to_string(),
            "username": claims.username,
            "isAdmin": claims.is_admin,
            "deviceId": claims.device_id.to_string(),
        }
    })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateDevicePublicKeyRequest {
    pub public_key: String,
}

/// Registrar/actualitzar la clau pública ML-KEM del dispositiu actual.
#[axum::debug_handler]
pub async fn update_device_public_key(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<UpdateDevicePublicKeyRequest>,
) -> Result<StatusCode, AppError> {
    info!("🔑 Actualitzant clau pública del dispositiu {} per l'usuari {}", claims.device_id, claims.user_id);

    if req.public_key.is_empty() {
        return Err(AppError::BadRequest);
    }

    state.db
        .update_device_public_key(claims.device_id, claims.user_id, &req.public_key)
        .await
        .map_err(AppError::DatabaseError)?;

    info!("✅ Clau pública actualitzada per device_id={}", claims.device_id);
    Ok(StatusCode::NO_CONTENT)
}

/// Router per a rutes d'usuari
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/user/me", axum::routing::get(get_user_me))
        .route("/api/user/me/device/publickey", axum::routing::put(update_device_public_key))
        .with_state(state)
}