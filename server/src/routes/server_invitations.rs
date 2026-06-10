//! Endpoints per al flux d'invitació de servidor amb acceptació explícita.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    db::SERVER_PERMISSION_MANAGE_MEMBERS,
    error::AppError,
    middleware::{AppState, AuthClaims},
};
use tracing::info;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInvitationCreated {
    pub invitation_id: Uuid,
    pub server_id: Uuid,
    pub invitee_username: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingServerInvitation {
    pub invitation_id: Uuid,
    pub server_id: Uuid,
    pub server_name: String,
    pub inviter_id: Uuid,
    pub inviter_username: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitationActionResponse {
    pub invitation_id: Uuid,
    pub status: String,
}

pub async fn create_server_invitation(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
    Json(req): Json<crate::routes::servers::InviteMemberRequest>,
) -> Result<(StatusCode, Json<ServerInvitationCreated>), AppError> {
    info!("create_server_invitation: server={} inviter={} invitee_username={}", server_id, claims.user_id, req.username);

    // L'invitant ha de tenir permís de gestió de membres
    if !claims.is_admin {
        let level = state
            .db
            .get_server_permission_level(server_id, claims.user_id)
            .await
            .map_err(AppError::DatabaseError)?
            .unwrap_or(0);
        if level < SERVER_PERMISSION_MANAGE_MEMBERS {
            return Err(AppError::Forbidden);
        }
    }

    let invitee = state
        .db
        .find_user_by_username(&req.username)
        .await
        .map_err(|_| AppError::InternalError)?
        .ok_or(AppError::UserNotFound)?;
    let invitee_id = invitee.0;

    // L'invitat no ha de ser ja membre
    if state.db.is_server_member(server_id, invitee_id).await.map_err(AppError::DatabaseError)?.is_some() {
        return Err(AppError::MemberExists);
    }

    let invitation_id = Uuid::new_v4();
    state
        .db
        .create_server_invitation(invitation_id, server_id, claims.user_id, invitee_id)
        .await
        .map_err(AppError::DatabaseError)?;

    // Notificar l'invitat via Socket.IO
    let invitee_room = format!("user:{}", invitee_id);
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
    if let Err(e) = state.io.to(invitee_room).emit("server-invitation", &invitation_event).await {
        tracing::warn!("Error enviant server-invitation: {:?}", e);
    }

    Ok((StatusCode::CREATED, Json(ServerInvitationCreated {
        invitation_id,
        server_id,
        invitee_username: req.username,
        status: "pending".to_string(),
    })))
}

pub async fn accept_server_invitation(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path((server_id, invitation_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<InvitationActionResponse>, AppError> {
    info!("accept_server_invitation: invitation={} user={}", invitation_id, claims.user_id);

    let inv = state
        .db
        .get_server_invitation(invitation_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::InvitationInvalid)?;

    let (_, inv_server_id, _, invitee_id, status) = inv;

    if inv_server_id != server_id || invitee_id != claims.user_id {
        return Err(AppError::Forbidden);
    }
    if status != "pending" {
        return Err(AppError::InvitationInvalid);
    }

    state
        .db
        .update_server_invitation_status(invitation_id, "accepted")
        .await
        .map_err(AppError::DatabaseError)?;

    state
        .db
        .add_server_member(server_id, claims.user_id, "member")
        .await
        .map_err(AppError::DatabaseError)?;

    // Notificar el servidor que hi ha un nou membre
    let joined_event = serde_json::json!({
        "userId": claims.user_id,
        "username": claims.username,
        "serverId": server_id,
        "reason": "invitation-accepted",
    });
    if let Err(e) = state.io.to(format!("server:{}", server_id)).emit("server-members-updated", &joined_event).await {
        tracing::warn!("Error enviant server-members-updated: {:?}", e);
    }

    Ok(Json(InvitationActionResponse {
        invitation_id,
        status: "accepted".to_string(),
    }))
}

pub async fn decline_server_invitation(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path((server_id, invitation_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<InvitationActionResponse>, AppError> {
    info!("decline_server_invitation: invitation={} user={}", invitation_id, claims.user_id);

    let inv = state
        .db
        .get_server_invitation(invitation_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::InvitationInvalid)?;

    let (_, inv_server_id, _, invitee_id, status) = inv;

    if inv_server_id != server_id || invitee_id != claims.user_id {
        return Err(AppError::Forbidden);
    }
    if status != "pending" {
        return Err(AppError::InvitationInvalid);
    }

    state
        .db
        .update_server_invitation_status(invitation_id, "declined")
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(Json(InvitationActionResponse {
        invitation_id,
        status: "declined".to_string(),
    }))
}

pub async fn list_my_pending_invitations(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<Vec<PendingServerInvitation>>, AppError> {
    let rows = state
        .db
        .list_pending_server_invitations_for_user(claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let invitations = rows
        .into_iter()
        .map(|(invitation_id, server_id, server_name, inviter_id, inviter_username)| {
            PendingServerInvitation {
                invitation_id,
                server_id,
                server_name,
                inviter_id,
                inviter_username,
            }
        })
        .collect();

    Ok(Json(invitations))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/servers/{server_id}/invitations",
            post(create_server_invitation),
        )
        .route(
            "/api/servers/{server_id}/invitations/{invitation_id}/accept",
            post(accept_server_invitation),
        )
        .route(
            "/api/servers/{server_id}/invitations/{invitation_id}/decline",
            post(decline_server_invitation),
        )
        .route(
            "/api/user/me/server-invitations",
            get(list_my_pending_invitations),
        )
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, LogLevel},
        db::{connect_db, DatabasePool},
        middleware::auth::UserPresenceState,
    };
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use socketioxide::extract::{Data, SocketRef};
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
        io.ns("/", |_s: SocketRef, Data(_): Data<serde_json::Value>| async move {});
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

    async fn setup(state: &AppState, owner_name: &str, invitee_name: &str) -> (Uuid, Uuid, Uuid) {
        let owner_id = state.db.create_user_with_role(owner_name, "hash", "user").await.unwrap();
        let invitee_id = state.db.create_user_with_role(invitee_name, "hash", "user").await.unwrap();
        let server_id = Uuid::new_v4();
        state.db.create_server_with_owner(server_id, &format!("srv-{}", owner_name), None, owner_id).await.unwrap();
        (owner_id, invitee_id, server_id)
    }

    #[tokio::test]
    async fn create_invitation_succeeds_for_owner() {
        let state = make_state().await;
        let (owner_id, _invitee_id, server_id) = setup(&state, "sinv_owner1", "sinv_invitee1").await;

        let result = create_server_invitation(
            State(state),
            axum::Extension(claims_for(owner_id, "sinv_owner1")),
            Path(server_id),
            Json(crate::routes::servers::InviteMemberRequest { username: "sinv_invitee1".to_string() }),
        ).await;

        assert!(result.is_ok(), "owner should be able to create invitation");
        assert_eq!(result.unwrap().0, StatusCode::CREATED);
    }

    #[tokio::test]
    async fn create_invitation_fails_for_non_member() {
        let state = make_state().await;
        let (_owner_id, _invitee_id, server_id) = setup(&state, "sinv_owner2", "sinv_invitee2").await;
        let stranger_id = state.db.create_user_with_role("sinv_stranger2", "hash", "user").await.unwrap();

        let result = create_server_invitation(
            State(state),
            axum::Extension(claims_for(stranger_id, "sinv_stranger2")),
            Path(server_id),
            Json(crate::routes::servers::InviteMemberRequest { username: "sinv_invitee2".to_string() }),
        ).await;

        let err = result.expect_err("non-member should be forbidden");
        assert_eq!(err.into_response().status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn create_invitation_fails_if_already_member() {
        let state = make_state().await;
        let (owner_id, invitee_id, server_id) = setup(&state, "sinv_owner3", "sinv_invitee3").await;
        state.db.add_server_member(server_id, invitee_id, "member").await.unwrap();

        let result = create_server_invitation(
            State(state),
            axum::Extension(claims_for(owner_id, "sinv_owner3")),
            Path(server_id),
            Json(crate::routes::servers::InviteMemberRequest { username: "sinv_invitee3".to_string() }),
        ).await;

        let err = result.expect_err("already member should be rejected");
        assert_eq!(err.into_response().status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn accept_invitation_adds_member() {
        let state = make_state().await;
        let (owner_id, invitee_id, server_id) = setup(&state, "sinv_owner4", "sinv_invitee4").await;

        // Crear invitació
        let inv_result = create_server_invitation(
            State(state.clone()),
            axum::Extension(claims_for(owner_id, "sinv_owner4")),
            Path(server_id),
            Json(crate::routes::servers::InviteMemberRequest { username: "sinv_invitee4".to_string() }),
        ).await.unwrap();
        let invitation_id = inv_result.1.0.invitation_id;

        // Acceptar
        let accept_result = accept_server_invitation(
            State(state.clone()),
            axum::Extension(claims_for(invitee_id, "sinv_invitee4")),
            Path((server_id, invitation_id)),
        ).await;

        assert!(accept_result.is_ok(), "invitee should be able to accept");
        assert_eq!(accept_result.unwrap().0.status, "accepted");

        // Verificar que és membre
        let role = state.db.is_server_member(server_id, invitee_id).await.unwrap();
        assert!(role.is_some(), "invitee should now be a member");
    }

    #[tokio::test]
    async fn decline_invitation_does_not_add_member() {
        let state = make_state().await;
        let (owner_id, invitee_id, server_id) = setup(&state, "sinv_owner5", "sinv_invitee5").await;

        let inv_result = create_server_invitation(
            State(state.clone()),
            axum::Extension(claims_for(owner_id, "sinv_owner5")),
            Path(server_id),
            Json(crate::routes::servers::InviteMemberRequest { username: "sinv_invitee5".to_string() }),
        ).await.unwrap();
        let invitation_id = inv_result.1.0.invitation_id;

        let decline_result = decline_server_invitation(
            State(state.clone()),
            axum::Extension(claims_for(invitee_id, "sinv_invitee5")),
            Path((server_id, invitation_id)),
        ).await;

        assert!(decline_result.is_ok());
        assert_eq!(decline_result.unwrap().0.status, "declined");

        let role = state.db.is_server_member(server_id, invitee_id).await.unwrap();
        assert!(role.is_none(), "declined invitee should NOT be a member");
    }

    #[tokio::test]
    async fn accept_invitation_wrong_user_is_forbidden() {
        let state = make_state().await;
        let (owner_id, _invitee_id, server_id) = setup(&state, "sinv_owner6", "sinv_invitee6").await;
        let stranger_id = state.db.create_user_with_role("sinv_stranger6", "hash", "user").await.unwrap();

        let inv_result = create_server_invitation(
            State(state.clone()),
            axum::Extension(claims_for(owner_id, "sinv_owner6")),
            Path(server_id),
            Json(crate::routes::servers::InviteMemberRequest { username: "sinv_invitee6".to_string() }),
        ).await.unwrap();
        let invitation_id = inv_result.1.0.invitation_id;

        // El stranger intenta acceptar la invitació d'un altre
        let result = accept_server_invitation(
            State(state),
            axum::Extension(claims_for(stranger_id, "sinv_stranger6")),
            Path((server_id, invitation_id)),
        ).await;

        let err = result.expect_err("stranger should not accept another's invitation");
        assert_eq!(err.into_response().status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn list_pending_invitations_for_user() {
        let state = make_state().await;
        let (owner_id, invitee_id, server_id) = setup(&state, "sinv_owner7", "sinv_invitee7").await;

        create_server_invitation(
            State(state.clone()),
            axum::Extension(claims_for(owner_id, "sinv_owner7")),
            Path(server_id),
            Json(crate::routes::servers::InviteMemberRequest { username: "sinv_invitee7".to_string() }),
        ).await.unwrap();

        let result = list_my_pending_invitations(
            State(state),
            axum::Extension(claims_for(invitee_id, "sinv_invitee7")),
        ).await;

        assert!(result.is_ok());
        let list = result.unwrap().0;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].server_id, server_id);
    }
}
