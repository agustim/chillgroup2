use super::*;
use sqlx::Row;
use uuid::Uuid;

impl DatabasePool {
    /// Returns (max_storage_bytes, max_transfer_bytes_monthly) for the user's plan.
    /// -1 means unlimited.
    pub async fn get_user_s3_quota(&self, user_id: Uuid) -> Result<(i64, i64), String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT p.max_storage_bytes, p.max_transfer_bytes_monthly \
                     FROM users u JOIN plans p ON u.plan_id = p.id \
                     WHERE u.id = $1",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("get_user_s3_quota Postgres: {}", e))?;
                Ok(row.map(|r| (r.get::<i64, _>(0), r.get::<i64, _>(1))).unwrap_or((-1, -1)))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT p.max_storage_bytes, p.max_transfer_bytes_monthly \
                     FROM users u JOIN plans p ON u.plan_id = p.id \
                     WHERE u.id = ?",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("get_user_s3_quota SQLite: {}", e))?;
                Ok(row.map(|r| (r.get::<i64, _>(0), r.get::<i64, _>(1))).unwrap_or((-1, -1)))
            }
        }
    }

    /// Returns (stored_bytes, transfer_bytes) for the given user/month (format: '2026-06').
    pub async fn get_user_storage_usage(
        &self,
        user_id: Uuid,
        year_month: &str,
    ) -> Result<(i64, i64), String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT stored_bytes, transfer_bytes \
                     FROM user_storage_usage_monthly \
                     WHERE user_id = $1 AND year_month = $2",
                )
                .bind(user_id)
                .bind(year_month)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("get_user_storage_usage Postgres: {}", e))?;
                Ok(row.map(|r| (r.get::<i64, _>(0), r.get::<i64, _>(1))).unwrap_or((0, 0)))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT stored_bytes, transfer_bytes \
                     FROM user_storage_usage_monthly \
                     WHERE user_id = ? AND year_month = ?",
                )
                .bind(user_id)
                .bind(year_month)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("get_user_storage_usage SQLite: {}", e))?;
                Ok(row.map(|r| (r.get::<i64, _>(0), r.get::<i64, _>(1))).unwrap_or((0, 0)))
            }
        }
    }

    /// Increments stored_bytes for the user/month. Returns new total.
    pub async fn increment_stored_bytes(
        &self,
        user_id: Uuid,
        year_month: &str,
        delta: i64,
    ) -> Result<i64, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "INSERT INTO user_storage_usage_monthly (user_id, year_month, stored_bytes) \
                     VALUES ($1, $2, $3) \
                     ON CONFLICT (user_id, year_month) \
                     DO UPDATE SET stored_bytes = user_storage_usage_monthly.stored_bytes + EXCLUDED.stored_bytes, \
                                   updated_at = now() \
                     RETURNING stored_bytes",
                )
                .bind(user_id)
                .bind(year_month)
                .bind(delta)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("increment_stored_bytes Postgres: {}", e))?;
                Ok(row.get::<i64, _>(0))
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO user_storage_usage_monthly (user_id, year_month, stored_bytes) \
                     VALUES (?, ?, ?) \
                     ON CONFLICT (user_id, year_month) \
                     DO UPDATE SET stored_bytes = stored_bytes + excluded.stored_bytes, \
                                   updated_at = CURRENT_TIMESTAMP",
                )
                .bind(user_id)
                .bind(year_month)
                .bind(delta)
                .execute(pool)
                .await
                .map_err(|e| format!("increment_stored_bytes SQLite: {}", e))?;
                let row = sqlx::query(
                    "SELECT stored_bytes FROM user_storage_usage_monthly WHERE user_id = ? AND year_month = ?",
                )
                .bind(user_id)
                .bind(year_month)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("increment_stored_bytes fetch SQLite: {}", e))?;
                Ok(row.get::<i64, _>(0))
            }
        }
    }

    /// Returns (warning_sent_at_80_is_set, warning_sent_at_90_is_set) for user/month.
    pub async fn get_quota_warning_timestamps(
        &self,
        user_id: Uuid,
        year_month: &str,
    ) -> Result<(bool, bool), String> {
        let parse_bool = |s: Option<String>| s.map(|v| !v.is_empty()).unwrap_or(false);
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT \
                        CASE WHEN warning_sent_at_80 IS NOT NULL THEN 'yes' ELSE '' END, \
                        CASE WHEN warning_sent_at_90 IS NOT NULL THEN 'yes' ELSE '' END \
                     FROM user_storage_usage_monthly \
                     WHERE user_id = $1 AND year_month = $2",
                )
                .bind(user_id)
                .bind(year_month)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("get_quota_warning_timestamps Postgres: {}", e))?;
                Ok(row
                    .map(|r| (
                        parse_bool(r.get::<Option<String>, _>(0)),
                        parse_bool(r.get::<Option<String>, _>(1)),
                    ))
                    .unwrap_or((false, false)))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT \
                        CASE WHEN warning_sent_at_80 IS NOT NULL THEN 'yes' ELSE '' END, \
                        CASE WHEN warning_sent_at_90 IS NOT NULL THEN 'yes' ELSE '' END \
                     FROM user_storage_usage_monthly \
                     WHERE user_id = ? AND year_month = ?",
                )
                .bind(user_id)
                .bind(year_month)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("get_quota_warning_timestamps SQLite: {}", e))?;
                Ok(row
                    .map(|r| (
                        parse_bool(r.get::<Option<String>, _>(0)),
                        parse_bool(r.get::<Option<String>, _>(1)),
                    ))
                    .unwrap_or((false, false)))
            }
        }
    }

    /// Marks warning_sent_at_80 or warning_sent_at_90 for user/month.
    pub async fn set_quota_warning_sent(
        &self,
        user_id: Uuid,
        year_month: &str,
        threshold: u8,
    ) -> Result<(), String> {
        let col = if threshold == 80 { "warning_sent_at_80" } else { "warning_sent_at_90" };
        match self {
            DatabasePool::Postgres(pool) => {
                let q = format!(
                    "INSERT INTO user_storage_usage_monthly (user_id, year_month, {col}) \
                     VALUES ($1, $2, now()) \
                     ON CONFLICT (user_id, year_month) \
                     DO UPDATE SET {col} = now(), updated_at = now()"
                );
                sqlx::query(&q)
                    .bind(user_id)
                    .bind(year_month)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("set_quota_warning_sent Postgres: {}", e))?;
            }
            DatabasePool::Sqlite(pool) => {
                let q = format!(
                    "INSERT INTO user_storage_usage_monthly (user_id, year_month, {col}) \
                     VALUES (?, ?, CURRENT_TIMESTAMP) \
                     ON CONFLICT (user_id, year_month) \
                     DO UPDATE SET {col} = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP"
                );
                sqlx::query(&q)
                    .bind(user_id)
                    .bind(year_month)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("set_quota_warning_sent SQLite: {}", e))?;
            }
        }
        Ok(())
    }

    /// Increments transfer_bytes for the user/month. Returns new total.
    pub async fn increment_transfer_bytes(
        &self,
        user_id: Uuid,
        year_month: &str,
        delta: i64,
    ) -> Result<i64, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "INSERT INTO user_storage_usage_monthly (user_id, year_month, transfer_bytes) \
                     VALUES ($1, $2, $3) \
                     ON CONFLICT (user_id, year_month) \
                     DO UPDATE SET transfer_bytes = user_storage_usage_monthly.transfer_bytes + EXCLUDED.transfer_bytes, \
                                   updated_at = now() \
                     RETURNING transfer_bytes",
                )
                .bind(user_id)
                .bind(year_month)
                .bind(delta)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("increment_transfer_bytes Postgres: {}", e))?;
                Ok(row.get::<i64, _>(0))
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO user_storage_usage_monthly (user_id, year_month, transfer_bytes) \
                     VALUES (?, ?, ?) \
                     ON CONFLICT (user_id, year_month) \
                     DO UPDATE SET transfer_bytes = transfer_bytes + excluded.transfer_bytes, \
                                   updated_at = CURRENT_TIMESTAMP",
                )
                .bind(user_id)
                .bind(year_month)
                .bind(delta)
                .execute(pool)
                .await
                .map_err(|e| format!("increment_transfer_bytes SQLite: {}", e))?;
                let row = sqlx::query(
                    "SELECT transfer_bytes FROM user_storage_usage_monthly WHERE user_id = ? AND year_month = ?",
                )
                .bind(user_id)
                .bind(year_month)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("increment_transfer_bytes fetch SQLite: {}", e))?;
                Ok(row.get::<i64, _>(0))
            }
        }
    }
}

