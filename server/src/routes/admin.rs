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

const SYSTEM_PLAN_NAMES: [&str; 3] = ["free", "pro", "enterprise"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserItem {
    pub user_id: Uuid,
    pub username: String,
    pub role: String,
    pub plan_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminServerItem {
    pub server_id: Uuid,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner_id: Uuid,
    pub member_count: u32,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateAdminUserRequest {
    pub username: String,
    pub password: String,
    pub role: Option<String>,
    pub plan_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAdminServerRequest {
    pub name: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateAdminServerRequest {
    pub name: Option<String>,
    pub icon_url: Option<Option<String>>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminPlanItem {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub max_servers: i32,
    pub max_channels_text_per_server: i32,
    pub max_channels_voice_per_server: i32,
    pub max_members_per_server: i32,
    pub api_calls_per_minute: i32,
    pub messages_per_day: i32,
    pub is_system: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAdminPlanRequest {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub max_servers: i32,
    pub max_channels_text_per_server: i32,
    pub max_channels_voice_per_server: i32,
    pub max_members_per_server: i32,
    pub api_calls_per_minute: i32,
    pub messages_per_day: i32,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAdminPlanRequest {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<Option<String>>,
    pub max_servers: Option<i32>,
    pub max_channels_text_per_server: Option<i32>,
    pub max_channels_voice_per_server: Option<i32>,
    pub max_members_per_server: Option<i32>,
    pub api_calls_per_minute: Option<i32>,
    pub messages_per_day: Option<i32>,
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

fn is_system_plan_name(name: &str) -> bool {
    SYSTEM_PLAN_NAMES.contains(&name)
}

fn normalize_plan_name(name: &str) -> Result<String, AppError> {
    let normalized = name.trim().to_ascii_lowercase();
    let valid = normalized
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
    if normalized.len() < 2 || normalized.len() > 32 || !valid {
        return Err(AppError::BadRequest);
    }

    Ok(normalized)
}

fn validate_limit_value(value: i32) -> Result<(), AppError> {
    if value < -1 {
        return Err(AppError::BadRequest);
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn to_admin_plan_item(
    id: Uuid,
    name: String,
    display_name: String,
    description: Option<String>,
    max_servers: i32,
    max_channels_text_per_server: i32,
    max_channels_voice_per_server: i32,
    max_members_per_server: i32,
    api_calls_per_minute: i32,
    messages_per_day: i32,
) -> AdminPlanItem {
    let is_system = is_system_plan_name(&name);
    AdminPlanItem {
        id,
        name,
        display_name,
        description,
        max_servers,
        max_channels_text_per_server,
        max_channels_voice_per_server,
        max_members_per_server,
        api_calls_per_minute,
        messages_per_day,
        is_system,
    }
}

pub async fn list_admin_plans(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let plans = state
        .db
        .list_plans_admin()
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let data = plans
        .into_iter()
        .map(
            |(
                id,
                name,
                display_name,
                description,
                max_servers,
                max_channels_text_per_server,
                max_channels_voice_per_server,
                max_members_per_server,
                api_calls_per_minute,
                messages_per_day,
            )| {
                to_admin_plan_item(
                    id,
                    name,
                    display_name,
                    description,
                    max_servers,
                    max_channels_text_per_server,
                    max_channels_voice_per_server,
                    max_members_per_server,
                    api_calls_per_minute,
                    messages_per_day,
                )
            },
        )
        .collect::<Vec<_>>();

    Ok(Json(serde_json::json!({
        "success": true,
        "data": data,
    })))
}

pub async fn create_admin_plan(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<CreateAdminPlanRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let name = normalize_plan_name(&req.name)?;
    let display_name = req.display_name.trim();
    if display_name.is_empty() {
        return Err(AppError::BadRequest);
    }
    validate_limit_value(req.max_servers)?;
    validate_limit_value(req.max_channels_text_per_server)?;
    validate_limit_value(req.max_channels_voice_per_server)?;
    validate_limit_value(req.max_members_per_server)?;
    validate_limit_value(req.api_calls_per_minute)?;
    validate_limit_value(req.messages_per_day)?;

    if state
        .db
        .get_plan_id_by_name(&name)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?
        .is_some()
    {
        return Err(AppError::PlanNameExists);
    }

    let plan_id = Uuid::new_v4();
    state
        .db
        .create_plan_admin(
            plan_id,
            &name,
            display_name,
            req.description.as_deref(),
            req.max_servers,
            req.max_channels_text_per_server,
            req.max_channels_voice_per_server,
            req.max_members_per_server,
            req.api_calls_per_minute,
            req.messages_per_day,
        )
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": {
                "id": plan_id,
                "name": name,
                "displayName": display_name,
                "description": req.description,
                "maxServers": req.max_servers,
                "maxChannelsTextPerServer": req.max_channels_text_per_server,
                "maxChannelsVoicePerServer": req.max_channels_voice_per_server,
                "maxMembersPerServer": req.max_members_per_server,
                "apiCallsPerMinute": req.api_calls_per_minute,
                "messagesPerDay": req.messages_per_day,
                "isSystem": false
            }
        })),
    ))
}

pub async fn update_admin_plan(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(plan_id): Path<Uuid>,
    Json(req): Json<UpdateAdminPlanRequest>,
) -> Result<StatusCode, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let existing = state
        .db
        .get_plan_by_id_admin(plan_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?
        .ok_or(AppError::PlanNotFound)?;

    if is_system_plan_name(&existing.1) {
        return Err(AppError::PlanProtected);
    }

    let name = match req.name {
        Some(name) => normalize_plan_name(&name)?,
        None => existing.1,
    };

    if let Some(other_plan_id) = state
        .db
        .get_plan_id_by_name(&name)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?
    {
        if other_plan_id != plan_id {
            return Err(AppError::PlanNameExists);
        }
    }

    let display_name = req
        .display_name
        .unwrap_or(existing.2)
        .trim()
        .to_string();
    if display_name.is_empty() {
        return Err(AppError::BadRequest);
    }

    let description = req.description.unwrap_or(existing.3);
    let max_servers = req.max_servers.unwrap_or(existing.4);
    let max_channels_text_per_server = req
        .max_channels_text_per_server
        .unwrap_or(existing.5);
    let max_channels_voice_per_server = req
        .max_channels_voice_per_server
        .unwrap_or(existing.6);
    let max_members_per_server = req.max_members_per_server.unwrap_or(existing.7);
    let api_calls_per_minute = req.api_calls_per_minute.unwrap_or(existing.8);
    let messages_per_day = req.messages_per_day.unwrap_or(existing.9);

    validate_limit_value(max_servers)?;
    validate_limit_value(max_channels_text_per_server)?;
    validate_limit_value(max_channels_voice_per_server)?;
    validate_limit_value(max_members_per_server)?;
    validate_limit_value(api_calls_per_minute)?;
    validate_limit_value(messages_per_day)?;

    let updated = state
        .db
        .update_plan_by_id(
            plan_id,
            &name,
            &display_name,
            description.as_deref(),
            max_servers,
            max_channels_text_per_server,
            max_channels_voice_per_server,
            max_members_per_server,
            api_calls_per_minute,
            messages_per_day,
        )
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    if !updated {
        return Err(AppError::PlanNotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_admin_plan(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(plan_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let existing = state
        .db
        .get_plan_by_id_admin(plan_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?
        .ok_or(AppError::PlanNotFound)?;

    if is_system_plan_name(&existing.1) {
        return Err(AppError::PlanProtected);
    }

    let users_count = state
        .db
        .count_users_with_plan(plan_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;
    if users_count > 0 {
        return Err(AppError::PlanInUse);
    }

    let deleted = state
        .db
        .delete_plan_by_id(plan_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;
    if !deleted {
        return Err(AppError::PlanNotFound);
    }

    Ok(StatusCode::NO_CONTENT)
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

pub async fn list_servers(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let servers = state
        .db
        .list_all_servers_admin()
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let data: Vec<AdminServerItem> = servers
        .into_iter()
        .map(|(server_id, name, icon_url, owner_id, member_count, created_at)| AdminServerItem {
            server_id,
            name,
            icon_url,
            owner_id,
            member_count,
            created_at,
        })
        .collect();

    Ok(Json(serde_json::json!({
        "success": true,
        "data": data,
    })))
}

pub async fn create_server(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<CreateAdminServerRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let name = req.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest);
    }

    if state
        .db
        .server_name_exists(name)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?
    {
        return Err(AppError::ServerNameExists);
    }

    let server_id = Uuid::new_v4();
    state
        .db
        .create_server_with_owner(server_id, name, req.icon_url.as_ref(), claims.user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": {
                "serverId": server_id,
                "name": name,
            }
        })),
    ))
}

pub async fn update_server(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
    Json(req): Json<UpdateAdminServerRequest>,
) -> Result<StatusCode, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    if let Some(name) = &req.name {
        if name.trim().is_empty() {
            return Err(AppError::BadRequest);
        }
    }

    state
        .db
        .update_server_metadata(
            server_id,
            req.name.as_deref(),
            req.icon_url.as_ref().map(|icon| icon.as_deref()),
        )
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let exists = state
        .db
        .get_server_full_info(server_id, claims.user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?
        .is_some();

    if !exists {
        return Err(AppError::ServerNotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_server(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let deleted = state
        .db
        .delete_server(server_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    if !deleted {
        return Err(AppError::ServerNotFound);
    }

    Ok(StatusCode::NO_CONTENT)
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
        .route("/api/admin/servers", get(list_servers).post(create_server))
        .route(
            "/api/admin/servers/{server_id}",
            put(update_server).delete(delete_server),
        )
        .route("/api/admin/plans", get(list_admin_plans).post(create_admin_plan))
        .route(
            "/api/admin/plans/{plan_id}",
            put(update_admin_plan).delete(delete_admin_plan),
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
            static_dir: None,
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

    #[tokio::test]
    async fn admin_can_create_update_and_delete_custom_plan() {
        let state = make_state().await;
        let admin_id = state
            .db
            .create_user_with_role("admin_plan_mgr", "hash", "admin")
            .await
            .expect("admin creation should work");

        let (status, body) = create_admin_plan(
            State(state.clone()),
            axum::Extension(claims_for(admin_id, "admin_plan_mgr", true)),
            Json(CreateAdminPlanRequest {
                name: "team_plus".to_string(),
                display_name: "Team Plus".to_string(),
                description: Some("Plan per equips".to_string()),
                max_servers: 8,
                max_channels_text_per_server: 30,
                max_channels_voice_per_server: 15,
                max_members_per_server: 800,
                api_calls_per_minute: 1200,
                messages_per_day: -1,
            }),
        )
        .await
        .expect("plan creation should work");

        assert_eq!(status, StatusCode::CREATED);
        let plan_id = Uuid::parse_str(
            body.0["data"]["id"]
                .as_str()
                .expect("created plan id should be string"),
        )
        .expect("created plan id should be valid uuid");

        let update_status = update_admin_plan(
            State(state.clone()),
            axum::Extension(claims_for(admin_id, "admin_plan_mgr", true)),
            Path(plan_id),
            Json(UpdateAdminPlanRequest {
                display_name: Some("Team Plus Updated".to_string()),
                max_servers: Some(10),
                ..Default::default()
            }),
        )
        .await
        .expect("plan update should work");
        assert_eq!(update_status, StatusCode::NO_CONTENT);

        let updated_plan = state
            .db
            .get_plan_by_id_admin(plan_id)
            .await
            .expect("plan lookup should work")
            .expect("plan should exist");
        assert_eq!(updated_plan.2, "Team Plus Updated");
        assert_eq!(updated_plan.4, 10);

        let delete_status = delete_admin_plan(
            State(state.clone()),
            axum::Extension(claims_for(admin_id, "admin_plan_mgr", true)),
            Path(plan_id),
        )
        .await
        .expect("plan delete should work");
        assert_eq!(delete_status, StatusCode::NO_CONTENT);

        let deleted_plan = state
            .db
            .get_plan_by_id_admin(plan_id)
            .await
            .expect("plan lookup should work after delete");
        assert!(deleted_plan.is_none());
    }

    #[tokio::test]
    async fn admin_cannot_delete_plan_if_in_use() {
        let state = make_state().await;
        let admin_id = state
            .db
            .create_user_with_role("admin_plan_guard", "hash", "admin")
            .await
            .expect("admin creation should work");
        let target_user_id = state
            .db
            .create_user_with_role("plan_target_user", "hash", "user")
            .await
            .expect("user creation should work");

        let created = create_admin_plan(
            State(state.clone()),
            axum::Extension(claims_for(admin_id, "admin_plan_guard", true)),
            Json(CreateAdminPlanRequest {
                name: "starter_plus".to_string(),
                display_name: "Starter Plus".to_string(),
                description: None,
                max_servers: 2,
                max_channels_text_per_server: 6,
                max_channels_voice_per_server: 4,
                max_members_per_server: 40,
                api_calls_per_minute: 120,
                messages_per_day: 20000,
            }),
        )
        .await
        .expect("plan creation should work");

        let plan_id = Uuid::parse_str(
            created
                .1
                .0["data"]["id"]
                .as_str()
                .expect("created plan id should be string"),
        )
        .expect("created plan id should be valid uuid");

        state
            .db
            .set_user_plan_by_id(target_user_id, plan_id)
            .await
            .expect("assigning plan should work");

        let result = delete_admin_plan(
            State(state),
            axum::Extension(claims_for(admin_id, "admin_plan_guard", true)),
            Path(plan_id),
        )
        .await;

        assert!(matches!(result, Err(AppError::PlanInUse)));
    }

    #[tokio::test]
    async fn admin_cannot_modify_system_plan() {
        let state = make_state().await;
        let admin_id = state
            .db
            .create_user_with_role("admin_plan_lock", "hash", "admin")
            .await
            .expect("admin creation should work");

        let free_plan_id = state
            .db
            .get_plan_id_by_name("free")
            .await
            .expect("free plan lookup should work")
            .expect("free plan should exist");

        let update_result = update_admin_plan(
            State(state.clone()),
            axum::Extension(claims_for(admin_id, "admin_plan_lock", true)),
            Path(free_plan_id),
            Json(UpdateAdminPlanRequest {
                display_name: Some("Free Updated".to_string()),
                ..Default::default()
            }),
        )
        .await;

        assert!(matches!(update_result, Err(AppError::PlanProtected)));

        let delete_result = delete_admin_plan(
            State(state),
            axum::Extension(claims_for(admin_id, "admin_plan_lock", true)),
            Path(free_plan_id),
        )
        .await;

        assert!(matches!(delete_result, Err(AppError::PlanProtected)));
    }
}
