//! Endpoints d'administració.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    crypto::hash,
    error::AppError,
    middleware::{AppState, AuthClaims},
};
use shared::constants::MIN_PASSWORD_LENGTH;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserItem {
    pub user_id: Uuid,
    pub username: String,
    pub role: String,
    pub plan_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAdminUserRequest {
    pub username: String,
    pub password: String,
    pub role: Option<String>,
    pub plan_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminTierLimits {
    pub max_servers: i32,
    pub max_channels_text_per_server: i32,
    pub max_channels_voice_per_server: i32,
    pub max_members_per_server: i32,
    pub api_calls_per_minute: i32,
    pub messages_per_day: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminTierUsage {
    pub total_servers: i64,
    pub total_text_channels: i64,
    pub total_voice_channels: i64,
    pub total_members_across_servers: i64,
    pub messages_today: i64,
    pub api_calls_this_minute: i64,
}

fn parse_role(role: Option<String>) -> Result<&'static str, AppError> {
    match role
        .unwrap_or_else(|| "user".to_string())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "user" => Ok("user"),
        "admin" => Ok("admin"),
        _ => Err(AppError::BadRequest),
    }
}

pub async fn list_users(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let users = state
        .db
        .list_all_users_admin()
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let data: Vec<AdminUserItem> = users
        .into_iter()
        .map(|(user_id, username, role, plan_id)| AdminUserItem {
            user_id,
            username,
            role,
            plan_id,
        })
        .collect();

    Ok(Json(serde_json::json!({
        "success": true,
        "data": data,
    })))
}

pub async fn update_user_plan(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path((user_id, plan_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let plan_exists = state
        .db
        .plan_exists_by_id(plan_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    if !plan_exists {
        return Err(AppError::BadRequest);
    }

    let updated = state
        .db
        .set_user_plan_by_id(user_id, plan_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    if !updated {
        return Err(AppError::UserNotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn create_user(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<CreateAdminUserRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    if req.username.trim().is_empty() || req.password.len() < MIN_PASSWORD_LENGTH {
        return Err(AppError::BadRequest);
    }

    let role = parse_role(req.role)?;

    let exists = state
        .db
        .user_exists(&req.username)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;
    if exists {
        return Err(AppError::UsernameExists);
    }

    let password_hash = hash::hash_password(&req.password).map_err(|_| AppError::InternalError)?;
    let user_id = state
        .db
        .create_user_with_role(&req.username, &password_hash, role)
        .await
        .map_err(|_| AppError::InternalError)?;

    if let Some(plan_id) = req.plan_id {
        let plan_exists = state
            .db
            .plan_exists_by_id(plan_id)
            .await
            .map_err(|_| AppError::DatabaseUnavailable)?;
        if !plan_exists {
            return Err(AppError::BadRequest);
        }

        state
            .db
            .set_user_plan_by_id(user_id, plan_id)
            .await
            .map_err(|_| AppError::DatabaseUnavailable)?;
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": {
                "userId": user_id,
                "username": req.username,
                "role": role,
            }
        })),
    ))
}

pub async fn get_user_limits(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

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
    ) = state
        .db
        .get_user_plan_limits(user_id)
        .await
        .map_err(|_| AppError::UserNotFound)?;

    let total_servers = state
        .db
        .count_owned_servers(user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;
    let total_text_channels = state
        .db
        .count_owned_channels_by_type(user_id, "text")
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;
    let total_voice_channels = state
        .db
        .count_owned_channels_by_type(user_id, "voice")
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;
    let total_members_across_servers = state
        .db
        .count_members_in_owned_servers(user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;
    let messages_today = state
        .db
        .count_user_messages_today(user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let api_calls_this_minute = 0i64;

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

    Ok(Json(serde_json::json!({
        "success": true,
        "data": {
            "userId": user_id,
            "plan": {
                "id": plan_id,
                "name": plan_name,
                "displayName": display_name,
                "description": description,
                "limits": AdminTierLimits {
                    max_servers,
                    max_channels_text_per_server: max_text_channels,
                    max_channels_voice_per_server: max_voice_channels,
                    max_members_per_server: max_members,
                    api_calls_per_minute,
                    messages_per_day,
                }
            },
            "usage": AdminTierUsage {
                total_servers,
                total_text_channels,
                total_voice_channels,
                total_members_across_servers,
                messages_today,
                api_calls_this_minute,
            },
            "remaining": {
                "servers": remaining_servers,
                "textChannels": remaining_text_channels,
                "voiceChannels": remaining_voice_channels,
                "members": remaining_members,
                "messagesToday": remaining_messages_today,
                "apiCallsThisMinute": remaining_api_calls_this_minute,
            }
        }
    })))
}

pub async fn update_user_role(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path((user_id, role)): Path<(Uuid, String)>,
) -> Result<StatusCode, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let normalized_role = parse_role(Some(role))?;
    let updated = state
        .db
        .update_user_role_by_id(user_id, normalized_role)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    if !updated {
        return Err(AppError::UserNotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_user(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    if claims.user_id == user_id {
        return Err(AppError::BadRequest);
    }

    let deleted = state
        .db
        .delete_user_by_id(user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    if !deleted {
        return Err(AppError::UserNotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/admin/users", get(list_users).post(create_user))
        .route("/api/admin/users/{user_id}", delete(delete_user))
        .route("/api/admin/users/{user_id}/limits", get(get_user_limits))
        .route("/api/admin/users/{user_id}/role/{role}", put(update_user_role))
        .route(
            "/api/admin/users/{user_id}/plan/{plan_id}",
            put(update_user_plan),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, LogLevel},
        db::connect_db,
        middleware::auth::UserPresenceState,
    };
    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    };
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
        };

        let db = connect_db(&config)
            .await
            .expect("sqlite test db should initialize");
        let (_layer, io) = socketioxide::SocketIo::new_layer();

        AppState {
            db,
            config,
            io,
            user_presence: Arc::new(RwLock::new(UserPresenceState {
                online_sockets: HashMap::<Uuid, HashSet<String>>::new(),
            })),
        }
    }

    fn claims_for(user_id: Uuid, username: &str, is_admin: bool) -> AuthClaims {
        AuthClaims {
            user_id,
            username: username.to_string(),
            device_id: Uuid::new_v4(),
            is_admin,
            exp: 0,
            iat: 0,
            jti: Uuid::new_v4().to_string(),
        }
    }

    #[tokio::test]
    async fn list_users_requires_admin() {
        let state = make_state().await;
        let user_id = state
            .db
            .create_user_with_role("normal_user", "hash", "user")
            .await
            .expect("user creation should work");

        let result = list_users(
            State(state),
            axum::Extension(claims_for(user_id, "normal_user", false)),
        )
        .await;

        assert!(matches!(result, Err(AppError::Forbidden)));
    }

    #[tokio::test]
    async fn admin_can_change_user_plan_by_id() {
        let state = make_state().await;
        let admin_id = state
            .db
            .create_user_with_role("admin_user", "hash", "admin")
            .await
            .expect("admin creation should work");
        let target_user_id = state
            .db
            .create_user_with_role("target_user", "hash", "user")
            .await
            .expect("target user creation should work");

        let pro_plan_id = state
            .db
            .get_plan_id_by_name("pro")
            .await
            .expect("query plans should work")
            .expect("pro plan should exist");

        let status = update_user_plan(
            State(state.clone()),
            axum::Extension(claims_for(admin_id, "admin_user", true)),
            Path((target_user_id, pro_plan_id)),
        )
        .await
        .expect("admin plan change should succeed");

        assert_eq!(status, StatusCode::NO_CONTENT);

        let max_servers = state
            .db
            .get_user_max_servers(target_user_id)
            .await
            .expect("read user limits should work");
        assert_eq!(max_servers, 5);
    }

    #[tokio::test]
    async fn admin_can_create_user_with_role() {
        let state = make_state().await;
        let admin_id = state
            .db
            .create_user_with_role("admin_creator", "hash", "admin")
            .await
            .expect("admin creation should work");

        let response = create_user(
            State(state.clone()),
            axum::Extension(claims_for(admin_id, "admin_creator", true)),
            Json(CreateAdminUserRequest {
                username: "created_user".to_string(),
                password: "password123".to_string(),
                role: Some("admin".to_string()),
                plan_id: None,
            }),
        )
        .await
        .expect("admin should create user");

        assert_eq!(response.0, StatusCode::CREATED);

        let created = state
            .db
            .find_user_auth_by_username("created_user")
            .await
            .expect("user lookup should work")
            .expect("created user should exist");
        assert!(created.3);
    }

    #[tokio::test]
    async fn admin_can_delete_user() {
        let state = make_state().await;
        let admin_id = state
            .db
            .create_user_with_role("admin_deleter", "hash", "admin")
            .await
            .expect("admin creation should work");
        let target_user_id = state
            .db
            .create_user_with_role("delete_me", "hash", "user")
            .await
            .expect("target user creation should work");

        let status = delete_user(
            State(state.clone()),
            axum::Extension(claims_for(admin_id, "admin_deleter", true)),
            Path(target_user_id),
        )
        .await
        .expect("delete should succeed");

        assert_eq!(status, StatusCode::NO_CONTENT);

        let still_exists = state
            .db
            .find_user_auth_by_username("delete_me")
            .await
            .expect("user lookup should work");
        assert!(still_exists.is_none());
    }
}
