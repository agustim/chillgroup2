//! Model Invitation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invitation {
    pub id: Uuid,
    pub code: String,
    pub created_by_user_id: Uuid,
    pub max_uses: i32,
    pub uses_count: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
