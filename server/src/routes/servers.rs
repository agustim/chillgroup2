//! Endpoints de servidors.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
    routing::{delete, get, put},
    Router, extract::Path,
};
use serde::{Deserialize, Serialize};
use shared::types::{ServerInfo, ServerFullInfo, ServerMember, ServerRole};
use uuid::Uuid;
use crate::{
    db::{
        SERVER_PERMISSION_MANAGE_MEMBERS,
        SERVER_PERMISSION_MANAGE_PROFILE,
        SERVER_PERMISSION_VIEW,
    },
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

#[derive(Debug, Default, Deserialize)]
pub struct UpdateServerRequest {
    pub name: Option<String>,
    pub icon_url: Option<Option<String>>,
    pub livekit_host: Option<Option<String>>,
    pub livekit_api_key: Option<Option<String>>,
    pub livekit_api_secret: Option<Option<String>>,
}

#[derive(Debug, Serialize)]
pub struct InviteMemberResponse {
    pub invited_user: String,
}

#[derive(Debug, Serialize)]
pub struct RemoveMemberResponse {
    pub user_id: Uuid,
    pub removed: bool,
}

#[derive(Debug, Serialize)]
pub struct DeleteServerResponse {
    pub server_id: Uuid,
    pub deleted: bool,
}

#[derive(Debug, Serialize)]
pub struct LeaveServerResponse {
    pub server_id: Uuid,
    pub left: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct LeaveServerParams {
    #[serde(default)]
    pub force: bool,
}

async fn ensure_server_permission(
    state: &AppState,
    server_id: Uuid,
    user_id: Uuid,
    is_admin: bool,
    min_level: i32,
) -> Result<i32, AppError> {
    if is_admin {
        return Ok(SERVER_PERMISSION_MANAGE_MEMBERS);
    }

    let level = state
        .db
        .get_server_permission_level(server_id, user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .unwrap_or(0);

    if level < min_level {
        return Err(AppError::Forbidden);
    }

    Ok(level)
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

    let max_servers = state
        .db
        .get_user_max_servers(claims.user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;
    let owned_servers = state
        .db
        .count_owned_servers(claims.user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    if max_servers != -1 && owned_servers >= i64::from(max_servers) {
        return Err(AppError::ServerLimitExceeded);
    }

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
        .get_server_full_info(server_id, claims.user_id)
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

    ensure_server_permission(&state, server_id, claims.user_id, claims.is_admin, SERVER_PERMISSION_VIEW).await?;

    let server_info = state
        .db
        .get_server_full_info(server_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ServerNotFound)?;
    Ok(Json(server_info))
}

pub async fn update_server(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
    Json(req): Json<UpdateServerRequest>,
) -> Result<Json<ServerFullInfo>, AppError> {
    ensure_server_permission(&state, server_id, claims.user_id, claims.is_admin, SERVER_PERMISSION_MANAGE_PROFILE).await?;

    let livekit_fields_present = [
        req.livekit_host.is_some(),
        req.livekit_api_key.is_some(),
        req.livekit_api_secret.is_some(),
    ];

    if livekit_fields_present.iter().any(|present| *present)
        && !livekit_fields_present.iter().all(|present| *present)
    {
        return Err(AppError::BadRequest);
    }

    state
        .db
        .update_server_metadata(
            server_id,
            req.name.as_deref(),
            req.icon_url.as_ref().map(|icon| icon.as_deref()),
        )
        .await
        .map_err(AppError::DatabaseError)?;

    if livekit_fields_present.iter().all(|present| *present) {
        state
            .db
            .set_server_livekit_override(
                server_id,
                req.livekit_host.as_ref().and_then(|value| value.as_deref()),
                req.livekit_api_key.as_ref().and_then(|value| value.as_deref()),
                req.livekit_api_secret.as_ref().and_then(|value| value.as_deref()),
            )
            .await
            .map_err(AppError::DatabaseError)?;
    }

    let server_info = state
        .db
        .get_server_full_info(server_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ServerNotFound)?;

    Ok(Json(server_info))
}

pub async fn delete_server(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<DeleteServerResponse>, AppError> {
    info!("Endpoint delete_server cridat: server_id={}, user_id={}", server_id, claims.user_id);

    if !claims.is_admin {
        let my_role = state
            .db
            .is_server_member(server_id, claims.user_id)
            .await
            .map_err(AppError::DatabaseError)?;

        if my_role.as_deref() != Some("owner") {
            return Err(AppError::Forbidden);
        }
    }

    let deleted = state
        .db
        .delete_server(server_id)
        .await
        .map_err(AppError::DatabaseError)?;

    if !deleted {
        return Err(AppError::ServerNotFound);
    }

    Ok(Json(DeleteServerResponse {
        server_id,
        deleted: true,
    }))
}

pub async fn list_server_members(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<Vec<ServerMember>>, AppError> {
    info!("Endpoint list_server_members cridat: server_id={}, user_id={}", server_id, claims.user_id);

    ensure_server_permission(&state, server_id, claims.user_id, claims.is_admin, SERVER_PERMISSION_VIEW).await?;

    let server_info = state
        .db
        .get_server_full_info(server_id, claims.user_id)
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

    ensure_server_permission(&state, server_id, claims.user_id, claims.is_admin, SERVER_PERMISSION_MANAGE_MEMBERS).await?;

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

    // Crear invitació pendent — l'usuari l'ha d'acceptar explícitament
    let invitation_id = Uuid::new_v4();
    state
        .db
        .create_server_invitation(invitation_id, server_id, claims.user_id, invited_user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let server_name = state
        .db
        .get_server_full_info(server_id, claims.user_id)
        .await
        .ok()
        .flatten()
        .map(|s| s.name)
        .unwrap_or_default();

    let invitation_event = serde_json::json!({
        "invitationId": invitation_id,
        "serverId": server_id,
        "serverName": server_name,
        "inviterUsername": claims.username,
    });
    let invited_user_room = format!("user:{}", invited_user_id);
    if let Err(e) = state
        .io
        .to(invited_user_room)
        .emit("server-invitation", &invitation_event)
        .await
    {
        tracing::warn!("Error enviant server-invitation: {:?}", e);
    }

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

    ensure_server_permission(&state, server_id, claims.user_id, claims.is_admin, SERVER_PERMISSION_MANAGE_MEMBERS).await?;

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
        .get_server_full_info(server_id, claims.user_id)
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

pub async fn remove_server_member(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path((server_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RemoveMemberResponse>, AppError> {
    ensure_server_permission(&state, server_id, claims.user_id, claims.is_admin, SERVER_PERMISSION_MANAGE_MEMBERS).await?;

    let target_role = state
        .db
        .is_server_member(server_id, user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::MemberNotFound)?;

    if target_role == "owner" {
        return Err(AppError::Forbidden);
    }

    let removed = state
        .db
        .remove_server_member(server_id, user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    if !removed {
        return Err(AppError::MemberNotFound);
    }

    Ok(Json(RemoveMemberResponse {
        user_id,
        removed: true,
    }))
}

pub async fn leave_server(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
    Query(params): Query<LeaveServerParams>,
) -> Result<Json<LeaveServerResponse>, AppError> {
    info!("Endpoint leave_server cridat: server_id={}, user_id={}, force={}", server_id, claims.user_id, params.force);

    let role = state
        .db
        .is_server_member(server_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::MemberNotFound)?;

    if role == "owner" {
        return Err(AppError::OwnerCannotLeave);
    }

    if role == "admin" && !params.force {
        let admin_count = state
            .db
            .count_server_admins(server_id)
            .await
            .map_err(AppError::DatabaseError)?;

        if admin_count <= 1 {
            return Err(AppError::ServerLastAdmin);
        }
    }

    state
        .db
        .remove_server_member(server_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(Json(LeaveServerResponse {
        server_id,
        left: true,
    }))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/servers", get(list_servers).post(create_server))
        .route("/api/servers/{server_id}", get(get_server).put(update_server).delete(delete_server))
        .route("/api/servers/{server_id}/members", get(list_server_members).post(invite_server_member))
        .route("/api/servers/{server_id}/members/me", delete(leave_server))
        .route("/api/servers/{server_id}/members/{user_id}/role", put(update_member_role))
        .route("/api/servers/{server_id}/members/{user_id}", delete(remove_server_member))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, LogLevel},
        db::{connect_db, ChannelKeyBundleWriteResult, DatabasePool},
        middleware::auth::UserPresenceState,
    };
    use chrono::Utc;
    use axum::response::IntoResponse;
    use socketioxide::extract::{Data, SocketRef};
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
            max_file_size_bytes: 100 * 1024 * 1024,
            allowed_origins: vec![],
        };

        let db = connect_db(&config).await.expect("sqlite test db should initialize");
        let (_layer, io) = socketioxide::SocketIo::new_layer();
        io.ns("/", |_socket: SocketRef, Data(_auth): Data<serde_json::Value>| async move {});

        AppState {
            db,
            config,
            io,
            user_presence: Arc::new(RwLock::new(UserPresenceState {
                online_sockets: HashMap::<Uuid, HashSet<String>>::new(),
            })),
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

    fn claims_for_admin(user_id: Uuid, username: &str) -> AuthClaims {
        AuthClaims {
            user_id,
            username: username.to_string(),
            device_id: Uuid::new_v4(),
            is_admin: true,
            exp: 0,
            iat: 0,
            jti: Uuid::new_v4().to_string(),
        }
    }

    #[tokio::test]
    async fn create_server_returns_429_when_free_plan_limit_reached() {
        let state = make_state().await;
        let user_id = state
            .db
            .create_user_with_role("free_user", "hash", "user")
            .await
            .expect("user creation should work");

        state
            .db
            .create_server_with_owner(Uuid::new_v4(), "existing-server", None, user_id)
            .await
            .expect("initial server should be created");

        let result = create_server(
            State(state),
            axum::Extension(claims_for(user_id, "free_user")),
            Json(CreateServerRequest {
                name: "new-server".to_string(),
                icon_url: None,
            }),
        )
        .await;

        let err = result.expect_err("free plan should not allow second server");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn create_server_allows_second_server_on_pro_plan() {
        let state = make_state().await;
        let user_id = state
            .db
            .create_user_with_role("pro_user", "hash", "user")
            .await
            .expect("user creation should work");

        state
            .db
            .set_user_plan_by_name(user_id, "pro")
            .await
            .expect("should assign pro plan");

        state
            .db
            .create_server_with_owner(Uuid::new_v4(), "existing-server-pro", None, user_id)
            .await
            .expect("initial server should be created");

        let result = create_server(
            State(state),
            axum::Extension(claims_for(user_id, "pro_user")),
            Json(CreateServerRequest {
                name: "second-server-pro".to_string(),
                icon_url: None,
            }),
        )
        .await;

        assert!(result.is_ok(), "pro plan should allow creating second server");
        let (status, _) = result.expect("request should succeed");
        assert_eq!(status, StatusCode::CREATED);
    }

    #[tokio::test]
    async fn invite_server_member_forbidden_for_plain_member() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_user", "hash", "user")
            .await
            .expect("owner user should be created");
        let member_id = state
            .db
            .create_user_with_role("member_user", "hash", "user")
            .await
            .expect("member user should be created");
        state
            .db
            .create_user_with_role("target_user", "hash", "user")
            .await
            .expect("target user should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "perm-server", None, owner_id)
            .await
            .expect("server should be created");

        state
            .db
            .add_server_member(server_id, member_id, "member")
            .await
            .expect("member should be added");

        let result = invite_server_member(
            State(state),
            axum::Extension(claims_for(member_id, "member_user")),
            Path(server_id),
            Json(InviteMemberRequest {
                username: "target_user".to_string(),
            }),
        )
        .await;

        let err = result.expect_err("plain member should not invite users");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

    }

    #[tokio::test]
    async fn update_server_allows_admin_manage_profile() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_profile", "hash", "user")
            .await
            .expect("owner user should be created");
        let admin_id = state
            .db
            .create_user_with_role("admin_profile", "hash", "user")
            .await
            .expect("admin user should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "server-before", None, owner_id)
            .await
            .expect("server should be created");

        state
            .db
            .add_server_member(server_id, admin_id, "admin")
            .await
            .expect("admin should be added");

        let result = update_server(
            State(state),
            axum::Extension(claims_for(admin_id, "admin_profile")),
            Path(server_id),
            Json(UpdateServerRequest {
                name: Some("server-after".to_string()),
                icon_url: Some(Some("https://example.com/icon.png".to_string())),
                ..Default::default()
            }),
        )
        .await
        .expect("admin should be able to update server profile");

        assert_eq!(result.0.name, "server-after");
        assert_eq!(result.0.icon_url.as_deref(), Some("https://example.com/icon.png"));
    }

    #[tokio::test]
    async fn global_admin_can_manage_server_without_membership() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_non_member_admin", "hash", "user")
            .await
            .expect("owner user should be created");
        let global_admin_id = state
            .db
            .create_user_with_role("global_admin_non_member", "hash", "admin")
            .await
            .expect("global admin user should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "server-non-member-before", None, owner_id)
            .await
            .expect("server should be created");

        let fetched = get_server(
            State(state.clone()),
            axum::Extension(claims_for_admin(global_admin_id, "global_admin_non_member")),
            Path(server_id),
        )
        .await
        .expect("global admin should read server without membership");

        assert_eq!(fetched.0.server_id, server_id);
        assert_eq!(fetched.0.name, "server-non-member-before");

        let updated = update_server(
            State(state.clone()),
            axum::Extension(claims_for_admin(global_admin_id, "global_admin_non_member")),
            Path(server_id),
            Json(UpdateServerRequest {
                name: Some("server-non-member-after".to_string()),
                icon_url: Some(Some("https://example.com/admin-non-member.png".to_string())),
                ..Default::default()
            }),
        )
        .await
        .expect("global admin should update server without membership");

        assert_eq!(updated.0.name, "server-non-member-after");
        assert_eq!(updated.0.icon_url.as_deref(), Some("https://example.com/admin-non-member.png"));
    }

    #[tokio::test]
    async fn delete_server_forbidden_for_admin() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_delete_admin", "hash", "user")
            .await
            .expect("owner user should be created");
        let admin_id = state
            .db
            .create_user_with_role("admin_delete_admin", "hash", "user")
            .await
            .expect("admin user should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "delete-admin-server", None, owner_id)
            .await
            .expect("server should be created");
        state
            .db
            .add_server_member(server_id, admin_id, "admin")
            .await
            .expect("admin should be added");

        let result = delete_server(
            State(state),
            axum::Extension(claims_for(admin_id, "admin_delete_admin")),
            Path(server_id),
        )
        .await;

        let err = result.expect_err("admin should not delete server");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn delete_server_forbidden_for_plain_member() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_delete_member", "hash", "user")
            .await
            .expect("owner user should be created");
        let member_id = state
            .db
            .create_user_with_role("member_delete_member", "hash", "user")
            .await
            .expect("member user should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "delete-member-server", None, owner_id)
            .await
            .expect("server should be created");
        state
            .db
            .add_server_member(server_id, member_id, "member")
            .await
            .expect("member should be added");

        let result = delete_server(
            State(state),
            axum::Extension(claims_for(member_id, "member_delete_member")),
            Path(server_id),
        )
        .await;

        let err = result.expect_err("member should not delete server");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn delete_server_succeeds_for_owner() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_delete_ok", "hash", "user")
            .await
            .expect("owner user should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "delete-owner-server", None, owner_id)
            .await
            .expect("server should be created");

        let result = delete_server(
            State(state.clone()),
            axum::Extension(claims_for(owner_id, "owner_delete_ok")),
            Path(server_id),
        )
        .await
        .expect("owner should delete server");

        assert_eq!(result.0.server_id, server_id);
        assert!(result.0.deleted);

        let deleted_server = state
            .db
            .get_server_full_info(server_id, owner_id)
            .await
            .expect("db query should work");
        assert!(deleted_server.is_none(), "server should be removed");

        let owner_membership = state
            .db
            .is_server_member(server_id, owner_id)
            .await
            .expect("membership query should work");
        assert!(owner_membership.is_none(), "owner membership should be removed");
    }

    #[tokio::test]
    async fn delete_server_performs_strong_cascade_cleanup() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_delete_cascade", "hash", "user")
            .await
            .expect("owner user should be created");
        let member_id = state
            .db
            .create_user_with_role("member_delete_cascade", "hash", "user")
            .await
            .expect("member user should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "delete-cascade-server", None, owner_id)
            .await
            .expect("server should be created");
        state
            .db
            .add_server_member(server_id, member_id, "member")
            .await
            .expect("member should be added");

        let channel_id = Uuid::new_v4();
        state
            .db
            .create_channel(channel_id, server_id, "cascade-text", "text", "symmetric", Some(3600), false)
            .await
            .expect("channel should be created");

        let owner_device_id = state
            .db
            .upsert_device_for_user(owner_id, "owner-device", None)
            .await
            .expect("owner device should be created");
        let member_device_id = state
            .db
            .upsert_device_for_user(member_id, "member-device", None)
            .await
            .expect("member device should be created");

        let message_id = Uuid::new_v4();
        state
            .db
            .create_message(
                message_id,
                channel_id,
                owner_id,
                "owner_delete_cascade",
                owner_device_id,
                "encrypted-payload",
                "iv",
                Some(1),
                None,
                Utc::now(),
                None,
            )
            .await
            .expect("message should be created");

        let key_version_id = state
            .db
            .create_channel_key_version(channel_id, 1, "encrypted-key", "nonce", owner_id)
            .await
            .expect("key version should be created");
        let bundle_write = state
            .db
            .store_channel_key_bundle_for_device(
                key_version_id,
                member_device_id,
                "bundle-key",
                "bundle-kem",
                None,
                None,
            )
            .await
            .expect("bundle store should succeed");
        assert_eq!(bundle_write, ChannelKeyBundleWriteResult::Inserted);

        state
            .db
            .mark_channel_read(owner_id, channel_id, Some(message_id))
            .await
            .expect("read state should be created");

        match &state.db {
            DatabasePool::Sqlite(pool) => {
                let channels_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channels WHERE server_id = ?")
                    .bind(server_id)
                    .fetch_one(pool)
                    .await
                    .expect("channels count should work");
                let messages_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE channel_id = ?")
                    .bind(channel_id)
                    .fetch_one(pool)
                    .await
                    .expect("messages count should work");
                let key_versions_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_key_versions WHERE channel_id = ?")
                    .bind(channel_id)
                    .fetch_one(pool)
                    .await
                    .expect("key versions count should work");
                let bundles_before: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM channel_key_device_bundles WHERE key_version_id = ?",
                )
                .bind(key_version_id)
                .fetch_one(pool)
                .await
                .expect("bundles count should work");
                let read_state_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_read_state WHERE channel_id = ?")
                    .bind(channel_id)
                    .fetch_one(pool)
                    .await
                    .expect("read state count should work");
                let members_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_members WHERE server_id = ?")
                    .bind(server_id)
                    .fetch_one(pool)
                    .await
                    .expect("members count should work");

                assert_eq!(channels_before, 1);
                assert_eq!(messages_before, 1);
                assert_eq!(key_versions_before, 1);
                assert_eq!(bundles_before, 1);
                assert_eq!(read_state_before, 1);
                assert_eq!(members_before, 2);
            }
            DatabasePool::Postgres(pool) => {
                let channels_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channels WHERE server_id = $1")
                    .bind(server_id)
                    .fetch_one(pool)
                    .await
                    .expect("channels count should work");
                let messages_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE channel_id = $1")
                    .bind(channel_id)
                    .fetch_one(pool)
                    .await
                    .expect("messages count should work");
                let key_versions_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_key_versions WHERE channel_id = $1")
                    .bind(channel_id)
                    .fetch_one(pool)
                    .await
                    .expect("key versions count should work");
                let bundles_before: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM channel_key_device_bundles WHERE key_version_id = $1",
                )
                .bind(key_version_id)
                .fetch_one(pool)
                .await
                .expect("bundles count should work");
                let read_state_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_read_state WHERE channel_id = $1")
                    .bind(channel_id)
                    .fetch_one(pool)
                    .await
                    .expect("read state count should work");
                let members_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_members WHERE server_id = $1")
                    .bind(server_id)
                    .fetch_one(pool)
                    .await
                    .expect("members count should work");

                assert_eq!(channels_before, 1);
                assert_eq!(messages_before, 1);
                assert_eq!(key_versions_before, 1);
                assert_eq!(bundles_before, 1);
                assert_eq!(read_state_before, 1);
                assert_eq!(members_before, 2);
            }
        }

        let delete_result = delete_server(
            State(state.clone()),
            axum::Extension(claims_for(owner_id, "owner_delete_cascade")),
            Path(server_id),
        )
        .await
        .expect("owner should delete server");
        assert!(delete_result.0.deleted);

        match &state.db {
            DatabasePool::Sqlite(pool) => {
                let servers_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM servers WHERE id = ?")
                    .bind(server_id)
                    .fetch_one(pool)
                    .await
                    .expect("servers count should work");
                let channels_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channels WHERE server_id = ?")
                    .bind(server_id)
                    .fetch_one(pool)
                    .await
                    .expect("channels count should work");
                let messages_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE channel_id = ?")
                    .bind(channel_id)
                    .fetch_one(pool)
                    .await
                    .expect("messages count should work");
                let key_versions_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_key_versions WHERE channel_id = ?")
                    .bind(channel_id)
                    .fetch_one(pool)
                    .await
                    .expect("key versions count should work");
                let bundles_after: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM channel_key_device_bundles WHERE key_version_id = ?",
                )
                .bind(key_version_id)
                .fetch_one(pool)
                .await
                .expect("bundles count should work");
                let read_state_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_read_state WHERE channel_id = ?")
                    .bind(channel_id)
                    .fetch_one(pool)
                    .await
                    .expect("read state count should work");
                let members_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_members WHERE server_id = ?")
                    .bind(server_id)
                    .fetch_one(pool)
                    .await
                    .expect("members count should work");

                assert_eq!(servers_after, 0);
                assert_eq!(channels_after, 0);
                assert_eq!(messages_after, 0);
                assert_eq!(key_versions_after, 0);
                assert_eq!(bundles_after, 0);
                assert_eq!(read_state_after, 0);
                assert_eq!(members_after, 0);
            }
            DatabasePool::Postgres(pool) => {
                let servers_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM servers WHERE id = $1")
                    .bind(server_id)
                    .fetch_one(pool)
                    .await
                    .expect("servers count should work");
                let channels_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channels WHERE server_id = $1")
                    .bind(server_id)
                    .fetch_one(pool)
                    .await
                    .expect("channels count should work");
                let messages_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE channel_id = $1")
                    .bind(channel_id)
                    .fetch_one(pool)
                    .await
                    .expect("messages count should work");
                let key_versions_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_key_versions WHERE channel_id = $1")
                    .bind(channel_id)
                    .fetch_one(pool)
                    .await
                    .expect("key versions count should work");
                let bundles_after: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM channel_key_device_bundles WHERE key_version_id = $1",
                )
                .bind(key_version_id)
                .fetch_one(pool)
                .await
                .expect("bundles count should work");
                let read_state_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_read_state WHERE channel_id = $1")
                    .bind(channel_id)
                    .fetch_one(pool)
                    .await
                    .expect("read state count should work");
                let members_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM server_members WHERE server_id = $1")
                    .bind(server_id)
                    .fetch_one(pool)
                    .await
                    .expect("members count should work");

                assert_eq!(servers_after, 0);
                assert_eq!(channels_after, 0);
                assert_eq!(messages_after, 0);
                assert_eq!(key_versions_after, 0);
                assert_eq!(bundles_after, 0);
                assert_eq!(read_state_after, 0);
                assert_eq!(members_after, 0);
            }
        }
    }

    #[tokio::test]
    async fn remove_server_member_forbidden_for_plain_member() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_remove", "hash", "user")
            .await
            .expect("owner user should be created");
        let member_id = state
            .db
            .create_user_with_role("member_remove", "hash", "user")
            .await
            .expect("member user should be created");
        let target_id = state
            .db
            .create_user_with_role("target_remove", "hash", "user")
            .await
            .expect("target user should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "remove-server", None, owner_id)
            .await
            .expect("server should be created");

        state
            .db
            .add_server_member(server_id, member_id, "member")
            .await
            .expect("member should be added");
        state
            .db
            .add_server_member(server_id, target_id, "member")
            .await
            .expect("target should be added");

        let result = remove_server_member(
            State(state),
            axum::Extension(claims_for(member_id, "member_remove")),
            Path((server_id, target_id)),
        )
        .await;

        let err = result.expect_err("plain member should not remove members");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn remove_server_member_succeeds_for_admin() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_remove_ok", "hash", "user")
            .await
            .expect("owner user should be created");
        let admin_id = state
            .db
            .create_user_with_role("admin_remove_ok", "hash", "user")
            .await
            .expect("admin user should be created");
        let target_id = state
            .db
            .create_user_with_role("target_remove_ok", "hash", "user")
            .await
            .expect("target user should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "remove-server-ok", None, owner_id)
            .await
            .expect("server should be created");

        state
            .db
            .add_server_member(server_id, admin_id, "admin")
            .await
            .expect("admin should be added");
        state
            .db
            .add_server_member(server_id, target_id, "member")
            .await
            .expect("target should be added");

        let result = remove_server_member(
            State(state.clone()),
            axum::Extension(claims_for(admin_id, "admin_remove_ok")),
            Path((server_id, target_id)),
        )
        .await
        .expect("admin should remove member");

        assert_eq!(result.0.user_id, target_id);
        assert!(result.0.removed);

        let still_member = state
            .db
            .is_server_member(server_id, target_id)
            .await
            .expect("db query should work");
        assert!(still_member.is_none(), "target should be removed from server");
    }

    #[tokio::test]
    async fn remove_server_member_rejects_owner_target() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_remove_owner", "hash", "user")
            .await
            .expect("owner user should be created");
        let admin_id = state
            .db
            .create_user_with_role("admin_remove_owner", "hash", "user")
            .await
            .expect("admin user should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "remove-owner-guard", None, owner_id)
            .await
            .expect("server should be created");

        state
            .db
            .add_server_member(server_id, admin_id, "admin")
            .await
            .expect("admin should be added");

        let result = remove_server_member(
            State(state),
            axum::Extension(claims_for(admin_id, "admin_remove_owner")),
            Path((server_id, owner_id)),
        )
        .await;

        let err = result.expect_err("owner should not be removable");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn global_admin_can_delete_server_without_membership() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_non_member_delete", "hash", "user")
            .await
            .expect("owner user should be created");
        let global_admin_id = state
            .db
            .create_user_with_role("global_admin_non_member_delete", "hash", "admin")
            .await
            .expect("global admin should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "delete-non-member-admin", None, owner_id)
            .await
            .expect("server should be created");

        let result = delete_server(
            State(state.clone()),
            axum::Extension(claims_for_admin(global_admin_id, "global_admin_non_member_delete")),
            Path(server_id),
        )
        .await
        .expect("global admin should delete server without membership");

        assert_eq!(result.0.server_id, server_id);
        assert!(result.0.deleted);

        let deleted_server = state
            .db
            .get_server_full_info(server_id, owner_id)
            .await
            .expect("db query should work");
        assert!(deleted_server.is_none(), "server should be removed");
    }

    #[tokio::test]
    async fn global_admin_can_list_server_members_without_membership() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_non_member_list_members", "hash", "user")
            .await
            .expect("owner user should be created");
        let member_id = state
            .db
            .create_user_with_role("member_non_member_list_members", "hash", "user")
            .await
            .expect("member user should be created");
        let global_admin_id = state
            .db
            .create_user_with_role("global_admin_non_member_list_members", "hash", "admin")
            .await
            .expect("global admin should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "members-visible-for-global-admin", None, owner_id)
            .await
            .expect("server should be created");
        state
            .db
            .add_server_member(server_id, member_id, "member")
            .await
            .expect("member should be added");

        let result = list_server_members(
            State(state),
            axum::Extension(claims_for_admin(global_admin_id, "global_admin_non_member_list_members")),
            Path(server_id),
        )
        .await
        .expect("global admin should list members without membership");

        assert_eq!(result.0.len(), 2);
        assert!(result.0.iter().any(|m| m.user_id == owner_id));
        assert!(result.0.iter().any(|m| m.user_id == member_id));
    }

    #[tokio::test]
    async fn global_admin_can_manage_members_without_membership() {
        let state = make_state().await;
        let owner_id = state
            .db
            .create_user_with_role("owner_non_member_manage_members", "hash", "user")
            .await
            .expect("owner user should be created");
        let invited_id = state
            .db
            .create_user_with_role("invited_non_member_manage_members", "hash", "user")
            .await
            .expect("invited user should be created");
        let global_admin_id = state
            .db
            .create_user_with_role("global_admin_non_member_manage_members", "hash", "admin")
            .await
            .expect("global admin should be created");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "member-management-by-global-admin", None, owner_id)
            .await
            .expect("server should be created");

        // L'admin global envia la invitació (pendent) i la verifiquem
        let invite_result = invite_server_member(
            State(state.clone()),
            axum::Extension(claims_for_admin(global_admin_id, "global_admin_non_member_manage_members")),
            Path(server_id),
            Json(InviteMemberRequest {
                username: "invited_non_member_manage_members".to_string(),
            }),
        )
        .await
        .expect("global admin should send invitation without membership");

        assert_eq!(invite_result.0, StatusCode::CREATED);

        // Afegim l'usuari directament (simula acceptació) per poder testar la gestió de rols
        state
            .db
            .add_server_member(server_id, invited_id, "member")
            .await
            .expect("should add invited user as member after accept");

        let promoted = update_member_role(
            State(state.clone()),
            axum::Extension(claims_for_admin(global_admin_id, "global_admin_non_member_manage_members")),
            Path((server_id, invited_id)),
            Json(UpdateMemberRoleRequest {
                role: ServerRole::Admin,
            }),
        )
        .await
        .expect("global admin should update member role without membership");

        assert_eq!(promoted.0.user_id, invited_id);
        assert_eq!(promoted.0.role, ServerRole::Admin);

        let removed = remove_server_member(
            State(state.clone()),
            axum::Extension(claims_for_admin(global_admin_id, "global_admin_non_member_manage_members")),
            Path((server_id, invited_id)),
        )
        .await
        .expect("global admin should remove member without membership");

        assert_eq!(removed.0.user_id, invited_id);
        assert!(removed.0.removed);

        let still_member = state
            .db
            .is_server_member(server_id, invited_id)
            .await
            .expect("membership query should work");
        assert!(still_member.is_none(), "invited user should be removed");
    }

    #[tokio::test]
    async fn leave_server_member_succeeds() {
        let state = make_state().await;
        let owner_id = state.db.create_user_with_role("leave_owner_1", "hash", "user").await.unwrap();
        let member_id = state.db.create_user_with_role("leave_member_1", "hash", "user").await.unwrap();
        let server_id = Uuid::new_v4();
        state.db.create_server_with_owner(server_id, "leave-server-1", None, owner_id).await.unwrap();
        state.db.add_server_member(server_id, member_id, "member").await.unwrap();

        let result = leave_server(
            State(state.clone()),
            axum::Extension(claims_for(member_id, "leave_member_1")),
            Path(server_id),
            Query(LeaveServerParams { force: false }),
        )
        .await;

        assert!(result.is_ok(), "member should be able to leave");
        assert!(result.unwrap().0.left);
        let still_member = state.db.is_server_member(server_id, member_id).await.unwrap();
        assert!(still_member.is_none(), "member should no longer be in server");
    }

    #[tokio::test]
    async fn leave_server_owner_is_blocked() {
        let state = make_state().await;
        let owner_id = state.db.create_user_with_role("leave_owner_2", "hash", "user").await.unwrap();
        let server_id = Uuid::new_v4();
        state.db.create_server_with_owner(server_id, "leave-server-2", None, owner_id).await.unwrap();

        let result = leave_server(
            State(state),
            axum::Extension(claims_for(owner_id, "leave_owner_2")),
            Path(server_id),
            Query(LeaveServerParams { force: false }),
        )
        .await;

        let err = result.expect_err("owner should not be able to leave");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn leave_server_last_admin_blocked_without_force() {
        let state = make_state().await;
        let owner_id = state.db.create_user_with_role("leave_owner_3", "hash", "user").await.unwrap();
        let admin_id = state.db.create_user_with_role("leave_admin_3", "hash", "user").await.unwrap();
        let server_id = Uuid::new_v4();
        state.db.create_server_with_owner(server_id, "leave-server-3", None, owner_id).await.unwrap();
        state.db.add_server_member(server_id, admin_id, "admin").await.unwrap();

        let result = leave_server(
            State(state),
            axum::Extension(claims_for(admin_id, "leave_admin_3")),
            Path(server_id),
            Query(LeaveServerParams { force: false }),
        )
        .await;

        let err = result.expect_err("last admin should be warned without force");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn leave_server_last_admin_allowed_with_force() {
        let state = make_state().await;
        let owner_id = state.db.create_user_with_role("leave_owner_4", "hash", "user").await.unwrap();
        let admin_id = state.db.create_user_with_role("leave_admin_4", "hash", "user").await.unwrap();
        let server_id = Uuid::new_v4();
        state.db.create_server_with_owner(server_id, "leave-server-4", None, owner_id).await.unwrap();
        state.db.add_server_member(server_id, admin_id, "admin").await.unwrap();

        let result = leave_server(
            State(state.clone()),
            axum::Extension(claims_for(admin_id, "leave_admin_4")),
            Path(server_id),
            Query(LeaveServerParams { force: true }),
        )
        .await;

        assert!(result.is_ok(), "last admin should be able to leave with force=true");
        let still_member = state.db.is_server_member(server_id, admin_id).await.unwrap();
        assert!(still_member.is_none(), "admin should no longer be in server");
    }

    #[tokio::test]
    async fn leave_server_admin_with_other_admin_succeeds() {
        let state = make_state().await;
        let owner_id = state.db.create_user_with_role("leave_owner_5", "hash", "user").await.unwrap();
        let admin1_id = state.db.create_user_with_role("leave_admin_5a", "hash", "user").await.unwrap();
        let admin2_id = state.db.create_user_with_role("leave_admin_5b", "hash", "user").await.unwrap();
        let server_id = Uuid::new_v4();
        state.db.create_server_with_owner(server_id, "leave-server-5", None, owner_id).await.unwrap();
        state.db.add_server_member(server_id, admin1_id, "admin").await.unwrap();
        state.db.add_server_member(server_id, admin2_id, "admin").await.unwrap();

        let result = leave_server(
            State(state.clone()),
            axum::Extension(claims_for(admin1_id, "leave_admin_5a")),
            Path(server_id),
            Query(LeaveServerParams { force: false }),
        )
        .await;

        assert!(result.is_ok(), "admin with another admin present should be able to leave");
        let still_member = state.db.is_server_member(server_id, admin1_id).await.unwrap();
        assert!(still_member.is_none());
    }

    #[tokio::test]
    async fn leave_server_non_member_returns_not_found() {
        let state = make_state().await;
        let owner_id = state.db.create_user_with_role("leave_owner_6", "hash", "user").await.unwrap();
        let stranger_id = state.db.create_user_with_role("leave_stranger_6", "hash", "user").await.unwrap();
        let server_id = Uuid::new_v4();
        state.db.create_server_with_owner(server_id, "leave-server-6", None, owner_id).await.unwrap();

        let result = leave_server(
            State(state),
            axum::Extension(claims_for(stranger_id, "leave_stranger_6")),
            Path(server_id),
            Query(LeaveServerParams { force: false }),
        )
        .await;

        let err = result.expect_err("non-member should get not found");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}