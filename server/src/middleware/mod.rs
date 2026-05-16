//! Middleware d'autenticació i estat compartit.

#[allow(dead_code)]
pub mod auth;

#[allow(dead_code)]
pub use auth::{AppState, AuthClaims, generate_token, generate_claims, extract_claims};
