//! Middleware d'autenticació i estat compartit.

#[allow(dead_code)]
pub mod auth;
pub mod rate_limit;

#[allow(dead_code)]
pub use auth::{AppState, AuthClaims, generate_token, generate_claims, extract_claims};
pub use rate_limit::{RateLimiter, rate_limit_middleware};

use axum::{
    http::Request,
    middleware::Next,
    response::Response,
    body::Body,
};

/// Middleware que insereix AppState a les extensions de la request,
/// perquè `extract_claims` el pugui trobar.
pub async fn insert_state(
    state: axum::extract::State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    req.extensions_mut().insert(state.0);
    Ok(next.run(req).await)
}

use crate::error::AppError;
