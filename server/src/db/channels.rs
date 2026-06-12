use super::*;
use sqlx::Row;
use uuid::Uuid;
use crate::models::{Channel, ChannelType, EncryptionType};

impl DatabasePool {
        pub async fn list_channels_for_server(&self, server_id: Uuid, user_id: Uuid) -> Result<Vec<Channel>, sqlx::Error> {
                let query = "SELECT c.id, c.server_id, c.name, c.channel_type AS channel_type, c.encryption_type, c.message_ttl, c.is_private, c.created_at::text, \
                                         CASE \
                                             WHEN COALESCE(c.scope, 'server') = 'dm' THEN CASE WHEN cm.user_id IS NOT NULL THEN 3 ELSE 0 END \
                                             WHEN cm.user_id IS NOT NULL THEN cm.permission_level \
                                             WHEN sm.role IN ('owner', 'admin') THEN 3 \
                                             ELSE 2 \
                                         END AS permission_level, \
                                         rs.last_read_message_id \
                                         FROM channels c \
                                         LEFT JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = $2 \
                                         LEFT JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = $2 \
                                         LEFT JOIN channel_read_state rs ON rs.channel_id = c.id AND rs.user_id = $2 \
                                         WHERE c.server_id = $1 \
                                             AND (c.is_private = false OR cm.user_id IS NOT NULL) \
                                         ORDER BY c.position ASC, c.channel_type ASC";
        let mut channels = Vec::new();
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(query)
                    .bind(server_id)
                    .bind(user_id)
                    .fetch_all(pool)
                    .await?;
                for row in rows {
                    let channel_type_str: String = row.get(3);
                    let channel_type = match channel_type_str.as_str() {
                        "voice" => ChannelType::Voice,
                        _ => ChannelType::Text,
                    };
                    let encryption_str: String = row.get(4);
                    let encryption_type = match encryption_str.as_str() {
                        "symmetric" => EncryptionType::Symmetric,
                        "asymmetric" => EncryptionType::Asymmetric,
                        _ => EncryptionType::None,
                    };
                    let is_private: bool = row.get(6);
                    let created_at_str: String = row.get(7);
                    let created_at = parse_datetime_required(&created_at_str);
                    let channel_id: Uuid = row.get(0);
                    let (key_version_id, key_version) = self
                        .get_channel_key_version_metadata(channel_id)
                        .await?
                        .map(|(id, version)| (Some(id), Some(version)))
                        .unwrap_or((None, None));
                    channels.push(Channel {
                        id: channel_id,
                        server_id: row.get(1),
                        name: row.get(2),
                        channel_type,
                        encryption_type,
                        message_ttl: row.get(5),
                        is_private,
                        permission_level: Some(row.get::<i32, _>(8)),
                        unread_count: 0,
                        key_version_id,
                        key_version,
                        last_read_message_id: row.get(9),
                        created_at,
                    });
                }
            }
            DatabasePool::Sqlite(pool) => {
                                let query = "SELECT c.id, c.server_id, c.name, c.type AS channel_type, c.encryption_type, c.message_ttl, c.is_private, c.created_at, \
                                                         CASE \
                                                             WHEN COALESCE(c.scope, 'server') = 'dm' THEN CASE WHEN cm.user_id IS NOT NULL THEN 3 ELSE 0 END \
                                                             WHEN cm.user_id IS NOT NULL THEN cm.permission_level \
                                                             WHEN sm.role IN ('owner', 'admin') THEN 3 \
                                                             ELSE 2 \
                                                         END AS permission_level, \
                                                         rs.last_read_message_id \
                                                         FROM channels c \
                                                         LEFT JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = ? \
                                                         LEFT JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = ? \
                                                         LEFT JOIN channel_read_state rs ON rs.channel_id = c.id AND rs.user_id = ? \
                                                         WHERE c.server_id = ? \
                                                             AND (c.is_private = 0 OR cm.user_id IS NOT NULL) \
                                                         ORDER BY c.type ASC, c.name ASC";
                let rows = sqlx::query(&query)
                    .bind(user_id)
                    .bind(user_id)
                    .bind(user_id)
                    .bind(server_id)
                    .fetch_all(pool)
                    .await?;
                for row in rows {
                    let channel_type_str: String = row.get(3);
                    let channel_type = match channel_type_str.as_str() {
                        "voice" => ChannelType::Voice,
                        _ => ChannelType::Text,
                    };
                    let encryption_str: String = row.get(4);
                    let encryption_type = match encryption_str.as_str() {
                        "symmetric" => EncryptionType::Symmetric,
                        "asymmetric" => EncryptionType::Asymmetric,
                        _ => EncryptionType::None,
                    };
                    let is_private: i64 = row.get(6);
                    let created_at_str: String = row.get(7);
                    let created_at = parse_datetime_required(&created_at_str);
                    let channel_id: Uuid = row.get(0);
                    let (key_version_id, key_version) = self
                        .get_channel_key_version_metadata(channel_id)
                        .await?
                        .map(|(id, version)| (Some(id), Some(version)))
                        .unwrap_or((None, None));
                    channels.push(Channel {
                        id: channel_id,
                        server_id: row.get(1),
                        name: row.get(2),
                        channel_type,
                        encryption_type,
                        message_ttl: row.get(5),
                        is_private: is_private != 0,
                        permission_level: Some(row.get::<i32, _>(8)),
                        unread_count: 0,
                        key_version_id,
                        key_version,
                        last_read_message_id: row.get(9),
                        created_at,
                    });
                }
            }
        }

        for channel in &mut channels {
            channel.unread_count = self
                .count_unread_messages_for_user(channel.id, user_id)
                .await
                .unwrap_or(0);
        }

        Ok(channels)
    }

    #[allow(dead_code)]
    pub async fn add_channel_member(&self, channel_id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
        self.add_channel_member_with_permission(channel_id, user_id, CHANNEL_PERMISSION_WRITE)
            .await
    }

    pub async fn add_channel_member_with_permission(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        permission_level: i32,
    ) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();
        let permission_level = permission_level.clamp(CHANNEL_PERMISSION_READ, CHANNEL_PERMISSION_MANAGE);
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO channel_members (id, channel_id, user_id, permission_level, joined_at) VALUES ($1, $2, $3, $4, NOW()) ON CONFLICT (channel_id, user_id) DO NOTHING"
                )
                .bind(id)
                .bind(channel_id)
                .bind(user_id)
                .bind(permission_level)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT OR IGNORE INTO channel_members (id, channel_id, user_id, permission_level, joined_at) VALUES (?, ?, ?, ?, ?)"
                )
                .bind(id)
                .bind(channel_id)
                .bind(user_id)
                .bind(permission_level)
                .bind(now.clone())
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn is_channel_member(&self, channel_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM channel_members WHERE channel_id = $1 AND user_id = $2)")
                    .bind(channel_id)
                    .bind(user_id)
                    .fetch_one(pool)
                    .await?;
                Ok(row.get::<bool, _>(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM channel_members WHERE channel_id = ? AND user_id = ?)")
                    .bind(channel_id)
                    .bind(user_id)
                    .fetch_one(pool)
                    .await?;
                Ok(row.get::<bool, _>(0))
            }
        }
    }

    #[allow(dead_code)]
    pub async fn user_can_access_channel(&self, channel_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        Ok(self
            .get_channel_permission_level(channel_id, user_id)
            .await?
            .unwrap_or(0)
            >= CHANNEL_PERMISSION_READ)
    }

    pub async fn get_channel_permission_level(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<i32>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT \
                        CASE \
                            WHEN COALESCE(c.scope, 'server') = 'dm' THEN CASE WHEN cm.user_id IS NOT NULL THEN 3 ELSE 0 END \
                            WHEN cm.user_id IS NOT NULL THEN cm.permission_level \
                            WHEN sm.user_id IS NULL THEN 0 \
                            WHEN sm.role IN ('owner', 'admin') THEN 3 \
                            ELSE 2 \
                        END AS permission_level \
                     FROM channels c \
                     LEFT JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = $2 \
                     LEFT JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = $2 \
                     WHERE c.id = $1"
                )
                .bind(channel_id)
                .bind(user_id)
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|r| r.get::<i32, _>(0)))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT \
                        CASE \
                            WHEN COALESCE(c.scope, 'server') = 'dm' THEN CASE WHEN cm.user_id IS NOT NULL THEN 3 ELSE 0 END \
                            WHEN cm.user_id IS NOT NULL THEN cm.permission_level \
                            WHEN sm.user_id IS NULL THEN 0 \
                            WHEN sm.role IN ('owner', 'admin') THEN 3 \
                            ELSE 2 \
                        END AS permission_level \
                     FROM channels c \
                     LEFT JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = ? \
                     LEFT JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = ? \
                     WHERE c.id = ?"
                )
                .bind(user_id)
                .bind(user_id)
                .bind(channel_id)
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|r| r.get::<i32, _>(0)))
            }
        }
    }

    pub async fn list_channel_permission_levels(
        &self,
        channel_id: Uuid,
    ) -> Result<Vec<(Uuid, String, i32)>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT
                        u.id,
                        u.username,
                        CASE
                            WHEN cm.user_id IS NOT NULL THEN cm.permission_level
                            WHEN sm.role IN ('owner', 'admin') THEN 3
                            ELSE 2
                        END AS permission_level
                     FROM channels c
                     JOIN server_members sm ON sm.server_id = c.server_id
                     JOIN users u ON u.id = sm.user_id
                     LEFT JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = u.id
                     WHERE c.id = $1
                     ORDER BY u.username ASC"
                )
                .bind(channel_id)
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|row| (row.get(0), row.get(1), row.get(2)))
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT
                        u.id,
                        u.username,
                        CASE
                            WHEN cm.user_id IS NOT NULL THEN cm.permission_level
                            WHEN sm.role IN ('owner', 'admin') THEN 3
                            ELSE 2
                        END AS permission_level
                     FROM channels c
                     JOIN server_members sm ON sm.server_id = c.server_id
                     JOIN users u ON u.id = sm.user_id
                     LEFT JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = u.id
                     WHERE c.id = ?
                     ORDER BY u.username ASC"
                )
                .bind(channel_id)
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|row| (row.get(0), row.get(1), row.get(2)))
                    .collect())
            }
        }
    }

    pub async fn list_explicit_channel_permissions(
        &self,
        channel_id: Uuid,
    ) -> Result<Vec<(Uuid, String, i32)>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT
                        u.id,
                        u.username,
                        cm.permission_level
                     FROM channel_members cm
                     JOIN users u ON u.id = cm.user_id
                     WHERE cm.channel_id = $1
                     ORDER BY cm.permission_level DESC, u.username ASC"
                )
                .bind(channel_id)
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|row| (row.get(0), row.get(1), row.get(2)))
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT
                        u.id,
                        u.username,
                        cm.permission_level
                     FROM channel_members cm
                     JOIN users u ON u.id = cm.user_id
                     WHERE cm.channel_id = ?
                     ORDER BY cm.permission_level DESC, u.username ASC"
                )
                .bind(channel_id)
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|row| (row.get(0), row.get(1), row.get(2)))
                    .collect())
            }
        }
    }

    pub async fn set_explicit_channel_permission(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
        permission_level: i32,
    ) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();
        let permission_level = permission_level.clamp(CHANNEL_PERMISSION_READ, CHANNEL_PERMISSION_MANAGE);

        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO channel_members (id, channel_id, user_id, permission_level, joined_at)
                     VALUES ($1, $2, $3, $4, NOW())
                     ON CONFLICT (channel_id, user_id)
                     DO UPDATE SET permission_level = EXCLUDED.permission_level"
                )
                .bind(id)
                .bind(channel_id)
                .bind(user_id)
                .bind(permission_level)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO channel_members (id, channel_id, user_id, permission_level, joined_at)
                     VALUES (?, ?, ?, ?, ?)
                     ON CONFLICT(channel_id, user_id)
                     DO UPDATE SET permission_level = excluded.permission_level"
                )
                .bind(id)
                .bind(channel_id)
                .bind(user_id)
                .bind(permission_level)
                .bind(now.clone())
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn remove_explicit_channel_permission(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM channel_members WHERE channel_id = $1 AND user_id = $2")
                    .bind(channel_id)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("DELETE FROM channel_members WHERE channel_id = ? AND user_id = ?")
                    .bind(channel_id)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn find_dm_channel_by_users(&self, user_a: Uuid, user_b: Uuid) -> Result<Option<Uuid>, sqlx::Error> {
        let (low, high) = if user_a < user_b { (user_a, user_b) } else { (user_b, user_a) };

        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id FROM channels
                     WHERE scope = 'dm'
                       AND LEAST(dm_user_a_id, dm_user_b_id) = $1
                       AND GREATEST(dm_user_a_id, dm_user_b_id) = $2
                     LIMIT 1",
                )
                .bind(low)
                .bind(high)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| r.get(0)))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id FROM channels
                     WHERE COALESCE(scope, 'server') = 'dm'
                       AND ((dm_user_a_id = ? AND dm_user_b_id = ?) OR (dm_user_a_id = ? AND dm_user_b_id = ?))
                     LIMIT 1",
                )
                .bind(low)
                .bind(high)
                .bind(high)
                .bind(low)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| r.get(0)))
            }
        }
    }

    pub async fn create_dm_channel(
        &self,
        channel_id: Uuid,
        creator_user_id: Uuid,
        target_user_id: Uuid,
        message_ttl: Option<i32>,
    ) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let (dm_user_a_id, dm_user_b_id) = if creator_user_id < target_user_id {
            (creator_user_id, target_user_id)
        } else {
            (target_user_id, creator_user_id)
        };

        let name = format!("dm-{}-{}", dm_user_a_id, dm_user_b_id);

        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO channels (id, server_id, name, channel_type, encryption_type, scope, dm_user_a_id, dm_user_b_id, message_ttl, is_private, created_at)
                     VALUES ($1, NULL, $2, 'text', 'asymmetric', 'dm', $3, $4, $5, true, NOW())",
                )
                .bind(channel_id)
                .bind(name)
                .bind(dm_user_a_id)
                .bind(dm_user_b_id)
                .bind(message_ttl)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO channels (id, server_id, name, type, encryption_type, scope, dm_user_a_id, dm_user_b_id, message_ttl, is_private, created_at)
                     VALUES (?, NULL, ?, 'text', 'asymmetric', 'dm', ?, ?, ?, 1, ?)",
                )
                .bind(channel_id)
                .bind(name)
                .bind(dm_user_a_id)
                .bind(dm_user_b_id)
                .bind(message_ttl)
                .bind(now)
                .execute(pool)
                .await?;
            }
        }

        self
            .add_channel_member_with_permission(channel_id, creator_user_id, CHANNEL_PERMISSION_MANAGE)
            .await?;
        self
            .add_channel_member_with_permission(channel_id, target_user_id, CHANNEL_PERMISSION_MANAGE)
            .await?;

        Ok(())
    }

    pub async fn list_dm_channels_for_user(&self, user_id: Uuid) -> Result<Vec<DirectChannelSummary>, sqlx::Error> {
        let mut rows_out = Vec::new();

        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT
                        c.id AS channel_id,
                        CASE WHEN c.dm_user_a_id = $1 THEN c.dm_user_b_id ELSE c.dm_user_a_id END AS peer_user_id,
                        u.username AS peer_username,
                        c.message_ttl,
                        MAX(m.timestamp)::text AS last_message_at
                     FROM channels c
                     JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = $1
                     JOIN users u ON u.id = CASE WHEN c.dm_user_a_id = $1 THEN c.dm_user_b_id ELSE c.dm_user_a_id END
                     LEFT JOIN messages m ON m.channel_id = c.id AND m.deleted_at IS NULL AND (m.expires_at IS NULL OR m.expires_at > NOW())
                     WHERE c.scope = 'dm'
                     GROUP BY c.id, peer_user_id, u.username, c.message_ttl
                     ORDER BY last_message_at DESC NULLS LAST, c.created_at DESC",
                )
                .bind(user_id)
                .fetch_all(pool)
                .await?;

                for row in rows {
                    rows_out.push(DirectChannelSummary {
                        channel_id: row.get(0),
                        peer_user_id: row.get(1),
                        peer_username: row.get(2),
                        message_ttl: row.get(3),
                        last_message_at: parse_datetime_utc(&row.get::<Option<String>, _>(4)),
                    });
                }
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT
                        c.id AS channel_id,
                        CASE WHEN c.dm_user_a_id = ? THEN c.dm_user_b_id ELSE c.dm_user_a_id END AS peer_user_id,
                        u.username AS peer_username,
                        c.message_ttl,
                        MAX(m.timestamp) AS last_message_at
                     FROM channels c
                     JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = ?
                     JOIN users u ON u.id = CASE WHEN c.dm_user_a_id = ? THEN c.dm_user_b_id ELSE c.dm_user_a_id END
                     LEFT JOIN messages m ON m.channel_id = c.id AND m.deleted_at IS NULL AND (m.expires_at IS NULL OR m.expires_at > datetime('now'))
                     WHERE COALESCE(c.scope, 'server') = 'dm'
                     GROUP BY c.id, peer_user_id, u.username, c.message_ttl, c.created_at
                     ORDER BY last_message_at DESC, c.created_at DESC",
                )
                .bind(user_id)
                .bind(user_id)
                .bind(user_id)
                .fetch_all(pool)
                .await?;

                for row in rows {
                    rows_out.push(DirectChannelSummary {
                        channel_id: row.get(0),
                        peer_user_id: row.get(1),
                        peer_username: row.get(2),
                        message_ttl: row.get(3),
                        last_message_at: parse_datetime_utc(&row.get::<Option<String>, _>(4)),
                    });
                }
            }
        }

        Ok(rows_out)
    }

    pub async fn get_dm_channel_ttl_for_member(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<Option<i32>>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT c.message_ttl
                     FROM channels c
                     JOIN channel_members cm ON cm.channel_id = c.id
                     WHERE c.id = $1 AND c.scope = 'dm' AND cm.user_id = $2
                     LIMIT 1",
                )
                .bind(channel_id)
                .bind(user_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| r.get(0)))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT c.message_ttl
                     FROM channels c
                     JOIN channel_members cm ON cm.channel_id = c.id
                     WHERE c.id = ? AND COALESCE(c.scope, 'server') = 'dm' AND cm.user_id = ?
                     LIMIT 1",
                )
                .bind(channel_id)
                .bind(user_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| r.get(0)))
            }
        }
    }

    pub async fn update_dm_channel_ttl(&self, channel_id: Uuid, message_ttl: Option<i32>) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE channels SET message_ttl = $1 WHERE id = $2 AND scope = 'dm'")
                    .bind(message_ttl)
                    .bind(channel_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("UPDATE channels SET message_ttl = ? WHERE id = ? AND COALESCE(scope, 'server') = 'dm'")
                    .bind(message_ttl)
                    .bind(channel_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn list_server_member_ids(&self, server_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query("SELECT user_id FROM server_members WHERE server_id = $1")
                    .bind(server_id)
                    .fetch_all(pool)
                    .await?;
                Ok(rows.into_iter().map(|r| r.get(0)).collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT user_id FROM server_members WHERE server_id = ?")
                    .bind(server_id)
                    .fetch_all(pool)
                    .await?;
                Ok(rows.into_iter().map(|r| r.get(0)).collect())
            }
        }
    }

    pub async fn list_channel_member_ids(&self, channel_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query("SELECT user_id FROM channel_members WHERE channel_id = $1")
                    .bind(channel_id)
                    .fetch_all(pool)
                    .await?;
                Ok(rows.into_iter().map(|r| r.get(0)).collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT user_id FROM channel_members WHERE channel_id = ?")
                    .bind(channel_id)
                    .fetch_all(pool)
                    .await?;
                Ok(rows.into_iter().map(|r| r.get(0)).collect())
            }
        }
    }

    pub async fn list_server_ids_for_user(&self, user_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query("SELECT server_id FROM server_members WHERE user_id = $1")
                    .bind(user_id)
                    .fetch_all(pool)
                    .await?;
                Ok(rows.into_iter().map(|r| r.get(0)).collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT server_id FROM server_members WHERE user_id = ?")
                    .bind(user_id)
                    .fetch_all(pool)
                    .await?;
                Ok(rows.into_iter().map(|r| r.get(0)).collect())
            }
        }
    }

    pub async fn create_channel(&self, channel_id: Uuid, server_id: Uuid, name: &str, channel_type: &str, encryption_type: &str, message_ttl: Option<i32>, is_private: bool) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO channels (id, server_id, name, channel_type, encryption_type, message_ttl, is_private, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())")
                    .bind(channel_id)
                    .bind(server_id)
                    .bind(name)
                    .bind(channel_type)
                    .bind(encryption_type)
                    .bind(message_ttl)
                    .bind(is_private)
                        .execute(pool)
                        .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("INSERT INTO channels (id, server_id, name, type, encryption_type, message_ttl, is_private, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                    .bind(channel_id)
                    .bind(server_id)
                    .bind(name)
                    .bind(channel_type)
                    .bind(encryption_type)
                    .bind(message_ttl)
                    .bind(is_private as i32)
                    .bind(chrono::Utc::now().to_rfc3339())
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn get_channel(&self, channel_id: Uuid) -> Result<Option<Channel>, sqlx::Error> {
        let query = "SELECT id, server_id, name, type, encryption_type, message_ttl, is_private, created_at FROM channels WHERE id = $1";
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT id, server_id, name, channel_type AS type, encryption_type, message_ttl, is_private, created_at::text FROM channels WHERE id = $1")
                    .bind(channel_id)
                    .fetch_optional(pool)
                    .await?;
                if let Some(row) = row {
                    let channel_type_str: String = row.get(3);
                    let channel_type = match channel_type_str.as_str() {
                        "voice" => ChannelType::Voice,
                        _ => ChannelType::Text,
                    };
                    let encryption_str: String = row.get(4);
                    let encryption_type = match encryption_str.as_str() {
                        "symmetric" => EncryptionType::Symmetric,
                        "asymmetric" => EncryptionType::Asymmetric,
                        _ => EncryptionType::None,
                    };
                    let is_private: bool = row.get(6);
                    let created_at_str: String = row.get(7);
                    let created_at = parse_datetime_required(&created_at_str);
                    let (key_version_id, key_version) = self
                        .get_channel_key_version_metadata(channel_id)
                        .await?
                        .map(|(id, version)| (Some(id), Some(version)))
                        .unwrap_or((None, None));
                    Ok(Some(Channel {
                        id: row.get(0),
                        server_id: row.get::<Option<Uuid>, _>(1).unwrap_or_else(Uuid::nil),
                        name: row.get(2),
                        channel_type,
                        encryption_type,
                        message_ttl: row.get(5),
                        is_private,
                        permission_level: None,
                        unread_count: 0,
                        key_version_id,
                        key_version,
                        last_read_message_id: None,
                        created_at,
                    }))
                } else {
                    Ok(None)
                }
            }
            DatabasePool::Sqlite(pool) => {
                let query = query.replace("$1", "?");
                let row = sqlx::query(&query)
                    .bind(channel_id)
                    .fetch_optional(pool)
                    .await?;
                if let Some(row) = row {
                    let channel_type_str: String = row.get(3);
                    let channel_type = match channel_type_str.as_str() {
                        "voice" => ChannelType::Voice,
                        _ => ChannelType::Text,
                    };
                    let encryption_str: String = row.get(4);
                    let encryption_type = match encryption_str.as_str() {
                        "symmetric" => EncryptionType::Symmetric,
                        "asymmetric" => EncryptionType::Asymmetric,
                        _ => EncryptionType::None,
                    };
                    let is_private: i64 = row.get(6);
                    let created_at_str: String = row.get(7);
                    let created_at = parse_datetime_required(&created_at_str);
                    let (key_version_id, key_version) = self
                        .get_channel_key_version_metadata(channel_id)
                        .await?
                        .map(|(id, version)| (Some(id), Some(version)))
                        .unwrap_or((None, None));
                    Ok(Some(Channel {
                        id: row.get(0),
                        server_id: row.get::<Option<Uuid>, _>(1).unwrap_or_else(Uuid::nil),
                        name: row.get(2),
                        channel_type,
                        encryption_type,
                        message_ttl: row.get(5),
                        is_private: is_private != 0,
                        permission_level: None,
                        unread_count: 0,
                        key_version_id,
                        key_version,
                        last_read_message_id: None,
                        created_at,
                    }))
                } else {
                    Ok(None)
                }
            }
        }
    }

    pub async fn update_channel(
        &self,
        channel_id: Uuid,
        server_id: Uuid,
        name: Option<&str>,
        channel_type: &str,
        encryption_type: &str,
        message_ttl: Option<i32>,
        is_private: bool,
    ) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                match name {
                    Some(n) => {
                        sqlx::query(
                            "UPDATE channels SET name=$1, channel_type=$2, encryption_type=$3, message_ttl=$4, is_private=$5 WHERE id=$6 AND server_id=$7",
                        )
                        .bind(n)
                        .bind(channel_type)
                        .bind(encryption_type)
                        .bind(message_ttl)
                        .bind(is_private)
                        .bind(channel_id)
                        .bind(server_id)
                        .execute(pool).await?;
                    }
                    None => {
                        sqlx::query(
                            "UPDATE channels SET channel_type=$1, encryption_type=$2, message_ttl=$3, is_private=$4 WHERE id=$5 AND server_id=$6",
                        )
                        .bind(channel_type)
                        .bind(encryption_type)
                        .bind(message_ttl)
                        .bind(is_private)
                        .bind(channel_id)
                        .bind(server_id)
                        .execute(pool).await?;
                    }
                }
            }
            DatabasePool::Sqlite(pool) => {
                match name {
                    Some(n) => {
                        sqlx::query(
                            "UPDATE channels SET name=?, type=?, encryption_type=?, message_ttl=?, is_private=? WHERE id=? AND server_id=?",
                        )
                        .bind(n)
                        .bind(channel_type)
                        .bind(encryption_type)
                        .bind(message_ttl)
                        .bind(is_private as i32)
                        .bind(channel_id)
                        .bind(server_id)
                        .execute(pool).await?;
                    }
                    None => {
                        sqlx::query(
                            "UPDATE channels SET type=?, encryption_type=?, message_ttl=?, is_private=? WHERE id=? AND server_id=?",
                        )
                        .bind(channel_type)
                        .bind(encryption_type)
                        .bind(message_ttl)
                        .bind(is_private as i32)
                        .bind(channel_id)
                        .bind(server_id)
                        .execute(pool).await?;
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn delete_channel(&self, channel_id: Uuid) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM channels WHERE id = $1")
                    .bind(channel_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Sqlite(pool) => {
                // SQLite schema històric no garanteix cascades a totes les FK de canals.
                // Eliminem dependències explícitament dins d'una transacció per evitar 500.
                let mut tx = pool.begin().await?;

                sqlx::query("DELETE FROM channel_read_state WHERE channel_id = ?")
                    .bind(channel_id)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query(
                    "DELETE FROM message_attachments WHERE message_id IN (SELECT id FROM messages WHERE channel_id = ?)",
                )
                .bind(channel_id)
                .execute(&mut *tx)
                .await?;

                sqlx::query("DELETE FROM messages WHERE channel_id = ?")
                    .bind(channel_id)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query("DELETE FROM attachments WHERE channel_id = ?")
                    .bind(channel_id)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query("DELETE FROM channel_members WHERE channel_id = ?")
                    .bind(channel_id)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query(
                    "DELETE FROM channel_key_device_bundles \
                     WHERE key_version_id IN (SELECT id FROM channel_key_versions WHERE channel_id = ?)",
                )
                .bind(channel_id)
                .execute(&mut *tx)
                .await?;

                sqlx::query("DELETE FROM channel_key_versions WHERE channel_id = ?")
                    .bind(channel_id)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query("DELETE FROM channels WHERE id = ?")
                    .bind(channel_id)
                    .execute(&mut *tx)
                    .await?;

                tx.commit().await?;
            }
        }
        Ok(())
    }

    pub async fn update_channel_position(&self, channel_id: Uuid, position: i32) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE channels SET position = $1 WHERE id = $2")
                    .bind(position)
                    .bind(channel_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("UPDATE channels SET position = ? WHERE id = ?")
                    .bind(position)
                    .bind(channel_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

}