impl DatabasePool {
    /// Returns max_streaming_hours_monthly for the user's plan. -1 = unlimited.
    pub async fn get_user_streaming_quota(&self, user_id: Uuid) -> Result<i32, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT p.max_streaming_hours_monthly \
                     FROM users u JOIN plans p ON u.plan_id = p.id \
                     WHERE u.id = $1",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("get_user_streaming_quota Postgres: {}", e))?;
                Ok(row.map(|r| r.get::<i32, _>(0)).unwrap_or(-1))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT p.max_streaming_hours_monthly \
                     FROM users u JOIN plans p ON u.plan_id = p.id \
                     WHERE u.id = ?",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("get_user_streaming_quota SQLite: {}", e))?;
                Ok(row.map(|r| r.get::<i32, _>(0)).unwrap_or(-1))
            }
        }
    }

    /// Returns streaming_seconds used this month for the user.
    pub async fn get_user_streaming_usage(
        &self,
        user_id: Uuid,
        year_month: &str,
    ) -> Result<i64, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT streaming_seconds FROM user_streaming_usage_monthly \
                     WHERE user_id = $1 AND year_month = $2",
                )
                .bind(user_id)
                .bind(year_month)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("get_user_streaming_usage Postgres: {}", e))?;
                Ok(row.map(|r| r.get::<i64, _>(0)).unwrap_or(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT streaming_seconds FROM user_streaming_usage_monthly \
                     WHERE user_id = ? AND year_month = ?",
                )
                .bind(user_id)
                .bind(year_month)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("get_user_streaming_usage SQLite: {}", e))?;
                Ok(row.map(|r| r.get::<i64, _>(0)).unwrap_or(0))
            }
        }
    }

    /// Increments streaming_seconds for user/month. Returns new total.
    pub async fn increment_streaming_seconds(
        &self,
        user_id: Uuid,
        year_month: &str,
        delta: i64,
    ) -> Result<i64, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "INSERT INTO user_streaming_usage_monthly (user_id, year_month, streaming_seconds) \
                     VALUES ($1, $2, $3) \
                     ON CONFLICT (user_id, year_month) \
                     DO UPDATE SET streaming_seconds = user_streaming_usage_monthly.streaming_seconds + EXCLUDED.streaming_seconds, \
                                   updated_at = now() \
                     RETURNING streaming_seconds",
                )
                .bind(user_id)
                .bind(year_month)
                .bind(delta)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("increment_streaming_seconds Postgres: {}", e))?;
                Ok(row.get::<i64, _>(0))
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO user_streaming_usage_monthly (user_id, year_month, streaming_seconds) \
                     VALUES (?, ?, ?) \
                     ON CONFLICT (user_id, year_month) \
                     DO UPDATE SET streaming_seconds = streaming_seconds + excluded.streaming_seconds, \
                                   updated_at = CURRENT_TIMESTAMP",
                )
                .bind(user_id)
                .bind(year_month)
                .bind(delta)
                .execute(pool)
                .await
                .map_err(|e| format!("increment_streaming_seconds SQLite: {}", e))?;
                let row = sqlx::query(
                    "SELECT streaming_seconds FROM user_streaming_usage_monthly \
                     WHERE user_id = ? AND year_month = ?",
                )
                .bind(user_id)
                .bind(year_month)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("increment_streaming_seconds fetch SQLite: {}", e))?;
                Ok(row.get::<i64, _>(0))
            }
        }
    }

    pub async fn get_admin_user_ids(&self) -> Result<Vec<Uuid>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query("SELECT id FROM users WHERE role = 'admin'")
                    .fetch_all(pool)
                    .await
                    .map_err(|e| format!("get_admin_user_ids Postgres: {}", e))?;
                Ok(rows.into_iter().map(|r| r.get(0)).collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT id FROM users WHERE role = 'admin'")
                    .fetch_all(pool)
                    .await
                    .map_err(|e| format!("get_admin_user_ids SQLite: {}", e))?;
                Ok(rows.into_iter().map(|r| r.get(0)).collect())
            }
        }
    }

    pub async fn create_plan_change_request(
        &self,
        user_id: Uuid,
        requested_plan_id: Uuid,
        message: Option<&str>,
    ) -> Result<Uuid, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "INSERT INTO plan_change_requests (user_id, requested_plan_id, message) \
                     VALUES ($1, $2, $3) RETURNING id",
                )
                .bind(user_id)
                .bind(requested_plan_id)
                .bind(message)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("create_plan_change_request Postgres: {}", e))?;
                Ok(row.get(0))
            }
            DatabasePool::Sqlite(pool) => {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO plan_change_requests (id, user_id, requested_plan_id, message) \
                     VALUES (?, ?, ?, ?)",
                )
                .bind(id)
                .bind(user_id)
                .bind(requested_plan_id)
                .bind(message)
                .execute(pool)
                .await
                .map_err(|e| format!("create_plan_change_request SQLite: {}", e))?;
                Ok(id)
            }
        }
    }

    pub async fn list_plan_change_requests_admin(
        &self,
    ) -> Result<Vec<(Uuid, Uuid, String, Uuid, String, String, Option<String>, Option<String>, String)>, String> {
        let query = "SELECT r.id, r.user_id, u.username, r.requested_plan_id, p.display_name, \
                     r.status, r.message, r.admin_note, r.created_at::text \
                     FROM plan_change_requests r \
                     JOIN users u ON u.id = r.user_id \
                     JOIN plans p ON p.id = r.requested_plan_id \
                     ORDER BY r.created_at DESC";
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(query)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| format!("list_plan_change_requests Postgres: {}", e))?;
                Ok(rows
                    .into_iter()
                    .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4), r.get(5), r.get(6), r.get(7), r.get(8)))
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let query_sqlite = "SELECT r.id, r.user_id, u.username, r.requested_plan_id, p.display_name, \
                                    r.status, r.message, r.admin_note, r.created_at \
                                    FROM plan_change_requests r \
                                    JOIN users u ON u.id = r.user_id \
                                    JOIN plans p ON p.id = r.requested_plan_id \
                                    ORDER BY r.created_at DESC";
                let rows = sqlx::query(query_sqlite)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| format!("list_plan_change_requests SQLite: {}", e))?;
                Ok(rows
                    .into_iter()
                    .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4), r.get(5), r.get(6), r.get(7), r.get(8)))
                    .collect())
            }
        }
    }

    pub async fn resolve_plan_change_request(
        &self,
        request_id: Uuid,
        status: &str,
        admin_note: Option<&str>,
    ) -> Result<Option<(Uuid, Uuid)>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "UPDATE plan_change_requests SET status = $1, admin_note = $2, updated_at = now() \
                     WHERE id = $3 AND status = 'pending' \
                     RETURNING user_id, requested_plan_id",
                )
                .bind(status)
                .bind(admin_note)
                .bind(request_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("resolve_plan_change_request Postgres: {}", e))?;
                Ok(row.map(|r| (r.get(0), r.get(1))))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT user_id, requested_plan_id FROM plan_change_requests \
                     WHERE id = ? AND status = 'pending'",
                )
                .bind(request_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("resolve_plan_change_request fetch SQLite: {}", e))?;
                if let Some(r) = row {
                    let user_id: Uuid = r.get(0);
                    let plan_id: Uuid = r.get(1);
                    sqlx::query(
                        "UPDATE plan_change_requests SET status = ?, admin_note = ?, updated_at = CURRENT_TIMESTAMP \
                         WHERE id = ?",
                    )
                    .bind(status)
                    .bind(admin_note)
                    .bind(request_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("resolve_plan_change_request update SQLite: {}", e))?;
                    Ok(Some((user_id, plan_id)))
                } else {
                    Ok(None)
                }
            }
        }
    }

    pub async fn get_pending_plan_change_request_for_user(
        &self,
        user_id: Uuid,
    ) -> Result<Option<Uuid>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id FROM plan_change_requests WHERE user_id = $1 AND status = 'pending' LIMIT 1",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("get_pending_plan_change_request Postgres: {}", e))?;
                Ok(row.map(|r| r.get(0)))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id FROM plan_change_requests WHERE user_id = ? AND status = 'pending' LIMIT 1",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("get_pending_plan_change_request SQLite: {}", e))?;
                Ok(row.map(|r| r.get(0)))
            }
        }
    }
}
