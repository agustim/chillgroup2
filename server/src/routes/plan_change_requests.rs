//! Endpoints per a sol·licituds de canvi de pla.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::AppError,
    middleware::{AppState, AuthClaims},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePlanChangeRequest {
    pub requested_plan_id: Uuid,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanChangeRequestItem {
    pub id: Uuid,
    pub user_id: Uuid,
    pub username: String,
    pub requested_plan_id: Uuid,
    pub requested_plan_name: String,
    pub status: String,
    pub message: Option<String>,
    pub admin_note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveRequest {
    pub admin_note: Option<String>,
}

pub async fn create_plan_change_request(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<CreatePlanChangeRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let plan_exists = state
        .db
        .plan_exists_by_id(req.requested_plan_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    if !plan_exists {
        return Err(AppError::BadRequest);
    }

    // Bloqueja si ja hi ha una sol·licitud pendent
    let existing = state
        .db
        .get_pending_plan_change_request_for_user(claims.user_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    if existing.is_some() {
        return Err(AppError::BadRequest);
    }

    let request_id = state
        .db
        .create_plan_change_request(
            claims.user_id,
            req.requested_plan_id,
            req.message.as_deref(),
        )
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    // Notificar tots els admins via Socket.IO
    let admin_ids = state
        .db
        .get_admin_user_ids()
        .await
        .unwrap_or_default();

    let username = state
        .db
        .find_username_by_user_id(claims.user_id)
        .await
        .ok()
        .flatten()
        .unwrap_or_default();

    let payload = serde_json::json!({
        "requestId": request_id,
        "userId": claims.user_id,
        "username": username,
        "requestedPlanId": req.requested_plan_id,
        "message": req.message,
    });

    for admin_id in admin_ids {
        let room = format!("user:{}", admin_id);
        let _ = state.io.to(room).emit("plan_change_request", &payload).await;
    }

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "id": request_id })),
    ))
}

pub async fn list_plan_change_requests(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let rows = state
        .db
        .list_plan_change_requests_admin()
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let items: Vec<PlanChangeRequestItem> = rows
        .into_iter()
        .map(|(id, user_id, username, plan_id, plan_name, status, message, admin_note, created_at)| {
            PlanChangeRequestItem {
                id,
                user_id,
                username,
                requested_plan_id: plan_id,
                requested_plan_name: plan_name,
                status,
                message,
                admin_note,
                created_at,
            }
        })
        .collect();

    Ok(Json(serde_json::json!(items)))
}

