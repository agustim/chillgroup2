//! Routes de l'API REST.

pub mod auth;
pub mod servers;
pub mod channels;
pub mod messages;
pub mod livekit;
pub mod health;

pub use auth::register;
pub use auth::login;