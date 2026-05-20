//! Model Channel i tipus relacionats.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Text,
    Voice,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionType {
    None,
    Symmetric,
    Asymmetric,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Channel {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    #[sqlx(rename = "type")]
    pub channel_type: ChannelType,
    pub encryption_type: EncryptionType,
    pub message_ttl: Option<i32>,
    pub is_private: bool,
    #[serde(default)]
    pub unread_count: usize,
    pub created_at: DateTime<Utc>,
}