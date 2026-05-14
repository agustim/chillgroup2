//! Endpoints de servidors.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing::{get, post, delete},
    Router, extract::Path,
};
use shared::types::{ServerInfo, ServerFullInfo};
use serde::Deserialize;
use uuid::Uuid;
use crate::{
    middleware::{AppState, AuthClaims},
    error::AppError,
    models::{Server, ServerMember, ChannelType, EncryptionType},
};

#[derive(Debug, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub icon_url: Option<String>,
}

pub async fn list_servers(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<Vec<ServerInfo>>, AppError> {
    // TODO: Query DB per obtenir servidors de l'usuari
    // SELECT s.*, sm.role FROM servers s JOIN server_members sm ON s.id = sm.server_id WHERE sm.user_id = $1

    Ok(Json(vec![]))
}

pub async fn create_server(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<CreateServerRequest>,
) -> Result<(StatusCode, Json<ServerFullInfo>), AppError> {
    // TODO: Validar nom (1-100 chars)
    // TODO: Comprovar límit de servidors
    // TODO: INSERT server + INSERT server_member (owner)

    let server_id = Uuid::new_v4();
    let now = chrono::Utc::now();

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
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
) -> Result<Json<ServerFullInfo>, AppError> {
    // TODO: Query DB per obtenir servidor + membres
    // SELECT * FROM servers WHERE id = $1
    // SELECT * FROM server_members WHERE server_id = $2

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
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(server_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    // TODO: Comprovar que l'usuari és owner
    // DELETE FROM servers WHERE id = $1

    Ok(StatusCode::OK)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/servers", get(list_servers).post(create_server))
        .route("/api/servers/:server_id", get(get_server).delete(delete_server))
        .with_state(state)
}