use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Attachment {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub uploader_user_id: Uuid,
    pub uploader_device_id: Uuid,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub created_at: DateTime<Utc>,
    pub object_key: String,
    pub status: String,
    pub upload_id: String,
    pub chunk_size_bytes: i64,
    pub chunk_count: i32,
    pub algorithm: Option<String>,
    pub file_iv: Option<String>,
    pub wrapped_file_key: Option<String>,
    pub key_version_id: Option<Uuid>,
    pub key_version: Option<i32>,
    pub ciphertext_sha256: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub thumbnail_attachment_id: Option<Uuid>,
}
