use super::*;
use sqlx::Row;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use crate::models::{Message, message::MessageReaction};

impl DatabasePool {
    // ── Message CRUD ──────────────────────────────────────────────

    pub async fn create_message(
        &self,
        message_id: Uuid,
        channel_id: Uuid,
        sender_user_id: Uuid,
        sender_username: &str,
        sender_device_id: Uuid,
        payload: &str,
        iv: &str,
        key_version: Option<i32>,
        expires_at: Option<DateTime<Utc>>,
        timestamp: DateTime<Utc>,
        reply_to_message_id: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO messages \
                     (id, channel_id, sender_user_id, sender_username, sender_device_id, \
                      encrypted_payload, iv, key_version, timestamp, expires_at, edited_at, deleted_at, reply_to_message_id) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::timestamptz, $10::timestamptz, NULL, NULL, $11)",
                )
                .bind(message_id)
                .bind(channel_id)
                .bind(sender_user_id)
                .bind(sender_username)
                .bind(sender_device_id)
                .bind(payload)
                .bind(iv)
                .bind(key_version)
                .bind(timestamp.to_rfc3339())
                .bind(expires_at.map(|d| d.to_rfc3339()))
                .bind(reply_to_message_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO messages \
                     (id, channel_id, sender_user_id, sender_username, sender_device_id, \
                      encrypted_payload, iv, key_version, timestamp, expires_at, edited_at, deleted_at, reply_to_message_id) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL, ?)",
                )
                .bind(message_id)
                .bind(channel_id)
                .bind(sender_user_id)
                .bind(sender_username)
                .bind(sender_device_id)
                .bind(payload)
                .bind(iv)
                .bind(key_version)
                .bind(timestamp.to_rfc3339())
                .bind(expires_at.map(|d| d.to_rfc3339()))
                .bind(reply_to_message_id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    async fn get_message_attachment_ids(
        &self,
        message_id: Uuid,
    ) -> Result<Vec<Uuid>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT attachment_id FROM message_attachments WHERE message_id = $1 ORDER BY attachment_id ASC",
                )
                .bind(message_id)
                .fetch_all(pool)
                .await?;

                Ok(rows.into_iter().map(|row| row.get(0)).collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT attachment_id FROM message_attachments WHERE message_id = ? ORDER BY attachment_id ASC",
                )
                .bind(message_id)
                .fetch_all(pool)
                .await?;

                Ok(rows.into_iter().map(|row| row.get(0)).collect())
            }
        }
    }

    pub async fn get_message(&self, message_id: Uuid) -> Result<Option<Message>, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, channel_id, sender_user_id, sender_username, sender_device_id, \
                     encrypted_payload, iv, key_version, timestamp::text, expires_at::text, edited_at::text, deleted_at::text, \
                     reply_to_message_id \
                     FROM messages WHERE id = $1 AND (expires_at IS NULL OR expires_at > $2::timestamptz)",
                )
                    .bind(message_id)
                    .bind(&now)
                    .fetch_optional(pool)
                    .await?;
                if let Some(row) = row {
                    let id: Uuid = row.get(0);
                    let attachment_ids = self.get_message_attachment_ids(id).await?;
                    let reactions = self.get_reactions_for_message(id).await?;
                    Ok(Some(Message {
                        id,
                        channel_id: row.get(1),
                        sender_user_id: row.get(2),
                        sender_username: row.get(3),
                        sender_device_id: row.get(4),
                        encrypted_payload: row.get(5),
                        iv: row.get(6),
                        attachment_ids,
                        key_version: row.get(7),
                        timestamp: parse_datetime_required(&row.get::<String, _>(8)),
                        expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                        edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                        deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
                        reply_to_message_id: row.get(12),
                        reactions,
                    }))
                } else {
                    Ok(None)
                }
            }
            DatabasePool::Sqlite(pool) => {
                let query = "SELECT id, channel_id, sender_user_id, sender_username, sender_device_id, \
                     encrypted_payload, iv, key_version, timestamp, expires_at, edited_at, deleted_at, \
                     reply_to_message_id \
                     FROM messages WHERE id = $1 AND (expires_at IS NULL OR expires_at > $2)";
                let query = query.replace("$1", "?");
                let row = sqlx::query(&query)
                    .bind(message_id)
                    .bind(&now)
                    .fetch_optional(pool)
                    .await?;
                if let Some(row) = row {
                    let id: Uuid = row.get(0);
                    let attachment_ids = self.get_message_attachment_ids(id).await?;
                    let reactions = self.get_reactions_for_message(id).await?;
                    Ok(Some(Message {
                        id,
                        channel_id: row.get(1),
                        sender_user_id: row.get(2),
                        sender_username: row.get(3),
                        sender_device_id: row.get(4),
                        encrypted_payload: row.get(5),
                        iv: row.get(6),
                        attachment_ids,
                        key_version: row.get(7),
                        timestamp: parse_datetime_required(&row.get::<String, _>(8)),
                        expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                        edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                        deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
                        reply_to_message_id: row.get(12),
                        reactions,
                    }))
                } else {
                    Ok(None)
                }
            }
        }
    }

    pub async fn list_messages(
        &self,
        channel_id: Uuid,
        limit: usize,
        after: Option<Uuid>,
        before: Option<Uuid>,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<Message>, sqlx::Error> {
        let mut msgs = Vec::new();
        let now = chrono::Utc::now().to_rfc3339();

        match self {
            DatabasePool::Postgres(pool) => {
                // Build WHERE clauses with proper $N placeholder numbering
                let mut conditions = vec![];

                conditions.push(format!("channel_id = $1"));
                conditions.push(format!("(expires_at IS NULL OR expires_at > $2::timestamptz)"));

                if let Some(a) = after {
                    // Timestamp subquery: get messages AFTER the anchor (oldest-first for unread UX)
                    conditions.push("timestamp > (SELECT timestamp FROM messages WHERE id = $3)".to_string());
                    let _ = a;
                }
                if let Some(b) = before {
                    let before_param = if after.is_some() { 4 } else { 3 };
                    // Timestamp subquery: get messages BEFORE the anchor (newest-first for scroll-up)
                    conditions.push(format!(
                        "timestamp < (SELECT timestamp FROM messages WHERE id = ${})",
                        before_param
                    ));
                    let _ = b;
                }
                if let Some(s) = since {
                    let since_param = 3 + usize::from(after.is_some()) + usize::from(before.is_some());
                    conditions.push(format!("timestamp > ${}::timestamptz", since_param));
                    let _ = s;
                }

                conditions.push("deleted_at IS NULL".to_string());

                // after → ASC (oldest unread first, user reads top-to-bottom from divider)
                // before or no cursor → DESC (newest messages first, frontend sorts ASC for display)
                let order = if after.is_some() {
                    "ORDER BY timestamp ASC, id ASC"
                } else {
                    "ORDER BY timestamp DESC, id DESC"
                };

                let query = format!(
                    "SELECT id, channel_id, sender_user_id, sender_username, sender_device_id, \
                     encrypted_payload, iv, key_version, timestamp::text, expires_at::text, edited_at::text, deleted_at::text, \
                     reply_to_message_id \
                     FROM messages WHERE {} {} LIMIT ${}",
                    conditions.join(" AND "),
                    order,
                    3 + usize::from(after.is_some()) + usize::from(before.is_some()) + usize::from(since.is_some())
                );

                let mut q = sqlx::query(&query);
                q = q.bind(channel_id);
                q = q.bind(&now);
                if let Some(a) = after { q = q.bind(a); }
                if let Some(b) = before { q = q.bind(b); }
                if let Some(s) = since { q = q.bind(s.to_rfc3339()); }
                q = q.bind((limit + 1) as i32);

                let rows = q.fetch_all(pool).await?;
                let mut msg_ids: Vec<Uuid> = Vec::new();
                for row in &rows {
                    msg_ids.push(row.get(0));
                }
                let reactions_map = self.get_reactions_for_messages(&msg_ids).await?;
                for row in rows {
                    let id: Uuid = row.get(0);
                    let attachment_ids = self.get_message_attachment_ids(id).await?;
                    let reactions = reactions_map.get(&id).cloned().unwrap_or_default();
                    msgs.push(Message {
                        id,
                        channel_id: row.get(1),
                        sender_user_id: row.get(2),
                        sender_username: row.get(3),
                        sender_device_id: row.get(4),
                        encrypted_payload: row.get(5),
                        iv: row.get(6),
                        attachment_ids,
                        key_version: row.get(7),
                        timestamp: parse_datetime_required(&row.get::<String, _>(8)),
                        expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                        edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                        deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
                        reply_to_message_id: row.get(12),
                        reactions,
                    });
                }
            }
            DatabasePool::Sqlite(pool) => {
                let mut conditions = vec!["channel_id = ?".to_string(), "(expires_at IS NULL OR expires_at > ?)".to_string()];

                if let Some(a) = after {
                    conditions.push("timestamp > (SELECT timestamp FROM messages WHERE id = ?)".to_string());
                    let _ = a;
                }
                if let Some(b) = before {
                    conditions.push("timestamp < (SELECT timestamp FROM messages WHERE id = ?)".to_string());
                    let _ = b;
                }
                if let Some(s) = since {
                    conditions.push("timestamp > ?".to_string());
                    let _ = s;
                }

                conditions.push("deleted_at IS NULL".to_string());

                let order = if after.is_some() {
                    "ORDER BY timestamp ASC, id ASC"
                } else {
                    "ORDER BY timestamp DESC, id DESC"
                };

                let query = format!(
                    "SELECT id, channel_id, sender_user_id, sender_username, sender_device_id, \
                     encrypted_payload, iv, key_version, timestamp, expires_at, edited_at, deleted_at, \
                     reply_to_message_id \
                     FROM messages WHERE {} {} LIMIT ?",
                    conditions.join(" AND "),
                    order
                );

                let mut q = sqlx::query(&query);
                q = q.bind(channel_id);
                q = q.bind(&now);
                if let Some(a) = after { q = q.bind(a); }
                if let Some(b) = before { q = q.bind(b); }
                if let Some(s) = since { q = q.bind(s.to_rfc3339()); }
                q = q.bind((limit + 1) as i32);

                let rows = q.fetch_all(pool).await?;
                let mut msg_ids: Vec<Uuid> = Vec::new();
                for row in &rows {
                    msg_ids.push(row.get(0));
                }
                let reactions_map = self.get_reactions_for_messages(&msg_ids).await?;
                for row in rows {
                    let id: Uuid = row.get(0);
                    let attachment_ids = self.get_message_attachment_ids(id).await?;
                    let reactions = reactions_map.get(&id).cloned().unwrap_or_default();
                    msgs.push(Message {
                        id,
                        channel_id: row.get(1),
                        sender_user_id: row.get(2),
                        sender_username: row.get(3),
                        sender_device_id: row.get(4),
                        encrypted_payload: row.get(5),
                        iv: row.get(6),
                        attachment_ids,
                        key_version: row.get(7),
                        timestamp: parse_datetime_required(&row.get::<String, _>(8)),
                        expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                        edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                        deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
                        reply_to_message_id: row.get(12),
                        reactions,
                    });
                }
            }
        }

        let has_more = msgs.len() > limit;
        if has_more {
            msgs.truncate(limit);
        }

        Ok(msgs)
    }

    pub async fn update_message(
        &self,
        message_id: Uuid,
        payload: &str,
        iv: &str,
        edited_at: DateTime<Utc>,
    ) -> Result<Message, sqlx::Error> {
        let query = "UPDATE messages \
                     SET encrypted_payload = $1, iv = $2, edited_at = $3::timestamptz \
                     WHERE id = $4 \
                     RETURNING id, channel_id, sender_user_id, sender_username, sender_device_id, \
                               encrypted_payload, iv, key_version, timestamp::text, expires_at::text, edited_at::text, deleted_at::text, \
                               reply_to_message_id";
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(query)
                    .bind(payload)
                    .bind(iv)
                    .bind(edited_at.to_rfc3339())
                    .bind(message_id)
                    .fetch_one(pool)
                    .await?;
                let attachment_ids = self.get_message_attachment_ids(message_id).await?;
                let reactions = self.get_reactions_for_message(message_id).await?;
                Ok(Message {
                    id: row.get(0),
                    channel_id: row.get(1),
                    sender_user_id: row.get(2),
                    sender_username: row.get(3),
                    sender_device_id: row.get(4),
                    encrypted_payload: row.get(5),
                    iv: row.get(6),
                    attachment_ids,
                    key_version: row.get(7),
                    timestamp: parse_datetime_required(&row.get::<String, _>(8)),
                    expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                    edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                    deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
                    reply_to_message_id: row.get(12),
                    reactions,
                })
            }
            DatabasePool::Sqlite(pool) => {
                let sqlite_query = "UPDATE messages SET encrypted_payload = ?, iv = ?, edited_at = ? \
                                    WHERE id = ? \
                                    RETURNING id, channel_id, sender_user_id, sender_username, sender_device_id, \
                                              encrypted_payload, iv, key_version, timestamp, expires_at, edited_at, deleted_at, \
                                              reply_to_message_id";
                let row = sqlx::query(sqlite_query)
                    .bind(payload)
                    .bind(iv)
                    .bind(edited_at.to_rfc3339())
                    .bind(message_id)
                    .fetch_one(pool)
                    .await?;
                let attachment_ids = self.get_message_attachment_ids(message_id).await?;
                let reactions = self.get_reactions_for_message(message_id).await?;
                Ok(Message {
                    id: row.get(0),
                    channel_id: row.get(1),
                    sender_user_id: row.get(2),
                    sender_username: row.get(3),
                    sender_device_id: row.get(4),
                    encrypted_payload: row.get(5),
                    iv: row.get(6),
                    attachment_ids,
                    key_version: row.get(7),
                    timestamp: parse_datetime_required(&row.get::<String, _>(8)),
                    expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                    edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                    deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
                    reply_to_message_id: row.get(12),
                    reactions,
                })
            }
        }
    }

    pub async fn update_message_expiry(
        &self,
        message_id: Uuid,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<Message, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let query = "UPDATE messages SET expires_at = $1::timestamptz WHERE id = $2 \
                             RETURNING id, channel_id, sender_user_id, sender_username, sender_device_id, \
                                       encrypted_payload, iv, key_version, timestamp::text, expires_at::text, edited_at::text, deleted_at::text, \
                                       reply_to_message_id";
                let row = sqlx::query(query)
                    .bind(expires_at.map(|d| d.to_rfc3339()))
                    .bind(message_id)
                    .fetch_one(pool)
                    .await?;
                let attachment_ids = self.get_message_attachment_ids(message_id).await?;
                let reactions = self.get_reactions_for_message(message_id).await?;
                Ok(Message {
                    id: row.get(0),
                    channel_id: row.get(1),
                    sender_user_id: row.get(2),
                    sender_username: row.get(3),
                    sender_device_id: row.get(4),
                    encrypted_payload: row.get(5),
                    iv: row.get(6),
                    attachment_ids,
                    key_version: row.get(7),
                    timestamp: parse_datetime_required(&row.get::<String, _>(8)),
                    expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                    edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                    deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
                    reply_to_message_id: row.get(12),
                    reactions,
                })
            }
            DatabasePool::Sqlite(pool) => {
                let query = "UPDATE messages SET expires_at = ? WHERE id = ? \
                             RETURNING id, channel_id, sender_user_id, sender_username, sender_device_id, \
                                       encrypted_payload, iv, key_version, timestamp, expires_at, edited_at, deleted_at, \
                                       reply_to_message_id";
                let row = sqlx::query(query)
                    .bind(expires_at.map(|d| d.to_rfc3339()))
                    .bind(message_id)
                    .fetch_one(pool)
                    .await?;
                let attachment_ids = self.get_message_attachment_ids(message_id).await?;
                let reactions = self.get_reactions_for_message(message_id).await?;
                Ok(Message {
                    id: row.get(0),
                    channel_id: row.get(1),
                    sender_user_id: row.get(2),
                    sender_username: row.get(3),
                    sender_device_id: row.get(4),
                    encrypted_payload: row.get(5),
                    iv: row.get(6),
                    attachment_ids,
                    key_version: row.get(7),
                    timestamp: parse_datetime_required(&row.get::<String, _>(8)),
                    expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                    edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                    deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
                    reply_to_message_id: row.get(12),
                    reactions,
                })
            }
        }
    }

    pub async fn get_reactions_for_message(&self, message_id: Uuid) -> Result<Vec<MessageReaction>, sqlx::Error> {
        self.get_reactions_for_messages(&[message_id])
            .await
            .map(|mut m| m.remove(&message_id).unwrap_or_default())
    }

    pub async fn get_reactions_for_messages(&self, message_ids: &[Uuid]) -> Result<HashMap<Uuid, Vec<MessageReaction>>, sqlx::Error> {
        if message_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let mut result: HashMap<Uuid, HashMap<String, (Vec<Uuid>, Vec<String>)>> = HashMap::new();

        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT message_id, emoji, user_id, username FROM message_reactions \
                     WHERE message_id = ANY($1) ORDER BY message_id, emoji, created_at",
                )
                .bind(message_ids)
                .fetch_all(pool)
                .await?;
                for row in rows {
                    let msg_id: Uuid = row.get(0);
                    let emoji: String = row.get(1);
                    let user_id: Uuid = row.get(2);
                    let username: String = row.get(3);
                    let entry = result.entry(msg_id).or_default().entry(emoji).or_default();
                    entry.0.push(user_id);
                    entry.1.push(username);
                }
            }
            DatabasePool::Sqlite(pool) => {
                for &msg_id in message_ids {
                    let rows = sqlx::query(
                        "SELECT message_id, emoji, user_id, username FROM message_reactions \
                         WHERE message_id = ? ORDER BY emoji, created_at",
                    )
                    .bind(msg_id)
                    .fetch_all(pool)
                    .await?;
                    for row in rows {
                        let mid: Uuid = row.get(0);
                        let emoji: String = row.get(1);
                        let user_id: Uuid = row.get(2);
                        let username: String = row.get(3);
                        let entry = result.entry(mid).or_default().entry(emoji).or_default();
                        entry.0.push(user_id);
                        entry.1.push(username);
                    }
                }
            }
        }

        Ok(result.into_iter().map(|(msg_id, emoji_map)| {
            let reactions = emoji_map.into_iter().map(|(emoji, (user_ids, usernames))| {
                MessageReaction {
                    count: user_ids.len() as i64,
                    emoji,
                    user_ids,
                    usernames,
                }
            }).collect();
            (msg_id, reactions)
        }).collect())
    }

    pub async fn add_reaction(&self, message_id: Uuid, user_id: Uuid, username: &str, emoji: &str) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO message_reactions (message_id, user_id, username, emoji) \
                     VALUES ($1, $2, $3, $4) ON CONFLICT (message_id, user_id, emoji) DO NOTHING",
                )
                .bind(message_id)
                .bind(user_id)
                .bind(username)
                .bind(emoji)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT OR IGNORE INTO message_reactions (message_id, user_id, username, emoji) \
                     VALUES (?, ?, ?, ?)",
                )
                .bind(message_id)
                .bind(user_id)
                .bind(username)
                .bind(emoji)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn remove_reaction(&self, message_id: Uuid, user_id: Uuid, emoji: &str) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "DELETE FROM message_reactions WHERE message_id = $1 AND user_id = $2 AND emoji = $3",
                )
                .bind(message_id)
                .bind(user_id)
                .bind(emoji)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "DELETE FROM message_reactions WHERE message_id = ? AND user_id = ? AND emoji = ?",
                )
                .bind(message_id)
                .bind(user_id)
                .bind(emoji)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn delete_message(&self, message_id: Uuid) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE messages SET deleted_at = NOW() WHERE id = $1",
                )
                .bind(message_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE messages SET deleted_at = ? WHERE id = ?",
                )
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(message_id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    /// Esborra físicament els missatges expirats i retorna els (message_id, channel_id) eliminats
    /// perquè el servei pugui notificar els clients connectats.
    pub async fn delete_expired_messages(&self) -> Result<(Vec<(Uuid, Uuid)>, Vec<String>), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut deleted = Vec::new();
        let mut object_keys: Vec<String> = Vec::new();
        match self {
            DatabasePool::Postgres(pool) => {
                // Recollir object_keys d'attachments (i thumbnails) vinculats als missatges que expiren
                let att_rows = sqlx::query(
                    "SELECT a.object_key, ta.object_key \
                     FROM message_attachments ma \
                     JOIN attachments a ON a.id = ma.attachment_id \
                     LEFT JOIN attachments ta ON ta.id = a.thumbnail_attachment_id \
                     WHERE ma.message_id IN (SELECT id FROM messages WHERE expires_at IS NOT NULL AND expires_at <= $1::timestamptz)",
                )
                .bind(&now)
                .fetch_all(pool)
                .await?;
                for row in &att_rows {
                    object_keys.push(row.get::<String, _>(0));
                    if let Some(thumb_key) = row.get::<Option<String>, _>(1) {
                        object_keys.push(thumb_key);
                    }
                }

                // Recollir thumbnail IDs ABANS de nullejar (el FK ja no existirà després)
                let thumbnail_ids: Vec<Uuid> = sqlx::query_scalar(
                    "SELECT a.thumbnail_attachment_id \
                     FROM message_attachments ma \
                     JOIN attachments a ON a.id = ma.attachment_id \
                     WHERE ma.message_id IN (SELECT id FROM messages WHERE expires_at IS NOT NULL AND expires_at <= $1::timestamptz) \
                     AND a.thumbnail_attachment_id IS NOT NULL",
                )
                .bind(&now)
                .fetch_all(pool)
                .await?;

                // Trencar la referència thumbnail_attachment_id→attachments(id) abans d'esborrar
                sqlx::query(
                    "UPDATE attachments SET thumbnail_attachment_id = NULL \
                     WHERE id IN ( \
                       SELECT ma.attachment_id FROM message_attachments ma \
                       WHERE ma.message_id IN (SELECT id FROM messages WHERE expires_at IS NOT NULL AND expires_at <= $1::timestamptz) \
                     )",
                )
                .bind(&now)
                .execute(pool)
                .await?;

                // Esborrar thumbnails per ID (ja sense referenciadors)
                if !thumbnail_ids.is_empty() {
                    sqlx::query("DELETE FROM attachments WHERE id = ANY($1)")
                        .bind(&thumbnail_ids as &[Uuid])
                        .execute(pool)
                        .await?;
                }

                // Esborrar attachments principals
                sqlx::query(
                    "DELETE FROM attachments WHERE id IN ( \
                       SELECT ma.attachment_id FROM message_attachments ma \
                       WHERE ma.message_id IN (SELECT id FROM messages WHERE expires_at IS NOT NULL AND expires_at <= $1::timestamptz) \
                     )",
                )
                .bind(&now)
                .execute(pool)
                .await?;

                let rows = sqlx::query(
                    "DELETE FROM messages WHERE expires_at IS NOT NULL AND expires_at <= $1::timestamptz \
                     RETURNING id, channel_id",
                )
                .bind(&now)
                .fetch_all(pool)
                .await?;
                for row in rows {
                    deleted.push((row.get::<Uuid, _>(0), row.get::<Uuid, _>(1)));
                }
            }
            DatabasePool::Sqlite(pool) => {
                // SQLite no suporta RETURNING en versions antigues; fem SELECT + DELETE
                let rows = sqlx::query(
                    "SELECT id, channel_id FROM messages \
                     WHERE expires_at IS NOT NULL AND expires_at <= ?",
                )
                .bind(&now)
                .fetch_all(pool)
                .await?;
                for row in &rows {
                    deleted.push((row.get::<Uuid, _>(0), row.get::<Uuid, _>(1)));
                }
                if !deleted.is_empty() {
                    let att_rows = sqlx::query(
                        "SELECT a.object_key, ta.object_key \
                         FROM message_attachments ma \
                         JOIN attachments a ON a.id = ma.attachment_id \
                         LEFT JOIN attachments ta ON ta.id = a.thumbnail_attachment_id \
                         WHERE ma.message_id IN (SELECT id FROM messages WHERE expires_at IS NOT NULL AND expires_at <= ?)",
                    )
                    .bind(&now)
                    .fetch_all(pool)
                    .await?;
                    for row in &att_rows {
                        object_keys.push(row.get::<String, _>(0));
                        if let Some(thumb_key) = row.get::<Option<String>, _>(1) {
                            object_keys.push(thumb_key);
                        }
                    }

                    let sqlite_thumb_ids: Vec<String> = sqlx::query_scalar(
                        "SELECT a.thumbnail_attachment_id \
                         FROM message_attachments ma \
                         JOIN attachments a ON a.id = ma.attachment_id \
                         WHERE ma.message_id IN (SELECT id FROM messages WHERE expires_at IS NOT NULL AND expires_at <= ?) \
                         AND a.thumbnail_attachment_id IS NOT NULL",
                    )
                    .bind(&now)
                    .fetch_all(pool)
                    .await?;

                    sqlx::query(
                        "UPDATE attachments SET thumbnail_attachment_id = NULL \
                         WHERE id IN ( \
                           SELECT ma.attachment_id FROM message_attachments ma \
                           WHERE ma.message_id IN (SELECT id FROM messages WHERE expires_at IS NOT NULL AND expires_at <= ?) \
                         )",
                    )
                    .bind(&now)
                    .execute(pool)
                    .await?;

                    for tid in &sqlite_thumb_ids {
                        sqlx::query("DELETE FROM attachments WHERE id = ?")
                            .bind(tid)
                            .execute(pool)
                            .await?;
                    }

                    sqlx::query(
                        "DELETE FROM attachments WHERE id IN ( \
                           SELECT ma.attachment_id FROM message_attachments ma \
                           WHERE ma.message_id IN (SELECT id FROM messages WHERE expires_at IS NOT NULL AND expires_at <= ?) \
                         )",
                    )
                    .bind(&now)
                    .execute(pool)
                    .await?;

                    sqlx::query(
                        "DELETE FROM messages WHERE expires_at IS NOT NULL AND expires_at <= ?",
                    )
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
        }
        Ok((deleted, object_keys))
    }

    pub async fn count_new_messages(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        since: &str,
    ) -> Result<(usize, Option<Uuid>), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT COUNT(*), MIN(id) FROM messages \
                     WHERE channel_id = $1 AND sender_user_id != $2 AND deleted_at IS NULL \
                     AND (expires_at IS NULL OR expires_at > $4::timestamptz) \
                     AND timestamp > $3::timestamptz",
                )
                .bind(channel_id)
                .bind(user_id)
                .bind(since)
                .bind(&now)
                .fetch_one(pool)
                .await?;
                let count: i64 = row.get(0);
                let first_id: Option<Uuid> = row.get(1);
                Ok((count as usize, first_id))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT COUNT(*), MIN(id) FROM messages \
                     WHERE channel_id = ? AND sender_user_id != ? AND deleted_at IS NULL \
                     AND (expires_at IS NULL OR expires_at > ?) \
                     AND timestamp > ?",
                )
                .bind(channel_id)
                .bind(user_id)
                .bind(&now)
                .bind(since)
                .fetch_one(pool)
                .await?;
                let count: i64 = row.get(0);
                let first_id: Option<Uuid> = row.get(1);
                Ok((count as usize, first_id))
            }
        }
    }

    pub async fn mark_channel_read(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        last_read_message_id: Option<Uuid>,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO channel_read_state (user_id, channel_id, last_read_message_id, last_read_at, updated_at) \
                     VALUES ($1, $2, $3, NOW(), NOW()) \
                     ON CONFLICT (user_id, channel_id) DO UPDATE SET \
                     last_read_message_id = EXCLUDED.last_read_message_id, \
                     last_read_at = EXCLUDED.last_read_at, \
                     updated_at = EXCLUDED.updated_at",
                )
                .bind(user_id)
                .bind(channel_id)
                .bind(last_read_message_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO channel_read_state (user_id, channel_id, last_read_message_id, last_read_at, updated_at) \
                     VALUES (?, ?, ?, ?, ?) \
                     ON CONFLICT(user_id, channel_id) DO UPDATE SET \
                     last_read_message_id = excluded.last_read_message_id, \
                     last_read_at = excluded.last_read_at, \
                     updated_at = excluded.updated_at",
                )
                .bind(user_id)
                .bind(channel_id)
                .bind(last_read_message_id)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

}
