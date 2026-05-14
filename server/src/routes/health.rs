//! Health check endpoint.

use axum::{
    extract::State,
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Serialize;
use crate::middleware::AppState;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub uptime_seconds: u64,
}

static START_TIME: std::sync::LazyLock<std::time::Instant> = std::sync::LazyLock::new(std::time::Instant::now);

pub async fn health_check(
    State(state): State<AppState>,
) -> (StatusCode, Json<HealthResponse>) {
    // Verificar connexió a DB
    let db_status = match state.db.acquire().await {
        Ok(_) => "connected",
        Err(_) => "error",
    };

    let uptime = START_TIME.elapsed().as_secs();

    let status = if db_status == "connected" {
        "healthy"
    } else {
        "degraded"
    };

    (
        StatusCode::OK,
        Json(HealthResponse {
            status: status.to_string(),
            database: db_status.to_string(),
            uptime_seconds: uptime,
        }),
    )
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .with_state(state)
}