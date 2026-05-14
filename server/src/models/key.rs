//! Model ChannelKey.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use super::EncryptionType;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChannelKey {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub device_id: Uuid,
    pub encrypted_key: String,   // Base64
    pub kem_ciphertext: String,  // Base64
    pub encryption_type: EncryptionType,
    pub created_at: DateTime<Utc>,
}