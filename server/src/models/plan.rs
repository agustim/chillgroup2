//! Model Plan.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Plan {
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
    pub created_at: DateTime<Utc>,
}
