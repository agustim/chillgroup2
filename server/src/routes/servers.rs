//! Endpoints de servidors.

#![allow(dead_code)]

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing::{get, post},
    Router, extract::Path,
};
use shared::types::{ServerInfo, ServerFullInfo};
use serde::Deserialize;
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

pub async fn list_servers(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<Vec<ServerInfo>>, AppError> {
    info!("Endpoint list_servers cridat per user_id={}", claims.user_id);
    Ok(Json(vec![]))
}

pub async fn create_server(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<CreateServerRequest>,
) -> Result<(StatusCode, Json<ServerFullInfo>), AppError> {
    info!("Endpoint create_server cridat per user_id={}, name={}", claims.user_id, req.name);
    let server_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    info!("Servidor creat amb èxit: server_id={}", server_id);
    Ok((
        StatusCode::CREATED,
        Json(ServerFullInfo {
            server_id,
            name: req.name,
            icon_url: req.icon_url,
            owner_id: claims.user_id,
            members: vec![],
            created_at: now.to_rfc3339(),
        }),
    ))
}

pub async fn get_server(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<ServerFullInfo>, AppError> {
    info!("Endpoint get_server cridat: server_id={}, user_id={}", server_id, claims.user_id);
    Ok(Json(ServerFullInfo {
        server_id,
        name: "Test Server".to_string(),
        icon_url: None,
        owner_id: claims.user_id,
        members: vec![],
        created_at: chrono::Utc::now().to_rfc3339(),
    }))
}

pub async fn delete_server(
    State(_state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    info!("Endpoint delete_server cridat: server_id={}, user_id={}", server_id, claims.user_id);
    Ok(StatusCode::OK)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/servers", get(list_servers).post(create_server))
        .route("/api/servers/{server_id}", get(get_server).delete(delete_server))
        .with_state(state)
}