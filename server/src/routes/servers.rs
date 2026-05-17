//! Endpoints de servidors.

#![allow(dead_code)]

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing::{get, put},
    Router, extract::Path,
};
use serde::{Deserialize, Serialize};
use shared::types::{ServerInfo, ServerFullInfo, ServerMember, ServerRole};
use uuid::Uuid;
use crate::{
    middleware::{AppState, AuthClaims},
    error::AppError,
};
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub icon_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InviteMemberRequest {
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberRoleRequest {
    pub role: ServerRole,
}

#[derive(Debug, Serialize)]
pub struct InviteMemberResponse {
    pub invited_user: String,
}

pub async fn list_servers(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<Vec<ServerInfo>>, AppError> {
    info!("Endpoint list_servers cridat per user_id={}", claims.user_id);
    let servers = state
        .db
        .list_servers_for_user(claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;
    Ok(Json(servers))
}

pub async fn create_server(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<CreateServerRequest>,
) -> Result<(StatusCode, Json<ServerFullInfo>), AppError> {
    info!("Endpoint create_server cridat per user_id={}, name={}", claims.user_id, req.name);

    if state.db.server_name_exists(&req.name).await.map_err(AppError::DatabaseError)? {
        return Err(AppError::ServerNameExists);
    }

    let server_id = Uuid::new_v4();
    state
        .db
        .create_server_with_owner(server_id, &req.name, req.icon_url.as_ref(), claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let server_info = state
        .db
        .get_server_full_info(server_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ServerNotFound)?;

    info!("Servidor creat amb èxit: server_id={}", server_id);
    Ok((StatusCode::CREATED, Json(server_info)))
}

pub async fn get_server(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<ServerFullInfo>, AppError> {
    info!("Endpoint get_server cridat: server_id={}, user_id={}", server_id, claims.user_id);

    if state
        .db
        .is_server_member(server_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .is_none()
    {
        return Err(AppError::NotServerMember);
    }

    let server_info = state
        .db
        .get_server_full_info(server_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ServerNotFound)?;
    Ok(Json(server_info))
}

pub async fn delete_server(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    info!("Endpoint delete_server cridat: server_id={}, user_id={}", server_id, claims.user_id);
    Ok(StatusCode::OK)
}

pub async fn list_server_members(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<Vec<ServerMember>>, AppError> {
    info!("Endpoint list_server_members cridat: server_id={}, user_id={}", server_id, claims.user_id);

    if state
        .db
        .is_server_member(server_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .is_none()
    {
        return Err(AppError::NotServerMember);
    }

    let server_info = state
        .db
        .get_server_full_info(server_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ServerNotFound)?;

    Ok(Json(server_info.members))
}

pub async fn invite_server_member(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
    Json(req): Json<InviteMemberRequest>,
) -> Result<(StatusCode, Json<InviteMemberResponse>), AppError> {
    info!("Endpoint invite_server_member cridat: server_id={}, username={}, user_id={}", server_id, req.username, claims.user_id);

    let current_role = state
        .db
        .is_server_member(server_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::NotServerMember)?;

    if current_role != "owner" && current_role != "admin" {
        return Err(AppError::ServerNotOwnerOrAdmin);
    }

    let user = state
        .db
        .find_user_by_username(&req.username)
        .await
        .map_err(|_| AppError::InternalError)?
        .ok_or(AppError::UserNotFound)?;

    let invited_user_id = user.0;
    if state
        .db
        .is_server_member(server_id, invited_user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .is_some()
    {
        return Err(AppError::MemberExists);
    }

    state
        .db
        .add_server_member(server_id, invited_user_id, "member")
        .await
        .map_err(AppError::DatabaseError)?;

    Ok((
        StatusCode::CREATED,
        Json(InviteMemberResponse { invited_user: req.username }),
    ))
}

pub async fn update_member_role(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path((server_id, user_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<UpdateMemberRoleRequest>,
) -> Result<Json<ServerMember>, AppError> {
    info!("Endpoint update_member_role cridat: server_id={}, user_id={}, target_user_id={}", server_id, claims.user_id, user_id);

    let current_role = state
        .db
        .is_server_member(server_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::NotServerMember)?;

    if current_role != "owner" && current_role != "admin" {
        return Err(AppError::ServerNotOwnerOrAdmin);
    }

    if req.role == ServerRole::Owner {
        return Err(AppError::Forbidden);
    }

    state
        .db
        .update_server_member_role(server_id, user_id, match req.role {
            ServerRole::Admin => "admin",
            ServerRole::Member => "member",
            ServerRole::Owner => unreachable!(),
        })
        .await
        .map_err(AppError::DatabaseError)?;

    let server_info = state
        .db
        .get_server_full_info(server_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ServerNotFound)?;

    let member = server_info
        .members
        .into_iter()
        .find(|m| m.user_id == user_id)
        .ok_or(AppError::MemberNotFound)?;

    Ok(Json(member))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/servers", get(list_servers).post(create_server))
        .route("/api/servers/{server_id}", get(get_server).delete(delete_server))
        .route("/api/servers/{server_id}/members", get(list_server_members).post(invite_server_member))
        .route("/api/servers/{server_id}/members/{user_id}/role", put(update_member_role))
        .with_state(state)
}