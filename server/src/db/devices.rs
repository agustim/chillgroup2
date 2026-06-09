use super::*;
use sqlx::Row;
use uuid::Uuid;

impl DatabasePool {
    /// Retorna el primer dispositiu actiu de l'usuari (device_id, kem_public_key).
    #[allow(dead_code)]
    pub async fn get_device_for_user(&self, user_id: Uuid) -> Result<Option<(Uuid, String)>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, kem_public_key FROM devices WHERE user_id = $1 AND revoked = false ORDER BY created_at ASC LIMIT 1"
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| (r.get(0), r.get(1))))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, kem_public_key FROM devices WHERE user_id = ? AND revoked = 0 ORDER BY created_at ASC LIMIT 1"
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| (r.get(0), r.get(1))))
            }
        }
    }

    /// Crea/reutilitza un dispositiu per usuari segons el `requested_device_id` enviat pel client.
    ///
    /// Regles:
    /// - Si el client envia un `requested_device_id` i ja pertany a l'usuari (no revocat), es reutilitza.
    /// - Si l'id enviat existeix però és d'un altre usuari o està revocat, se'n crea un de nou.
    /// - Si no s'envia cap id, se'n crea un de nou.
    pub async fn upsert_device_for_user(
        &self,
        user_id: Uuid,
        label: &str,
        requested_device_id: Option<Uuid>,
    ) -> Result<Uuid, sqlx::Error> {
        if let Some(candidate_id) = requested_device_id {
            let can_reuse = match self {
                DatabasePool::Postgres(pool) => {
                    sqlx::query(
                        "SELECT 1 FROM devices WHERE id = $1 AND user_id = $2 AND revoked = false LIMIT 1",
                    )
                    .bind(candidate_id)
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?
                    .is_some()
                }
                DatabasePool::Sqlite(pool) => {
                    sqlx::query(
                        "SELECT 1 FROM devices WHERE id = ? AND user_id = ? AND revoked = 0 LIMIT 1",
                    )
                    .bind(candidate_id)
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?
                    .is_some()
                }
            };

            if can_reuse {
                return Ok(candidate_id);
            }

            let exists_for_anyone = match self {
                DatabasePool::Postgres(pool) => {
                    sqlx::query("SELECT 1 FROM devices WHERE id = $1 LIMIT 1")
                        .bind(candidate_id)
                        .fetch_optional(pool)
                        .await?
                        .is_some()
                }
                DatabasePool::Sqlite(pool) => {
                    sqlx::query("SELECT 1 FROM devices WHERE id = ? LIMIT 1")
                        .bind(candidate_id)
                        .fetch_optional(pool)
                        .await?
                        .is_some()
                }
            };

            if !exists_for_anyone {
                let now = chrono::Utc::now().to_rfc3339();
                match self {
                    DatabasePool::Postgres(pool) => {
                        sqlx::query(
                            "INSERT INTO devices (id, user_id, label, public_key, kem_public_key, dsa_public_key, last_seen, revoked, created_at) \
                             VALUES ($1, $2, $3, '', '', '', NOW(), false, NOW()) ON CONFLICT DO NOTHING",
                        )
                        .bind(candidate_id)
                        .bind(user_id)
                        .bind(label)
                        .execute(pool)
                        .await?;
                    }
                    DatabasePool::Sqlite(pool) => {
                        sqlx::query(
                            "INSERT OR IGNORE INTO devices (id, user_id, label, public_key, kem_public_key, dsa_public_key, last_seen, revoked, created_at) \
                             VALUES (?, ?, ?, '', '', '', ?, 0, ?)",
                        )
                        .bind(candidate_id)
                        .bind(user_id)
                        .bind(label)
                        .bind(&now)
                        .bind(&now)
                        .execute(pool)
                        .await?;
                    }
                }

                return Ok(candidate_id);
            }
        }

        let device_id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO devices (id, user_id, label, public_key, kem_public_key, dsa_public_key, last_seen, revoked, created_at) \
                     VALUES ($1, $2, $3, '', '', '', NOW(), false, NOW()) ON CONFLICT DO NOTHING",
                )
                .bind(device_id)
                .bind(user_id)
                .bind(label)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT OR IGNORE INTO devices (id, user_id, label, public_key, kem_public_key, dsa_public_key, last_seen, revoked, created_at) \
                     VALUES (?, ?, ?, '', '', '', ?, 0, ?)",
                )
                .bind(device_id)
                .bind(user_id)
                .bind(label)
                .bind(&now)
                .bind(&now)
                .execute(pool)
                .await?;
            }
        }
        Ok(device_id)
    }

    /// Actualitza les claus públiques d'un dispositiu de l'usuari.
    pub async fn update_device_public_keys(
        &self,
        device_id: Uuid,
        user_id: Uuid,
        kem_public_key: &str,
        dsa_public_key: &str,
    ) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE devices SET public_key = $1, kem_public_key = $1, dsa_public_key = $2, last_seen = NOW() WHERE id = $3 AND user_id = $4"
                )
                .bind(kem_public_key)
                .bind(dsa_public_key)
                .bind(device_id)
                .bind(user_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                let now = chrono::Utc::now().to_rfc3339();
                sqlx::query(
                    "UPDATE devices SET public_key = ?, kem_public_key = ?, dsa_public_key = ?, last_seen = ? WHERE id = ? AND user_id = ?"
                )
                .bind(kem_public_key)
                .bind(kem_public_key)
                .bind(dsa_public_key)
                .bind(&now)
                .bind(device_id)
                .bind(user_id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    /// Llista tots els dispositius d'un usuari.
    pub async fn list_devices_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<(Uuid, Option<String>, String, String, String, String, bool)>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, label, kem_public_key, dsa_public_key, created_at, last_seen, revoked \
                     FROM devices \
                     WHERE user_id = $1 \
                     ORDER BY created_at ASC"
                )
                .bind(user_id)
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .into_iter()
                    .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4), r.get(5), r.get(6)))
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, label, kem_public_key, dsa_public_key, created_at, last_seen, revoked \
                     FROM devices \
                     WHERE user_id = ? \
                     ORDER BY created_at ASC"
                )
                .bind(user_id)
                .fetch_all(pool)
                .await?;
                Ok(rows
                    .into_iter()
                    .map(|r| {
                        let revoked: i64 = r.get(6);
                        (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4), r.get(5), revoked != 0)
                    })
                    .collect())
            }
        }
    }

    /// Revoca un dispositiu de l'usuari.
    pub async fn revoke_device_for_user(&self, device_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query(
                    "UPDATE devices SET revoked = true, last_seen = NOW() WHERE id = $1 AND user_id = $2"
                )
                .bind(device_id)
                .bind(user_id)
                .execute(pool)
                .await?;
                Ok(result.rows_affected() > 0)
            }
            DatabasePool::Sqlite(pool) => {
                let now = chrono::Utc::now().to_rfc3339();
                let result = sqlx::query(
                    "UPDATE devices SET revoked = 1, last_seen = ? WHERE id = ? AND user_id = ?"
                )
                .bind(&now)
                .bind(device_id)
                .bind(user_id)
                .execute(pool)
                .await?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    /// Retorna el bundle de clau de canal més recent per a un dispositiu concret.
    pub async fn get_latest_channel_key_bundle_for_device(
        &self,
        channel_id: Uuid,
        device_id: Uuid,
    ) -> Result<Option<(Uuid, i32, String, String, Option<String>, Option<Uuid>)>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT ckv.id, ckv.version, ckdb.encrypted_key, ckdb.kem_ciphertext, ckdb.signature, ckdb.signed_by_device_id \
                     FROM channel_key_device_bundles ckdb \
                     JOIN channel_key_versions ckv ON ckv.id = ckdb.key_version_id \
                     WHERE ckv.channel_id = $1 AND ckdb.device_id = $2 \
                     ORDER BY ckv.version DESC \
                     LIMIT 1"
                )
                .bind(channel_id)
                .bind(device_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4), r.get(5))))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT ckv.id, ckv.version, ckdb.encrypted_key, ckdb.kem_ciphertext, ckdb.signature, ckdb.signed_by_device_id \
                     FROM channel_key_device_bundles ckdb \
                     JOIN channel_key_versions ckv ON ckv.id = ckdb.key_version_id \
                     WHERE ckv.channel_id = ? AND ckdb.device_id = ? \
                     ORDER BY ckv.version DESC \
                     LIMIT 1"
                )
                .bind(channel_id)
                .bind(device_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4), r.get(5))))
            }
        }
    }

    /// Retorna les claus públiques d'un dispositiu concret si pertany a l'usuari i no està revocat.

        /// Retorna tots els bundles de clau de canal del canal, per a tots els dispositius, ordenats per versió ascendent.
        pub async fn get_all_channel_key_bundles(
            &self,
            channel_id: Uuid,
        ) -> Result<Vec<(Uuid, Uuid, i32, String, String, Option<String>, Option<Uuid>)>, sqlx::Error> {
            match self {
                DatabasePool::Postgres(pool) => {
                    let rows = sqlx::query(
                        "SELECT ckdb.device_id, ckv.id, ckv.version, ckdb.encrypted_key, ckdb.kem_ciphertext, ckdb.signature, ckdb.signed_by_device_id \
                         FROM channel_key_device_bundles ckdb \
                         JOIN channel_key_versions ckv ON ckv.id = ckdb.key_version_id \
                         WHERE ckv.channel_id = $1 \
                         ORDER BY ckv.version ASC"
                    )
                    .bind(channel_id)
                    .fetch_all(pool)
                    .await?;
                    Ok(rows.iter().map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4), r.get(5), r.get(6))).collect())
                }
                DatabasePool::Sqlite(pool) => {
                    let rows = sqlx::query(
                        "SELECT ckdb.device_id, ckv.id, ckv.version, ckdb.encrypted_key, ckdb.kem_ciphertext, ckdb.signature, ckdb.signed_by_device_id \
                         FROM channel_key_device_bundles ckdb \
                         JOIN channel_key_versions ckv ON ckv.id = ckdb.key_version_id \
                         WHERE ckv.channel_id = ? \
                         ORDER BY ckv.version ASC"
                    )
                    .bind(channel_id)
                    .fetch_all(pool)
                    .await?;
                    Ok(rows.iter().map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4), r.get(5), r.get(6))).collect())
                }
            }
        }

    pub async fn get_device_public_keys_for_user(
        &self,
        device_id: Uuid,
        user_id: Uuid,
    ) -> Result<Option<(String, String)>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT kem_public_key, dsa_public_key FROM devices WHERE id = $1 AND user_id = $2 AND revoked = false LIMIT 1"
                )
                .bind(device_id)
                .bind(user_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| (
                    r.get::<Option<String>, _>(0).unwrap_or_default(),
                    r.get::<Option<String>, _>(1).unwrap_or_default(),
                )))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT kem_public_key, dsa_public_key FROM devices WHERE id = ? AND user_id = ? AND revoked = 0 LIMIT 1"
                )
                .bind(device_id)
                .bind(user_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| (
                    r.get::<Option<String>, _>(0).unwrap_or_default(),
                    r.get::<Option<String>, _>(1).unwrap_or_default(),
                )))
            }
        }
    }

    #[allow(dead_code)]
    pub async fn get_dsa_public_key_for_device(&self, device_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT dsa_public_key FROM devices WHERE id = $1 AND revoked = false LIMIT 1"
                )
                .bind(device_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| r.get::<Option<String>, _>(0).unwrap_or_default()))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT dsa_public_key FROM devices WHERE id = ? AND revoked = 0 LIMIT 1"
                )
                .bind(device_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| r.get::<Option<String>, _>(0).unwrap_or_default()))
            }
        }
    }

    /// Crea una nova versió de clau simètrica d'un canal.
    pub async fn create_channel_key_version(
        &self,
        channel_id: Uuid,
        version: i32,
        encrypted_key: &str,
        nonce: &str,
        created_by: Uuid,
    ) -> Result<Uuid, sqlx::Error> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO channel_key_versions (id, channel_id, version, encrypted_key, nonce, created_by, created_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, NOW())"
                )
                .bind(id)
                .bind(channel_id)
                .bind(version)
                .bind(encrypted_key)
                .bind(nonce)
                .bind(created_by)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO channel_key_versions (id, channel_id, version, encrypted_key, nonce, created_by, created_at) \
                     VALUES (?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(id)
                .bind(channel_id)
                .bind(version)
                .bind(encrypted_key)
                .bind(nonce)
                .bind(created_by)
                .bind(&now)
                .execute(pool)
                .await?;
            }
        }
        Ok(id)
    }

    /// Obté la versió més recent de clau simètrica d'un canal.
    pub async fn get_latest_channel_key_version(
        &self,
        channel_id: Uuid,
    ) -> Result<Option<(Uuid, i32, String, String)>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, version, encrypted_key, nonce \
                     FROM channel_key_versions \
                     WHERE channel_id = $1 AND deprecated_at IS NULL \
                     ORDER BY version DESC \
                     LIMIT 1"
                )
                .bind(channel_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| (r.get(0), r.get(1), r.get(2), r.get(3))))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, version, encrypted_key, nonce \
                     FROM channel_key_versions \
                     WHERE channel_id = ? AND deprecated_at IS NULL \
                     ORDER BY version DESC \
                     LIMIT 1"
                )
                .bind(channel_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| (r.get(0), r.get(1), r.get(2), r.get(3))))
            }
        }
    }

    /// Obté una versió concreta de clau simètrica d'un canal.
    pub async fn get_channel_key_version(
        &self,
        channel_id: Uuid,
        version: i32,
    ) -> Result<Option<(Uuid, i32, String, String)>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, version, encrypted_key, nonce \
                     FROM channel_key_versions \
                     WHERE channel_id = $1 AND version = $2 \
                     LIMIT 1"
                )
                .bind(channel_id)
                .bind(version)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| (r.get(0), r.get(1), r.get(2), r.get(3))))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, version, encrypted_key, nonce \
                     FROM channel_key_versions \
                     WHERE channel_id = ? AND version = ? \
                     LIMIT 1"
                )
                .bind(channel_id)
                .bind(version)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| (r.get(0), r.get(1), r.get(2), r.get(3))))
            }
        }
    }

    pub async fn get_channel_key_version_metadata(
        &self,
        channel_id: Uuid,
    ) -> Result<Option<(Uuid, i32)>, sqlx::Error> {
        Ok(self
            .get_latest_channel_key_version(channel_id)
            .await?
            .map(|(key_version_id, key_version, _, _)| (key_version_id, key_version)))
    }

    /// Guarda un bundle de clau per dispositiu sense permetre sobrescriptura divergent.
    pub async fn store_channel_key_bundle_for_device(
        &self,
        key_version_id: Uuid,
        device_id: Uuid,
        encrypted_key: &str,
        kem_ciphertext: &str,
        signature: Option<&str>,
        signed_by_device_id: Option<Uuid>,
    ) -> Result<ChannelKeyBundleWriteResult, sqlx::Error> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();

        let is_same_payload = |existing_encrypted_key: String,
                               existing_kem_ciphertext: String,
                               existing_signature: Option<String>,
                               existing_signed_by_device_id: Option<Uuid>| {
            existing_encrypted_key == encrypted_key
                && existing_kem_ciphertext == kem_ciphertext
                && existing_signature.as_deref() == signature
                && existing_signed_by_device_id == signed_by_device_id
        };

        match self {
            DatabasePool::Postgres(pool) => {
                let insert_result = sqlx::query(
                    "INSERT INTO channel_key_device_bundles (id, key_version_id, device_id, encrypted_key, kem_ciphertext, signature, signed_by_device_id, created_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, NOW()) \
                     ON CONFLICT (key_version_id, device_id) DO NOTHING"
                )
                .bind(id)
                .bind(key_version_id)
                .bind(device_id)
                .bind(encrypted_key)
                .bind(kem_ciphertext)
                .bind(signature)
                .bind(signed_by_device_id)
                .execute(pool)
                .await?;

                if insert_result.rows_affected() > 0 {
                    return Ok(ChannelKeyBundleWriteResult::Inserted);
                }

                let existing = sqlx::query(
                    "SELECT encrypted_key, kem_ciphertext, signature, signed_by_device_id \
                     FROM channel_key_device_bundles \
                     WHERE key_version_id = $1 AND device_id = $2 \
                     LIMIT 1"
                )
                .bind(key_version_id)
                .bind(device_id)
                .fetch_optional(pool)
                .await?;

                if let Some(row) = existing {
                    if is_same_payload(row.get(0), row.get(1), row.get(2), row.get(3)) {
                        return Ok(ChannelKeyBundleWriteResult::Unchanged);
                    }
                }

                Ok(ChannelKeyBundleWriteResult::Conflict)
            }
            DatabasePool::Sqlite(pool) => {
                let insert_result = sqlx::query(
                    "INSERT INTO channel_key_device_bundles (id, key_version_id, device_id, encrypted_key, kem_ciphertext, signature, signed_by_device_id, created_at) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
                     ON CONFLICT (key_version_id, device_id) DO NOTHING"
                )
                .bind(id)
                .bind(key_version_id)
                .bind(device_id)
                .bind(encrypted_key)
                .bind(kem_ciphertext)
                .bind(signature)
                .bind(signed_by_device_id)
                .bind(&now)
                .execute(pool)
                .await?;

                if insert_result.rows_affected() > 0 {
                    return Ok(ChannelKeyBundleWriteResult::Inserted);
                }

                let existing = sqlx::query(
                    "SELECT encrypted_key, kem_ciphertext, signature, signed_by_device_id \
                     FROM channel_key_device_bundles \
                     WHERE key_version_id = ? AND device_id = ? \
                     LIMIT 1"
                )
                .bind(key_version_id)
                .bind(device_id)
                .fetch_optional(pool)
                .await?;

                if let Some(row) = existing {
                    if is_same_payload(row.get(0), row.get(1), row.get(2), row.get(3)) {
                        return Ok(ChannelKeyBundleWriteResult::Unchanged);
                    }
                }

                Ok(ChannelKeyBundleWriteResult::Conflict)
            }
        }
    }

    /// Retorna els dispositius actius dels membres del canal, tinguin o no claus públiques registrades.
    pub async fn get_member_devices_for_channel(&self, channel_id: Uuid) -> Result<Vec<(Uuid, String, String)>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT DISTINCT d.id, d.kem_public_key, d.dsa_public_key \
                     FROM devices d \
                     JOIN channels c ON c.id = $1 \
                     LEFT JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = d.user_id \
                     LEFT JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = d.user_id \
                     WHERE d.revoked = false \
                       AND (\
                            (c.scope = 'dm' AND cm.user_id IS NOT NULL) \
                            OR \
                            (c.scope != 'dm' AND sm.user_id IS NOT NULL AND (c.is_private = false OR cm.user_id IS NOT NULL))\
                       )"
                )
                .bind(channel_id)
                .fetch_all(pool)
                .await?;
                Ok(rows.into_iter().map(|r| (r.get(0), r.get(1), r.get(2))).collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT DISTINCT d.id, d.kem_public_key, d.dsa_public_key \
                     FROM devices d \
                     JOIN channels c ON c.id = ? \
                     LEFT JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = d.user_id \
                     LEFT JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = d.user_id \
                                         WHERE d.revoked = 0 \
                       AND (\
                            (COALESCE(c.scope, 'server') = 'dm' AND cm.user_id IS NOT NULL) \
                            OR \
                            (COALESCE(c.scope, 'server') != 'dm' AND sm.user_id IS NOT NULL AND (c.is_private = 0 OR cm.user_id IS NOT NULL))\
                       )"
                )
                .bind(channel_id)
                .fetch_all(pool)
                .await?;
                Ok(rows.into_iter().map(|r| (r.get(0), r.get(1), r.get(2))).collect())
            }
        }
    }

    pub async fn count_unread_messages_for_user(
        &self,
        channel_id: Uuid,
        user_id: Uuid,
    ) -> Result<usize, sqlx::Error> {
                                let now = chrono::Utc::now().to_rfc3339();
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT COUNT(*) FROM messages m \
                     LEFT JOIN channel_read_state rs \
                       ON rs.channel_id = m.channel_id AND rs.user_id = $2 \
                     WHERE m.channel_id = $1 \
                       AND m.sender_user_id != $2 \
                       AND m.deleted_at IS NULL \
                                             AND (m.expires_at IS NULL OR m.expires_at > $3::timestamptz) \
                                             AND m.timestamp > COALESCE(rs.last_read_at, TIMESTAMPTZ '1970-01-01 00:00:00+00')",
                )
                .bind(channel_id)
                .bind(user_id)
                                .bind(&now)
                .fetch_one(pool)
                .await?;
                Ok(row.get::<i64, _>(0) as usize)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT COUNT(*) FROM messages m \
                     LEFT JOIN channel_read_state rs \
                       ON rs.channel_id = m.channel_id AND rs.user_id = ? \
                     WHERE m.channel_id = ? \
                       AND m.sender_user_id != ? \
                       AND m.deleted_at IS NULL \
                                             AND (m.expires_at IS NULL OR m.expires_at > ?) \
                       AND m.timestamp > COALESCE(rs.last_read_at, '1970-01-01T00:00:00Z')",
                )
                .bind(user_id)
                .bind(channel_id)
                .bind(user_id)
                                .bind(&now)
                .fetch_one(pool)
                .await?;
                Ok(row.get::<i64, _>(0) as usize)
            }
        }
    }
}
