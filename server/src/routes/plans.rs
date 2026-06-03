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

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/plans", get(list_plans))
        .with_state(state)
}