pub async fn approve_plan_change_request(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(request_id): Path<Uuid>,
    Json(req): Json<ResolveRequest>,
) -> Result<StatusCode, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let result = state
        .db
        .resolve_plan_change_request(request_id, "approved", req.admin_note.as_deref())
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let (user_id, plan_id) = result.ok_or(AppError::BadRequest)?;

    // Aplica el canvi de pla
    state
        .db
        .set_user_plan_by_id(user_id, plan_id)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    // Notifica l'usuari
    let room = format!("user:{}", user_id);
    let _ = state
        .io
        .to(room)
        .emit(
            "plan_change_resolved",
            &serde_json::json!({
                "status": "approved",
                "requestId": request_id,
                "adminNote": req.admin_note,
            }),
        )
        .await;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn reject_plan_change_request(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(request_id): Path<Uuid>,
    Json(req): Json<ResolveRequest>,
) -> Result<StatusCode, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let result = state
        .db
        .resolve_plan_change_request(request_id, "rejected", req.admin_note.as_deref())
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let (user_id, _) = result.ok_or(AppError::BadRequest)?;

    let room = format!("user:{}", user_id);
    let _ = state
        .io
        .to(room)
        .emit(
            "plan_change_resolved",
            &serde_json::json!({
                "status": "rejected",
                "requestId": request_id,
                "adminNote": req.admin_note,
            }),
        )
        .await;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, LogLevel},
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

    fn admin_claims_for(user_id: Uuid, username: &str) -> AuthClaims {
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

    async fn get_pro_plan_id(state: &AppState) -> Uuid {
        let plans = state.db.list_plans_admin().await.unwrap();
        plans.iter()
            .find(|(_, name, ..)| name == "pro")
            .map(|(id, ..)| *id)
            .expect("pro plan should exist")
    }

    #[tokio::test]
    async fn create_plan_change_request_succeeds() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("pcr_user", "hash", "user").await.unwrap();
        let plan_id = get_pro_plan_id(&state).await;

        let result = create_plan_change_request(
            State(state),
            axum::Extension(claims_for(user_id, "pcr_user")),
            Json(CreatePlanChangeRequest { requested_plan_id: plan_id, message: None }),
        )
        .await;

        assert!(result.is_ok());
        let (status, body) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert!(body.0["id"].is_string());
    }

    #[tokio::test]
    async fn create_plan_change_request_fails_for_invalid_plan() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("pcr_badplan", "hash", "user").await.unwrap();

        let result = create_plan_change_request(
            State(state),
            axum::Extension(claims_for(user_id, "pcr_badplan")),
            Json(CreatePlanChangeRequest { requested_plan_id: Uuid::new_v4(), message: None }),
        )
        .await;

        let err = result.expect_err("invalid plan should fail");
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn create_plan_change_request_blocks_duplicate_pending() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("pcr_dup", "hash", "user").await.unwrap();
        let plan_id = get_pro_plan_id(&state).await;

        create_plan_change_request(
            State(state.clone()),
            axum::Extension(claims_for(user_id, "pcr_dup")),
            Json(CreatePlanChangeRequest { requested_plan_id: plan_id, message: None }),
        )
        .await
        .unwrap();

        let result = create_plan_change_request(
            State(state),
            axum::Extension(claims_for(user_id, "pcr_dup")),
            Json(CreatePlanChangeRequest { requested_plan_id: plan_id, message: None }),
        )
        .await;

        let err = result.expect_err("duplicate pending request should fail");
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn list_plan_change_requests_requires_admin() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("pcr_nonadmin", "hash", "user").await.unwrap();

        let result = list_plan_change_requests(
            State(state),
            axum::Extension(claims_for(user_id, "pcr_nonadmin")),
        )
        .await;

        let err = result.expect_err("non-admin should be forbidden");
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn approve_plan_change_request_succeeds() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("pcr_approve_user", "hash", "user").await.unwrap();
        let admin_id = state.db.create_user_with_role("pcr_approve_admin", "hash", "admin").await.unwrap();
        let plan_id = get_pro_plan_id(&state).await;

        let (_, body) = create_plan_change_request(
            State(state.clone()),
            axum::Extension(claims_for(user_id, "pcr_approve_user")),
            Json(CreatePlanChangeRequest { requested_plan_id: plan_id, message: None }),
        )
        .await
        .unwrap();

        let request_id: Uuid = serde_json::from_value(body.0["id"].clone()).unwrap();

        let result = approve_plan_change_request(
            State(state),
            axum::Extension(admin_claims_for(admin_id, "pcr_approve_admin")),
            Path(request_id),
            Json(ResolveRequest { admin_note: None }),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn reject_plan_change_request_succeeds() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("pcr_reject_user", "hash", "user").await.unwrap();
        let admin_id = state.db.create_user_with_role("pcr_reject_admin", "hash", "admin").await.unwrap();
        let plan_id = get_pro_plan_id(&state).await;

        let (_, body) = create_plan_change_request(
            State(state.clone()),
            axum::Extension(claims_for(user_id, "pcr_reject_user")),
            Json(CreatePlanChangeRequest { requested_plan_id: plan_id, message: None }),
        )
        .await
        .unwrap();

        let request_id: Uuid = serde_json::from_value(body.0["id"].clone()).unwrap();

        let result = reject_plan_change_request(
            State(state),
            axum::Extension(admin_claims_for(admin_id, "pcr_reject_admin")),
            Path(request_id),
            Json(ResolveRequest { admin_note: Some("Not eligible".to_string()) }),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/user/me/plan-change-request",
            post(create_plan_change_request),
        )
        .route(
            "/api/admin/plan-change-requests",
            get(list_plan_change_requests),
        )
        .route(
            "/api/admin/plan-change-requests/{request_id}/approve",
            put(approve_plan_change_request),
        )
        .route(
            "/api/admin/plan-change-requests/{request_id}/reject",
            put(reject_plan_change_request),
        )
        .with_state(state)
}
