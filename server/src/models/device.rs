//! Model Device.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Device {
    pub id: Uuid,
    pub user_id: Uuid,
    pub label: Option<String>,
    pub public_key: String, // Base64 encoded Kyber-1024
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub revoked: bool,
}