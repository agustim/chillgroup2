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

    #[tokio::test]
    async fn list_friends_returns_empty_initially() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("fr_list_user", "hash", "user").await.unwrap();

        let result = list_my_friends(
            State(state),
            axum::Extension(claims_for(user_id, "fr_list_user")),
        )
        .await
        .unwrap();

        let json = result.0;
        assert_eq!(json["success"], true);
        assert_eq!(json["data"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn add_friend_succeeds() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("fr_owner", "hash", "user").await.unwrap();
        state.db.create_user_with_role("fr_friend", "hash", "user").await.unwrap();

        let result = add_friend(
            State(state),
            axum::Extension(claims_for(user_id, "fr_owner")),
            Json(AddFriendRequest { username: "fr_friend".to_string() }),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn add_friend_fails_for_self() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("fr_self", "hash", "user").await.unwrap();

        let result = add_friend(
            State(state),
            axum::Extension(claims_for(user_id, "fr_self")),
            Json(AddFriendRequest { username: "fr_self".to_string() }),
        )
        .await;

        let err = result.expect_err("should fail adding self");
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn add_friend_fails_for_nonexistent_user() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("fr_noexist_owner", "hash", "user").await.unwrap();

        let result = add_friend(
            State(state),
            axum::Extension(claims_for(user_id, "fr_noexist_owner")),
            Json(AddFriendRequest { username: "nonexistent_user_xyz".to_string() }),
        )
        .await;

        let err = result.expect_err("should fail for nonexistent user");
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn remove_friend_succeeds() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("fr_rm_owner", "hash", "user").await.unwrap();
        let friend_id = state.db.create_user_with_role("fr_rm_friend", "hash", "user").await.unwrap();
        state.db.add_friend_for_user(user_id, friend_id).await.unwrap();

        let result = remove_friend(
            State(state),
            axum::Extension(claims_for(user_id, "fr_rm_owner")),
            Path(friend_id),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn list_friends_shows_added_friend() {
        let state = make_state().await;
        let user_id = state.db.create_user_with_role("fr_show_owner", "hash", "user").await.unwrap();
        let friend_id = state.db.create_user_with_role("fr_show_friend", "hash", "user").await.unwrap();
        state.db.add_friend_for_user(user_id, friend_id).await.unwrap();

        let result = list_my_friends(
            State(state),
            axum::Extension(claims_for(user_id, "fr_show_owner")),
        )
        .await
        .unwrap();

        let friends = result.0["data"].as_array().unwrap();
        assert_eq!(friends.len(), 1);
        assert_eq!(friends[0]["username"], "fr_show_friend");
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/friends", axum::routing::get(list_my_friends))
        .route("/api/friends", axum::routing::post(add_friend))
        .route("/api/friends/{friend_user_id}", axum::routing::delete(remove_friend))
        .with_state(state)
}