//! Rutes d'usuari — `/api/user/me`
//!
//! Retorna la informació de l'usuari autenticat extreta del JWT.

use axum::{
    extract::Path,
    extract::Query,
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
use shared::constants::{MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH};

use crate::{
    crypto::hash,
    middleware::{AppState, AuthClaims},
    error::AppError,
};

#[derive(Debug, Deserialize)]
pub struct UserSearchQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSearchResult {
    pub user_id: Uuid,
    pub username: String,
    pub is_friend: bool,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

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

#[axum::debug_handler]
pub async fn search_users(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Query(query): Query<UserSearchQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let search = query.q.unwrap_or_default();
    let limit = query.limit.unwrap_or(20).clamp(1, 50);

    let results = state
        .db
        .search_users_for_user(claims.user_id, &search, limit)
        .await
        .map_err(AppError::DatabaseError)?;

    let presence = state.user_presence.read().await;
    let data: Vec<UserSearchResult> = results
        .into_iter()
        .map(|(user_id, username, is_friend)| UserSearchResult {
            user_id,
            username,
            is_friend,
            status: if presence.online_sockets.contains_key(&user_id) { "online".to_string() } else { "offline".to_string() },
        })
        .collect();

    Ok(Json(serde_json::json!({
        "success": true,
        "data": data,
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

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TierLimits {
    pub max_servers: i32,
    pub max_channels_text_per_server: i32,
    pub max_channels_voice_per_server: i32,
    pub max_members_per_server: i32,
    pub api_calls_per_minute: i32,
    pub messages_per_day: i32,
    pub max_storage_bytes: i64,
    pub max_transfer_bytes_monthly: i64,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPlanSummary {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub limits: TierLimits,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserUsageSummary {
    pub total_servers: i64,
    pub total_text_channels: i64,
    pub total_voice_channels: i64,
    pub total_members_across_servers: i64,
    pub messages_today: i64,
    pub api_calls_this_minute: i64,
    pub stored_bytes: i64,
    pub transfer_bytes_this_month: i64,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPermissionsSummary {
    pub can_create_server: bool,
    pub can_create_text_channel: bool,
    pub can_create_voice_channel: bool,
    pub can_add_members: bool,
    pub can_send_message: bool,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserRemainingSummary {
    pub servers: i64,
    pub text_channels: i64,
    pub voice_channels: i64,
    pub members: i64,
    pub messages_today: i64,
    pub api_calls_this_minute: i64,
}

#[axum::debug_handler]
pub async fn get_user_limits(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<serde_json::Value>, AppError> {
    let (
        plan_id,
        plan_name,
        display_name,
        description,
        max_servers,
        max_text_channels,
        max_voice_channels,
        max_members,
        api_calls_per_minute,
        messages_per_day,
        max_storage_bytes,
        max_transfer_bytes_monthly,
    ) = state
        .db
        .get_user_plan_limits(claims.user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let total_servers = state
        .db
        .count_owned_servers(claims.user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;
    let total_text_channels = state
        .db
        .count_owned_channels_by_type(claims.user_id, "text")
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;
    let total_voice_channels = state
        .db
        .count_owned_channels_by_type(claims.user_id, "voice")
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;
    let total_members_across_servers = state
        .db
        .count_members_in_owned_servers(claims.user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;
    let messages_today = state
        .db
        .count_user_messages_today(claims.user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    // TODO: quan integrem comptador de rate limit global per usuari,
    // aquest camp vindrà d'aquell magatzem temporal.
    let api_calls_this_minute = 0i64;

    let can_create_server = max_servers == -1 || total_servers < i64::from(max_servers);
    let can_create_text_channel = max_text_channels == -1 || total_text_channels < i64::from(max_text_channels);
    let can_create_voice_channel = max_voice_channels == -1 || total_voice_channels < i64::from(max_voice_channels);
    let can_add_members = max_members == -1 || total_members_across_servers < i64::from(max_members);
    let can_send_message = messages_per_day == -1 || messages_today < i64::from(messages_per_day);

    let remaining_servers = if max_servers == -1 {
        -1
    } else {
        (i64::from(max_servers) - total_servers).max(0)
    };
    let remaining_text_channels = if max_text_channels == -1 {
        -1
    } else {
        (i64::from(max_text_channels) - total_text_channels).max(0)
    };
    let remaining_voice_channels = if max_voice_channels == -1 {
        -1
    } else {
        (i64::from(max_voice_channels) - total_voice_channels).max(0)
    };
    let remaining_members = if max_members == -1 {
        -1
    } else {
        (i64::from(max_members) - total_members_across_servers).max(0)
    };
    let remaining_messages_today = if messages_per_day == -1 {
        -1
    } else {
        (i64::from(messages_per_day) - messages_today).max(0)
    };
    let remaining_api_calls_this_minute = if api_calls_per_minute == -1 {
        -1
    } else {
        (i64::from(api_calls_per_minute) - api_calls_this_minute).max(0)
    };

    let year_month = chrono::Utc::now().format("%Y-%m").to_string();
    let (stored_bytes, transfer_bytes_this_month) = state
        .db
        .get_user_storage_usage(claims.user_id, &year_month)
        .await
        .unwrap_or((0, 0));

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "plan": UserPlanSummary {
                id: plan_id,
                name: plan_name,
                display_name,
                description,
                limits: TierLimits {
                    max_servers,
                    max_channels_text_per_server: max_text_channels,
                    max_channels_voice_per_server: max_voice_channels,
                    max_members_per_server: max_members,
                    api_calls_per_minute,
                    messages_per_day,
                    max_storage_bytes,
                    max_transfer_bytes_monthly,
                }
            },
            "usage": UserUsageSummary {
                total_servers,
                total_text_channels,
                total_voice_channels,
                total_members_across_servers,
                messages_today,
                api_calls_this_minute,
                stored_bytes,
                transfer_bytes_this_month,
            },
            "permissions": UserPermissionsSummary {
                can_create_server,
                can_create_text_channel,
                can_create_voice_channel,
                can_add_members,
                can_send_message,
            },
            "remaining": UserRemainingSummary {
                servers: remaining_servers,
                text_channels: remaining_text_channels,
                voice_channels: remaining_voice_channels,
                members: remaining_members,
                messages_today: remaining_messages_today,
                api_calls_this_minute: remaining_api_calls_this_minute,
            }
        }
    })))
}

/// Registrar/actualitzar la clau pública ML-KEM del dispositiu actual.
#[axum::debug_handler]
pub async fn update_device_public_key(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<UpdateDevicePublicKeyRequest>,
) -> Result<StatusCode, AppError> {
    info!("🔑 Actualitzant claus públiques del dispositiu {} per l'usuari {}", claims.device_id, claims.user_id);

    if req.kem_public_key.trim().is_empty() {
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

#[axum::debug_handler]
pub async fn change_my_password(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<StatusCode, AppError> {
    let old_password = req.old_password.trim();
    let new_password = req.new_password.trim();

    if old_password.is_empty() || new_password.is_empty() {
        return Err(AppError::BadRequest);
    }

    if new_password.len() < MIN_PASSWORD_LENGTH || new_password.len() > MAX_PASSWORD_LENGTH {
        return Err(AppError::BadRequest);
    }

    if old_password == new_password {
        return Err(AppError::BadRequest);
    }

    let user = state
        .db
        .find_user_by_username(&claims.username)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let Some((user_id, _, current_password_hash)) = user else {
        return Err(AppError::UserNotFound);
    };

    if user_id != claims.user_id {
        return Err(AppError::Forbidden);
    }

    let old_password_valid = hash::verify_password(old_password, &current_password_hash)
        .map_err(|_| AppError::InternalError)?;

    if !old_password_valid {
        return Err(AppError::UnauthorizedCredentials);
    }

    let new_password_hash = hash::hash_password(new_password).map_err(|_| AppError::InternalError)?;

    let updated = state
        .db
        .update_user_password_hash_by_id(claims.user_id, &new_password_hash)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    if !updated {
        return Err(AppError::UserNotFound);
    }

    info!("✅ Password actualitzada per user_id={}", claims.user_id);

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, LogLevel},
        crypto::hash,
        db::connect_db,
        middleware::auth::UserPresenceState,
    };
    use axum::response::IntoResponse;
    use std::{collections::{HashMap, HashSet}, sync::Arc};
    use tokio::sync::RwLock;

    async fn make_state() -> AppState {
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
            livekit_api_secret: "test-secret".to_string(),
            jwt_secret: "test-secret".to_string(),
            jwt_expiration_days: 7,
            backend_debug: LogLevel::Info,
            server_master_key: [7u8; 32],
            static_dir: None,
            max_file_size_bytes: 100 * 1024 * 1024,
            allowed_origins: vec![],
        };
        let db = connect_db(&config).await.expect("sqlite test db");
        let (_layer, io) = socketioxide::SocketIo::new_layer();
        use socketioxide::extract::{Data, SocketRef};
        io.ns("/", |_socket: SocketRef, Data(_auth): Data<serde_json::Value>| async move {});
        AppState {
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
    async fn get_user_me_returns_user_info() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("me_user", "hash", "user").await.unwrap();

        let result = get_user_me(
            State(state),
            axum::Extension(claims_for(user_id, "me_user")),
        )
        .await
        .unwrap();

        let data = &result.0["data"];
        assert_eq!(data["username"], "me_user");
        assert_eq!(data["isAdmin"], false);
    }

    #[tokio::test]
    async fn get_user_me_returns_not_found_for_deleted_user() {
        let state = make_state().await;
        let ghost_id = Uuid::new_v4();

        let result = get_user_me(
            State(state),
            axum::Extension(claims_for(ghost_id, "ghost_user_xyz")),
        )
        .await;

        let err = result.expect_err("deleted user should return error");
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn search_users_returns_results() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("srch_owner", "hash", "user").await.unwrap();
        state.db.create_user_with_role("srch_target", "hash", "user").await.unwrap();

        let result = search_users(
            State(state),
            axum::Extension(claims_for(user_id, "srch_owner")),
            Query(UserSearchQuery { q: Some("srch_target".to_string()), limit: None }),
        )
        .await
        .unwrap();

        let results = result.0["data"].as_array().unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r["username"] == "srch_target"));
    }

    #[tokio::test]
    async fn change_my_password_succeeds() {
        let state = make_state().await;
        let password_hash = hash::hash_password("old-password-123").unwrap();
        let user_id = state.db.create_user_with_role("pw_user", &password_hash, "user").await.unwrap();

        let result = change_my_password(
            State(state),
            axum::Extension(claims_for(user_id, "pw_user")),
            Json(ChangePasswordRequest {
                old_password: "old-password-123".to_string(),
                new_password: "new-password-456".to_string(),
            }),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn change_my_password_fails_with_wrong_old_password() {
        let state = make_state().await;
        let password_hash = hash::hash_password("correct-password-123").unwrap();
        let user_id = state.db.create_user_with_role("pw_wrong", &password_hash, "user").await.unwrap();

        let result = change_my_password(
            State(state),
            axum::Extension(claims_for(user_id, "pw_wrong")),
            Json(ChangePasswordRequest {
                old_password: "wrong-password-000".to_string(),
                new_password: "new-password-456".to_string(),
            }),
        )
        .await;

        let err = result.expect_err("wrong old password should fail");
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}

/// Router per a rutes d'usuari
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/user/me", axum::routing::get(get_user_me))
        .route("/api/user/me/limits", axum::routing::get(get_user_limits))
    .route("/api/users/search", axum::routing::get(search_users))
        .route("/api/user/me/devices", axum::routing::get(list_my_devices))
        .route("/api/user/me/devices/{device_id}", axum::routing::delete(revoke_my_device))
        .route("/api/user/me/password", axum::routing::put(change_my_password))
        .route("/api/user/me/device/publickey", axum::routing::put(update_device_public_key))
        .with_state(state)
}
