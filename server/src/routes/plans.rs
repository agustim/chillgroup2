//! Endpoints de plans/tiers.

use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    error::AppError,
    middleware::AppState,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItem {
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
    pub max_storage_bytes: i64,
    pub max_transfer_bytes_monthly: i64,
}

pub async fn list_plans(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let plans = state
        .db
        .list_plans_admin()
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let data: Vec<PlanItem> = plans
        .into_iter()
        .map(|(
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
            max_storage_bytes,
            max_transfer_bytes_monthly,
        )| PlanItem {
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
            max_storage_bytes,
            max_transfer_bytes_monthly,
        })
        .collect();

    Ok(Json(serde_json::json!({
        "success": true,
        "data": data,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, LogLevel},
        db::connect_db,
        middleware::auth::UserPresenceState,
    };
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
        }
    }

    #[tokio::test]
    async fn list_plans_returns_default_plans() {
        let state = make_state().await;

        let result = list_plans(State(state)).await.unwrap();

        let plans = result.0["data"].as_array().unwrap();
        assert!(!plans.is_empty(), "should have at least the default plans seeded");
        let names: Vec<&str> = plans.iter()
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        assert!(names.contains(&"free"), "free plan should exist");
    }

    #[tokio::test]
    async fn list_plans_success_flag_is_true() {
        let state = make_state().await;

        let result = list_plans(State(state)).await.unwrap();

        assert_eq!(result.0["success"], true);
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/plans", get(list_plans))
        .with_state(state)
}
