//! Rutes de gestió d'amics — `/api/friends`

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
    Router,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

use crate::{
    error::AppError,
    middleware::{AppState, AuthClaims},
};

#[derive(Debug, Deserialize)]
pub struct AddFriendRequest {
    pub username: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendResponse {
    pub user_id: Uuid,
    pub username: String,
    pub status: String,
}

fn presence_status(user_id: Uuid, state: &AppState) -> String {
    if state
        .user_presence
        .try_read()
        .map(|presence| presence.online_sockets.contains_key(&user_id))
        .unwrap_or(false)
    {
        "online".to_string()
    } else {
        "offline".to_string()
    }
}

#[axum::debug_handler]
pub async fn list_my_friends(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
) -> Result<Json<serde_json::Value>, AppError> {
    info!("📋 Endpoint /api/friends cridat per user_id={}", claims.user_id);

    let friends = state
        .db
        .list_friends_for_user(claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    let data: Vec<FriendResponse> = friends
        .into_iter()
        .map(|(user_id, username)| FriendResponse {
            user_id,
            username,
            status: presence_status(user_id, &state),
        })
        .collect();

    Ok(Json(serde_json::json!({
        "success": true,
        "data": data,
    })))
}

#[axum::debug_handler]
pub async fn add_friend(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Json(req): Json<AddFriendRequest>,
) -> Result<StatusCode, AppError> {
    let username = req.username.trim();
    if username.is_empty() {
        return Err(AppError::BadRequest);
    }

    let Some((friend_user_id, friend_username, _)) = state
        .db
        .find_user_by_username(username)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?
    else {
        return Err(AppError::UserNotFound);
    };

    if friend_user_id == claims.user_id {
        return Err(AppError::BadRequest);
    }

    state
        .db
        .add_friend_for_user(claims.user_id, friend_user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    info!("Amic afegit: owner_user_id={}, friend_user_id={}, friend_username={}", claims.user_id, friend_user_id, friend_username);

    Ok(StatusCode::NO_CONTENT)
}

#[axum::debug_handler]
pub async fn remove_friend(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(friend_user_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    state
        .db
        .remove_friend_for_user(claims.user_id, friend_user_id)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(StatusCode::NO_CONTENT)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/friends", axum::routing::get(list_my_friends))
        .route("/api/friends", axum::routing::post(add_friend))
        .route("/api/friends/{friend_user_id}", axum::routing::delete(remove_friend))
        .with_state(state)
}