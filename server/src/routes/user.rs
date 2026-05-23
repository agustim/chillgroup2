//! Rutes d'usuari — `/api/user/me`
//!
//! Retorna la informació de l'usuari autenticat extreta del JWT.

use axum::{
    extract::Path,
    extract::State,
    Json,
    Router,
    http::StatusCode,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ml_kem::ml_kem_1024;
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

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
    pub kem_public_key: String,
    pub dsa_public_key: String,
}

fn is_valid_kem_public_key_b64(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    let decoded = match STANDARD.decode(trimmed) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let key: Result<ml_kem::kem::Key<ml_kem_1024::EncapsulationKey>, _> = decoded.as_slice().try_into();
    key.is_ok()
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDeviceResponse {
    pub device_id: Uuid,
    pub label: String,
    pub kem_public_key: String,
    pub dsa_public_key: String,
    pub has_kem_public_key: bool,
    pub has_dsa_public_key: bool,
    pub created_at: String,
    pub last_seen: String,
    pub revoked: bool,
    pub is_current: bool,
}

/// Registrar/actualitzar la clau pública ML-KEM del dispositiu actual.
#[axum::debug_handler]
pub async fn update_device_public_key(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<UpdateDevicePublicKeyRequest>,
) -> Result<StatusCode, AppError> {
    info!("🔑 Actualitzant claus públiques del dispositiu {} per l'usuari {}", claims.device_id, claims.user_id);

    if req.kem_public_key.trim().is_empty() || req.dsa_public_key.trim().is_empty() {
        return Err(AppError::BadRequest);
    }

    if !is_valid_kem_public_key_b64(&req.kem_public_key) {
        return Err(AppError::BadRequest);
    }

    state.db
        .update_device_public_keys(
            claims.device_id,
            claims.user_id,
            req.kem_public_key.trim(),
            req.dsa_public_key.trim(),
        )
        .await
        .map_err(AppError::DatabaseError)?;

    info!("✅ Clau pública actualitzada per device_id={}", claims.device_id);
    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn list_my_devices(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<serde_json::Value>, AppError> {
    let devices = state
        .db
        .list_devices_for_user(claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let data: Vec<UserDeviceResponse> = devices
        .into_iter()
        .map(|(device_id, label, kem_public_key, dsa_public_key, created_at, last_seen, revoked)| UserDeviceResponse {
            device_id,
            label: label.unwrap_or_else(|| "Dispositiu".to_string()),
            has_kem_public_key: !kem_public_key.trim().is_empty(),
            has_dsa_public_key: !dsa_public_key.trim().is_empty(),
            kem_public_key,
            dsa_public_key,
            created_at,
            last_seen,
            revoked,
            is_current: device_id == claims.device_id,
        })
        .collect();

    Ok(Json(serde_json::json!({
        "success": true,
        "data": data,
    })))
}

#[axum::debug_handler]
pub async fn revoke_my_device(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(device_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    if device_id == claims.device_id {
        return Err(AppError::Forbidden);
    }

    let updated = state
        .db
        .revoke_device_for_user(device_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    if !updated {
        return Err(AppError::UserNotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Router per a rutes d'usuari
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/user/me", axum::routing::get(get_user_me))
        .route("/api/user/me/devices", axum::routing::get(list_my_devices))
        .route("/api/user/me/devices/{device_id}", axum::routing::delete(revoke_my_device))
        .route("/api/user/me/device/publickey", axum::routing::put(update_device_public_key))
        .with_state(state)
}