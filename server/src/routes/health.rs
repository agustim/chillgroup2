//! Endpoint de health check.

#![allow(dead_code)]

use axum::{
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

pub async fn health_check() -> (StatusCode, Json<HealthResponse>) {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok".to_string(),
            version: "2.0.0".to_string(),
        }),
    )
}

pub fn router(state: crate::middleware::AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .with_state(state)
}