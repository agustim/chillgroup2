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
