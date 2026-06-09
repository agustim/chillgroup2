use super::*;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::Attachment;

impl DatabasePool {
    // ── Attachments CRUD ──────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn create_attachment_init(
        &self,
        attachment_id: Uuid,
        channel_id: Uuid,
        uploader_user_id: Uuid,
        uploader_device_id: Uuid,
        file_name: &str,
        mime_type: &str,
        size_bytes: i64,
        created_at: DateTime<Utc>,
        object_key: &str,
        upload_id: &str,
        chunk_size_bytes: i64,
        chunk_count: i32,
    ) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO attachments \
                     (id, channel_id, uploader_user_id, uploader_device_id, file_name, mime_type, size_bytes, \
                      created_at, object_key, status, upload_id, chunk_size_bytes, chunk_count) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8::timestamptz, $9, 'initiated', $10, $11, $12)",
                )
                .bind(attachment_id)
                .bind(channel_id)
                .bind(uploader_user_id)
                .bind(uploader_device_id)
                .bind(file_name)
                .bind(mime_type)
                .bind(size_bytes)
                .bind(created_at.to_rfc3339())
                .bind(object_key)
                .bind(upload_id)
                .bind(chunk_size_bytes)
                .bind(chunk_count)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO attachments \
                     (id, channel_id, uploader_user_id, uploader_device_id, file_name, mime_type, size_bytes, \
                      created_at, object_key, status, upload_id, chunk_size_bytes, chunk_count) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'initiated', ?, ?, ?)",
                )
                .bind(attachment_id)
                .bind(channel_id)
                .bind(uploader_user_id)
                .bind(uploader_device_id)
                .bind(file_name)
                .bind(mime_type)
                .bind(size_bytes)
                .bind(created_at.to_rfc3339())
                .bind(object_key)
                .bind(upload_id)
                .bind(chunk_size_bytes)
                .bind(chunk_count)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_attachment_by_id(
        &self,
        channel_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<Option<Attachment>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, channel_id, uploader_user_id, uploader_device_id, file_name, mime_type, size_bytes, \
                        created_at::text, object_key, status, upload_id, chunk_size_bytes, chunk_count, algorithm, \
                        file_iv, wrapped_file_key, key_version_id, key_version, ciphertext_sha256, completed_at::text, \
                        thumbnail_attachment_id \
                     FROM attachments WHERE channel_id = $1 AND id = $2",
                )
                .bind(channel_id)
                .bind(attachment_id)
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|row| Attachment {
                    id: row.get(0),
                    channel_id: row.get(1),
                    uploader_user_id: row.get(2),
                    uploader_device_id: row.get(3),
                    file_name: row.get(4),
                    mime_type: row.get(5),
                    size_bytes: row.get(6),
                    created_at: parse_datetime_required(&row.get::<String, _>(7)),
                    object_key: row.get(8),
                    status: row.get(9),
                    upload_id: row.get(10),
                    chunk_size_bytes: row.get(11),
                    chunk_count: row.get(12),
                    algorithm: row.get(13),
                    file_iv: row.get(14),
                    wrapped_file_key: row.get(15),
                    key_version_id: row.get(16),
                    key_version: row.get(17),
                    ciphertext_sha256: row.get(18),
                    completed_at: parse_datetime_utc(&row.get::<Option<String>, _>(19)),
                    thumbnail_attachment_id: row.get(20),
                }))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, channel_id, uploader_user_id, uploader_device_id, file_name, mime_type, size_bytes, \
                        created_at, object_key, status, upload_id, chunk_size_bytes, chunk_count, algorithm, \
                        file_iv, wrapped_file_key, key_version_id, key_version, ciphertext_sha256, completed_at, \
                        thumbnail_attachment_id \
                     FROM attachments WHERE channel_id = ? AND id = ?",
                )
                .bind(channel_id)
                .bind(attachment_id)
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|row| Attachment {
                    id: row.get(0),
                    channel_id: row.get(1),
                    uploader_user_id: row.get(2),
                    uploader_device_id: row.get(3),
                    file_name: row.get(4),
                    mime_type: row.get(5),
                    size_bytes: row.get(6),
                    created_at: parse_datetime_required(&row.get::<String, _>(7)),
                    object_key: row.get(8),
                    status: row.get(9),
                    upload_id: row.get(10),
                    chunk_size_bytes: row.get(11),
                    chunk_count: row.get(12),
                    algorithm: row.get(13),
                    file_iv: row.get(14),
                    wrapped_file_key: row.get(15),
                    key_version_id: row.get(16),
                    key_version: row.get(17),
                    ciphertext_sha256: row.get(18),
                    completed_at: parse_datetime_utc(&row.get::<Option<String>, _>(19)),
                    thumbnail_attachment_id: row.get(20),
                }))
            }
        }
    }

    pub async fn complete_attachment(
        &self,
        attachment_id: Uuid,
        algorithm: &str,
        file_iv: &str,
        wrapped_file_key: &str,
        key_version_id: Uuid,
        key_version: i32,
        ciphertext_sha256: &str,
        thumbnail_attachment_id: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        let completed_at = Utc::now().to_rfc3339();
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE attachments \
                     SET status = 'ready', algorithm = $1, file_iv = $2, wrapped_file_key = $3, \
                         key_version_id = $4, key_version = $5, ciphertext_sha256 = $6, completed_at = $7::timestamptz, \
                         thumbnail_attachment_id = $9 \
                     WHERE id = $8",
                )
                .bind(algorithm)
                .bind(file_iv)
                .bind(wrapped_file_key)
                .bind(key_version_id)
                .bind(key_version)
                .bind(ciphertext_sha256)
                .bind(&completed_at)
                .bind(attachment_id)
                .bind(thumbnail_attachment_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE attachments \
                     SET status = 'ready', algorithm = ?, file_iv = ?, wrapped_file_key = ?, \
                         key_version_id = ?, key_version = ?, ciphertext_sha256 = ?, completed_at = ?, \
                         thumbnail_attachment_id = ? \
                     WHERE id = ?",
                )
                .bind(algorithm)
                .bind(file_iv)
                .bind(wrapped_file_key)
                .bind(key_version_id)
                .bind(key_version)
                .bind(ciphertext_sha256)
                .bind(completed_at)
                .bind(thumbnail_attachment_id)
                .bind(attachment_id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn attach_message_attachments(
        &self,
        message_id: Uuid,
        channel_id: Uuid,
        uploader_user_id: Uuid,
        attachment_ids: &[Uuid],
    ) -> Result<(), sqlx::Error> {
        if attachment_ids.is_empty() {
            return Ok(());
        }

        match self {
            DatabasePool::Postgres(pool) => {
                let mut tx = pool.begin().await?;
                for attachment_id in attachment_ids {
                    let updated = sqlx::query(
                        "UPDATE attachments \
                         SET status = 'linked' \
                         WHERE id = $1 AND channel_id = $2 AND uploader_user_id = $3 AND status = 'ready'",
                    )
                    .bind(attachment_id)
                    .bind(channel_id)
                    .bind(uploader_user_id)
                    .execute(&mut *tx)
                    .await?;

                    if updated.rows_affected() == 0 {
                        return Err(sqlx::Error::RowNotFound);
                    }

                    sqlx::query(
                        "INSERT INTO message_attachments (message_id, attachment_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                    )
                    .bind(message_id)
                    .bind(attachment_id)
                    .execute(&mut *tx)
                    .await?;
                }
                tx.commit().await?;
            }
            DatabasePool::Sqlite(pool) => {
                let mut tx = pool.begin().await?;
                for attachment_id in attachment_ids {
                    let updated = sqlx::query(
                        "UPDATE attachments \
                         SET status = 'linked' \
                         WHERE id = ? AND channel_id = ? AND uploader_user_id = ? AND status = 'ready'",
                    )
                    .bind(attachment_id)
                    .bind(channel_id)
                    .bind(uploader_user_id)
                    .execute(&mut *tx)
                    .await?;

                    if updated.rows_affected() == 0 {
                        return Err(sqlx::Error::RowNotFound);
                    }

                    sqlx::query(
                        "INSERT OR IGNORE INTO message_attachments (message_id, attachment_id) VALUES (?, ?)",
                    )
                    .bind(message_id)
                    .bind(attachment_id)
                    .execute(&mut *tx)
                    .await?;
                }
                tx.commit().await?;
            }
        }
        Ok(())
    }

}
