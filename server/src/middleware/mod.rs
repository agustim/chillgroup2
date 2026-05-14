//! Middleware d'autenticació i estat compartit.

pub mod auth;

pub use auth::{AppState, AuthClaims, extract_claims, generate_token, generate_claims};