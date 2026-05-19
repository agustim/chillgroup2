//! Model Message.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub sender_user_id: Uuid,
    pub sender_username: Option<String>,
    pub sender_device_id: Uuid,
    pub encrypted_payload: String, // Base64 AES-GCM
    pub iv: String,                // Base64 nonce
    pub timestamp: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}
