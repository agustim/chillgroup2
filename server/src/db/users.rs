use super::*;
use sqlx::Row;
use uuid::Uuid;

impl DatabasePool {
    pub async fn execute_query(&self, query: &str) -> Result<(), String> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(query)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error PostgreSQL: {}", e))?;
                Ok(())
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(query)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error SQLite: {}", e))?;
                Ok(())
            }
        }
    }

    /// Buscar usuari per username i obtenir (id, username, password_hash).
    pub async fn find_user_by_username(&self, username: &str) -> Result<Option<(Uuid, String, String)>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT id, username, password_hash FROM users WHERE username = $1")
                    .bind(username)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| format!("Error PostgreSQL: {}", e))?;
                Ok(row.map(|r| (r.get(0), r.get(1), r.get(2))))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT id, username, password_hash FROM users WHERE username = ?")
                    .bind(username)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| format!("Error SQLite: {}", e))?;
                Ok(row.map(|r| (r.get(0), r.get(1), r.get(2))))
            }
        }
    }

    pub async fn find_username_by_user_id(&self, user_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT username FROM users WHERE id = $1")
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?;
                Ok(row.map(|r| r.get(0)))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT username FROM users WHERE id = ?")
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?;
                Ok(row.map(|r| r.get(0)))
            }
        }
    }

    /// Crear un nou usuari. Retorna el user_id generat.
    #[allow(dead_code)]
    pub async fn create_user(&self, username: &str, password_hash: &str) -> Result<Uuid, String> {
        self.create_user_with_role(username, password_hash, "user").await
    }

    pub async fn create_user_with_role(
        &self,
        username: &str,
        password_hash: &str,
        role: &str,
    ) -> Result<Uuid, String> {
        let free_plan_id = self
            .get_plan_id_by_name("free")
            .await?
            .ok_or_else(|| "No s'ha trobat el pla 'free'".to_string())?;

        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("INSERT INTO users (username, password_hash, role, plan_id) VALUES ($1, $2, $3, $4) RETURNING id")
                    .bind(username)
                    .bind(password_hash)
                    .bind(role)
                    .bind(free_plan_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error PostgreSQL: {}", e))?;
                Ok(row.get(0))
            }
            DatabasePool::Sqlite(pool) => {
                let user_id = Uuid::new_v4();
                sqlx::query("INSERT INTO users (id, username, password_hash, role, plan_id) VALUES (?, ?, ?, ?, ?)")
                    .bind(user_id)
                    .bind(username)
                    .bind(password_hash)
                    .bind(role)
                    .bind(free_plan_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error SQLite: {}", e))?;
                Ok(user_id)
            }
        }
    }

    pub async fn ensure_default_plans(&self) -> Result<(), String> {
        let free_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655441001")
            .map_err(|e| format!("UUID free invàlid: {}", e))?;
        let pro_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655441002")
            .map_err(|e| format!("UUID pro invàlid: {}", e))?;
        let enterprise_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655441003")
            .map_err(|e| format!("UUID enterprise invàlid: {}", e))?;

        // free: 10 GB storage, 100 GB transfer/month, 10h streaming
        // pro:  50 GB storage, 500 GB transfer/month, 50h streaming
        // enterprise: unlimited (-1)
        const FREE_STORAGE: i64   = 10 * 1024 * 1024 * 1024;
        const FREE_TRANSFER: i64  = 100 * 1024 * 1024 * 1024;
        const FREE_STREAMING: i32 = 10;
        const PRO_STORAGE: i64    = 50 * 1024 * 1024 * 1024;
        const PRO_TRANSFER: i64   = 500 * 1024 * 1024 * 1024;
        const PRO_STREAMING: i32  = 50;

        match self {
            DatabasePool::Postgres(pool) => {
                let insert = "INSERT INTO plans (id, name, display_name, description, max_servers, max_channels_text_per_server, max_channels_voice_per_server, max_members_per_server, api_calls_per_minute, messages_per_day, max_storage_bytes, max_transfer_bytes_monthly, max_streaming_hours_monthly) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13) ON CONFLICT (name) DO UPDATE SET max_storage_bytes = EXCLUDED.max_storage_bytes, max_transfer_bytes_monthly = EXCLUDED.max_transfer_bytes_monthly, max_streaming_hours_monthly = EXCLUDED.max_streaming_hours_monthly";
                sqlx::query(insert)
                    .bind(free_id)
                    .bind("free")
                    .bind("Free")
                    .bind("Plan gratuït")
                    .bind(1i32)
                    .bind(3i32)
                    .bind(2i32)
                    .bind(20i32)
                    .bind(60i32)
                    .bind(10000i32)
                    .bind(FREE_STORAGE)
                    .bind(FREE_TRANSFER)
                    .bind(FREE_STREAMING)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error inserint plan free (Postgres): {}", e))?;

                sqlx::query(insert)
                    .bind(pro_id)
                    .bind("pro")
                    .bind("Pro")
                    .bind("Plan professional")
                    .bind(5i32)
                    .bind(20i32)
                    .bind(10i32)
                    .bind(500i32)
                    .bind(600i32)
                    .bind(-1i32)
                    .bind(PRO_STORAGE)
                    .bind(PRO_TRANSFER)
                    .bind(PRO_STREAMING)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error inserint plan pro (Postgres): {}", e))?;

                sqlx::query(insert)
                    .bind(enterprise_id)
                    .bind("enterprise")
                    .bind("Enterprise")
                    .bind("Plan enterprise")
                    .bind(-1i32)
                    .bind(-1i32)
                    .bind(-1i32)
                    .bind(-1i32)
                    .bind(-1i32)
                    .bind(-1i32)
                    .bind(-1i64)
                    .bind(-1i64)
                    .bind(-1i32)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error inserint plan enterprise (Postgres): {}", e))?;
            }
            DatabasePool::Sqlite(pool) => {
                let insert = "INSERT INTO plans (id, name, display_name, description, max_servers, max_channels_text_per_server, max_channels_voice_per_server, max_members_per_server, api_calls_per_minute, messages_per_day, max_storage_bytes, max_transfer_bytes_monthly, max_streaming_hours_monthly) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(name) DO UPDATE SET max_storage_bytes = excluded.max_storage_bytes, max_transfer_bytes_monthly = excluded.max_transfer_bytes_monthly, max_streaming_hours_monthly = excluded.max_streaming_hours_monthly";
                sqlx::query(insert)
                    .bind(free_id)
                    .bind("free")
                    .bind("Free")
                    .bind("Plan gratuït")
                    .bind(1i32)
                    .bind(3i32)
                    .bind(2i32)
                    .bind(20i32)
                    .bind(60i32)
                    .bind(10000i32)
                    .bind(FREE_STORAGE)
                    .bind(FREE_TRANSFER)
                    .bind(FREE_STREAMING)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error inserint plan free (SQLite): {}", e))?;

                sqlx::query(insert)
                    .bind(pro_id)
                    .bind("pro")
                    .bind("Pro")
                    .bind("Plan professional")
                    .bind(5i32)
                    .bind(20i32)
                    .bind(10i32)
                    .bind(500i32)
                    .bind(600i32)
                    .bind(-1i32)
                    .bind(PRO_STORAGE)
                    .bind(PRO_TRANSFER)
                    .bind(PRO_STREAMING)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error inserint plan pro (SQLite): {}", e))?;

                sqlx::query(insert)
                    .bind(enterprise_id)
                    .bind("enterprise")
                    .bind("Enterprise")
                    .bind("Plan enterprise")
                    .bind(-1i32)
                    .bind(-1i32)
                    .bind(-1i32)
                    .bind(-1i32)
                    .bind(-1i32)
                    .bind(-1i32)
                    .bind(-1i64)
                    .bind(-1i64)
                    .bind(-1i32)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error inserint plan enterprise (SQLite): {}", e))?;
            }
        }

        self.ensure_users_have_default_plan().await
    }

    pub async fn ensure_users_have_default_plan(&self) -> Result<(), String> {
        let free_plan_id = self
            .get_plan_id_by_name("free")
            .await?
            .ok_or_else(|| "No s'ha trobat el pla 'free'".to_string())?;

        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE users SET plan_id = $1 WHERE plan_id IS NULL")
                    .bind(free_plan_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error assignant pla free (Postgres): {}", e))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("UPDATE users SET plan_id = ? WHERE plan_id IS NULL")
                    .bind(free_plan_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error assignant pla free (SQLite): {}", e))?;
            }
        }

        Ok(())
    }

    pub async fn get_plan_id_by_name(&self, name: &str) -> Result<Option<Uuid>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT id FROM plans WHERE name = $1")
                    .bind(name)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| format!("Error obtenint plan_id (Postgres): {}", e))?;
                Ok(row.map(|r| r.get(0)))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT id FROM plans WHERE name = ?")
                    .bind(name)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| format!("Error obtenint plan_id (SQLite): {}", e))?;
                Ok(row.map(|r| r.get(0)))
            }
        }
    }

    #[allow(dead_code)]
    pub async fn set_user_plan_by_name(&self, user_id: Uuid, plan_name: &str) -> Result<(), String> {
        let plan_id = self
            .get_plan_id_by_name(plan_name)
            .await?
            .ok_or_else(|| format!("No s'ha trobat el pla '{}'", plan_name))?;

        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE users SET plan_id = $1 WHERE id = $2")
                    .bind(plan_id)
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error canviant pla d'usuari (Postgres): {}", e))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("UPDATE users SET plan_id = ? WHERE id = ?")
                    .bind(plan_id)
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error canviant pla d'usuari (SQLite): {}", e))?;
            }
        }

        Ok(())
    }

    pub async fn plan_exists_by_id(&self, plan_id: Uuid) -> Result<bool, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM plans WHERE id = $1)")
                    .bind(plan_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error comprovant plan_id (Postgres): {}", e))?;
                Ok(row.get(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM plans WHERE id = ?)")
                    .bind(plan_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error comprovant plan_id (SQLite): {}", e))?;
                Ok(row.get(0))
            }
        }
    }

    pub async fn set_user_plan_by_id(&self, user_id: Uuid, plan_id: Uuid) -> Result<bool, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query("UPDATE users SET plan_id = $1 WHERE id = $2")
                    .bind(plan_id)
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error canviant pla per id (Postgres): {}", e))?;
                Ok(result.rows_affected() > 0)
            }
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query("UPDATE users SET plan_id = ? WHERE id = ?")
                    .bind(plan_id)
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error canviant pla per id (SQLite): {}", e))?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    pub async fn get_plan_by_id_admin(
        &self,
        plan_id: Uuid,
    ) -> Result<Option<(Uuid, String, String, Option<String>, i32, i32, i32, i32, i32, i32, i64, i64)>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, name, display_name, description, max_servers, max_channels_text_per_server, max_channels_voice_per_server, max_members_per_server, api_calls_per_minute, messages_per_day, max_storage_bytes, max_transfer_bytes_monthly FROM plans WHERE id = $1",
                )
                .bind(plan_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Error obtenint plan per id (Postgres): {}", e))?;

                Ok(row.map(|r| {
                    (
                        r.get(0),
                        r.get(1),
                        r.get(2),
                        r.get(3),
                        r.get(4),
                        r.get(5),
                        r.get(6),
                        r.get(7),
                        r.get(8),
                        r.get(9),
                        r.get(10),
                        r.get(11),
                    )
                }))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, name, display_name, description, max_servers, max_channels_text_per_server, max_channels_voice_per_server, max_members_per_server, api_calls_per_minute, messages_per_day, max_storage_bytes, max_transfer_bytes_monthly FROM plans WHERE id = ?",
                )
                .bind(plan_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Error obtenint plan per id (SQLite): {}", e))?;

                Ok(row.map(|r| {
                    (
                        r.get(0),
                        r.get(1),
                        r.get(2),
                        r.get(3),
                        r.get(4),
                        r.get(5),
                        r.get(6),
                        r.get(7),
                        r.get(8),
                        r.get(9),
                        r.get(10),
                        r.get(11),
                    )
                }))
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_plan_admin(
        &self,
        plan_id: Uuid,
        name: &str,
        display_name: &str,
        description: Option<&str>,
        max_servers: i32,
        max_channels_text_per_server: i32,
        max_channels_voice_per_server: i32,
        max_members_per_server: i32,
        api_calls_per_minute: i32,
        messages_per_day: i32,
    ) -> Result<(), String> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO plans (id, name, display_name, description, max_servers, max_channels_text_per_server, max_channels_voice_per_server, max_members_per_server, api_calls_per_minute, messages_per_day) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                )
                .bind(plan_id)
                .bind(name)
                .bind(display_name)
                .bind(description)
                .bind(max_servers)
                .bind(max_channels_text_per_server)
                .bind(max_channels_voice_per_server)
                .bind(max_members_per_server)
                .bind(api_calls_per_minute)
                .bind(messages_per_day)
                .execute(pool)
                .await
                .map_err(|e| format!("Error creant plan (Postgres): {}", e))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO plans (id, name, display_name, description, max_servers, max_channels_text_per_server, max_channels_voice_per_server, max_members_per_server, api_calls_per_minute, messages_per_day) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(plan_id)
                .bind(name)
                .bind(display_name)
                .bind(description)
                .bind(max_servers)
                .bind(max_channels_text_per_server)
                .bind(max_channels_voice_per_server)
                .bind(max_members_per_server)
                .bind(api_calls_per_minute)
                .bind(messages_per_day)
                .execute(pool)
                .await
                .map_err(|e| format!("Error creant plan (SQLite): {}", e))?;
            }
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update_plan_by_id(
        &self,
        plan_id: Uuid,
        name: &str,
        display_name: &str,
        description: Option<&str>,
        max_servers: i32,
        max_channels_text_per_server: i32,
        max_channels_voice_per_server: i32,
        max_members_per_server: i32,
        api_calls_per_minute: i32,
        messages_per_day: i32,
    ) -> Result<bool, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query(
                    "UPDATE plans SET name = $1, display_name = $2, description = $3, max_servers = $4, max_channels_text_per_server = $5, max_channels_voice_per_server = $6, max_members_per_server = $7, api_calls_per_minute = $8, messages_per_day = $9 WHERE id = $10",
                )
                .bind(name)
                .bind(display_name)
                .bind(description)
                .bind(max_servers)
                .bind(max_channels_text_per_server)
                .bind(max_channels_voice_per_server)
                .bind(max_members_per_server)
                .bind(api_calls_per_minute)
                .bind(messages_per_day)
                .bind(plan_id)
                .execute(pool)
                .await
                .map_err(|e| format!("Error actualitzant plan (Postgres): {}", e))?;
                Ok(result.rows_affected() > 0)
            }
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query(
                    "UPDATE plans SET name = ?, display_name = ?, description = ?, max_servers = ?, max_channels_text_per_server = ?, max_channels_voice_per_server = ?, max_members_per_server = ?, api_calls_per_minute = ?, messages_per_day = ? WHERE id = ?",
                )
                .bind(name)
                .bind(display_name)
                .bind(description)
                .bind(max_servers)
                .bind(max_channels_text_per_server)
                .bind(max_channels_voice_per_server)
                .bind(max_members_per_server)
                .bind(api_calls_per_minute)
                .bind(messages_per_day)
                .bind(plan_id)
                .execute(pool)
                .await
                .map_err(|e| format!("Error actualitzant plan (SQLite): {}", e))?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    pub async fn delete_plan_by_id(&self, plan_id: Uuid) -> Result<bool, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query("DELETE FROM plans WHERE id = $1")
                    .bind(plan_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error eliminant plan (Postgres): {}", e))?;
                Ok(result.rows_affected() > 0)
            }
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query("DELETE FROM plans WHERE id = ?")
                    .bind(plan_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error eliminant plan (SQLite): {}", e))?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    pub async fn count_users_with_plan(&self, plan_id: Uuid) -> Result<i64, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT COUNT(*) FROM users WHERE plan_id = $1")
                    .bind(plan_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error comptant usuaris per plan (Postgres): {}", e))?;
                Ok(row.get(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT COUNT(*) FROM users WHERE plan_id = ?")
                    .bind(plan_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error comptant usuaris per plan (SQLite): {}", e))?;
                Ok(row.get(0))
            }
        }
    }

    pub async fn list_all_users_admin(&self) -> Result<Vec<(Uuid, String, String, Option<Uuid>)>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query("SELECT id, username, role, plan_id FROM users ORDER BY created_at ASC")
                    .fetch_all(pool)
                    .await
                    .map_err(|e| format!("Error llistant usuaris (Postgres): {}", e))?;
                Ok(rows
                    .into_iter()
                    .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3)))
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT id, username, role, plan_id FROM users ORDER BY created_at ASC")
                    .fetch_all(pool)
                    .await
                    .map_err(|e| format!("Error llistant usuaris (SQLite): {}", e))?;
                Ok(rows
                    .into_iter()
                    .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3)))
                    .collect())
            }
        }
    }

    pub async fn get_user_max_servers(&self, user_id: Uuid) -> Result<i32, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT p.max_servers FROM users u JOIN plans p ON p.id = u.plan_id WHERE u.id = $1",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Error obtenint límit de servidors (Postgres): {}", e))?;
                Ok(row.map(|r| r.get(0)).unwrap_or(1))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT p.max_servers FROM users u JOIN plans p ON p.id = u.plan_id WHERE u.id = ?",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Error obtenint límit de servidors (SQLite): {}", e))?;
                Ok(row.map(|r| r.get(0)).unwrap_or(1))
            }
        }
    }

    pub async fn get_user_channel_limits(&self, user_id: Uuid) -> Result<(i32, i32), String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT p.max_channels_text_per_server, p.max_channels_voice_per_server FROM users u JOIN plans p ON p.id = u.plan_id WHERE u.id = $1",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Error obtenint límits de canals (Postgres): {}", e))?;
                Ok(row.map(|r| (r.get(0), r.get(1))).unwrap_or((3, 2)))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT p.max_channels_text_per_server, p.max_channels_voice_per_server FROM users u JOIN plans p ON p.id = u.plan_id WHERE u.id = ?",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Error obtenint límits de canals (SQLite): {}", e))?;
                Ok(row.map(|r| (r.get(0), r.get(1))).unwrap_or((3, 2)))
            }
        }
    }

    pub async fn get_user_plan_limits(
        &self,
        user_id: Uuid,
    ) -> Result<(Uuid, String, String, Option<String>, i32, i32, i32, i32, i32, i32, i64, i64), String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT p.id, p.name, p.display_name, p.description, p.max_servers, p.max_channels_text_per_server, p.max_channels_voice_per_server, p.max_members_per_server, p.api_calls_per_minute, p.messages_per_day, p.max_storage_bytes, p.max_transfer_bytes_monthly FROM users u JOIN plans p ON p.id = u.plan_id WHERE u.id = $1",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Error obtenint límits del pla (Postgres): {}", e))?;

                row.map(|r| {
                    Ok((
                        r.get(0),
                        r.get(1),
                        r.get(2),
                        r.get(3),
                        r.get(4),
                        r.get(5),
                        r.get(6),
                        r.get(7),
                        r.get(8),
                        r.get(9),
                        r.get(10),
                        r.get(11),
                    ))
                })
                .unwrap_or_else(|| Err("No s'ha trobat pla per a l'usuari".to_string()))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT p.id, p.name, p.display_name, p.description, p.max_servers, p.max_channels_text_per_server, p.max_channels_voice_per_server, p.max_members_per_server, p.api_calls_per_minute, p.messages_per_day, p.max_storage_bytes, p.max_transfer_bytes_monthly FROM users u JOIN plans p ON p.id = u.plan_id WHERE u.id = ?",
                )
                .bind(user_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Error obtenint límits del pla (SQLite): {}", e))?;

                row.map(|r| {
                    Ok((
                        r.get(0),
                        r.get(1),
                        r.get(2),
                        r.get(3),
                        r.get(4),
                        r.get(5),
                        r.get(6),
                        r.get(7),
                        r.get(8),
                        r.get(9),
                        r.get(10),
                        r.get(11),
                    ))
                })
                .unwrap_or_else(|| Err("No s'ha trobat pla per a l'usuari".to_string()))
            }
        }
    }

    pub async fn list_plans_admin(
        &self,
    ) -> Result<Vec<(Uuid, String, String, Option<String>, i32, i32, i32, i32, i32, i32, i64, i64)>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT id, name, display_name, description, max_servers, max_channels_text_per_server, max_channels_voice_per_server, max_members_per_server, api_calls_per_minute, messages_per_day, max_storage_bytes, max_transfer_bytes_monthly FROM plans ORDER BY created_at ASC",
                )
                .fetch_all(pool)
                .await
                .map_err(|e| format!("Error llistant plans (Postgres): {}", e))?;

                Ok(rows
                    .into_iter()
                    .map(|r| {
                        (
                            r.get(0),
                            r.get(1),
                            r.get(2),
                            r.get(3),
                            r.get(4),
                            r.get(5),
                            r.get(6),
                            r.get(7),
                            r.get(8),
                            r.get(9),
                            r.get(10),
                            r.get(11),
                        )
                    })
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT id, name, display_name, description, max_servers, max_channels_text_per_server, max_channels_voice_per_server, max_members_per_server, api_calls_per_minute, messages_per_day, max_storage_bytes, max_transfer_bytes_monthly FROM plans ORDER BY created_at ASC",
                )
                .fetch_all(pool)
                .await
                .map_err(|e| format!("Error llistant plans (SQLite): {}", e))?;

                Ok(rows
                    .into_iter()
                    .map(|r| {
                        (
                            r.get(0),
                            r.get(1),
                            r.get(2),
                            r.get(3),
                            r.get(4),
                            r.get(5),
                            r.get(6),
                            r.get(7),
                            r.get(8),
                            r.get(9),
                            r.get(10),
                            r.get(11),
                        )
                    })
                    .collect())
            }
        }
    }

    pub async fn count_channels_by_type_in_server(&self, server_id: Uuid, channel_type: &str) -> Result<i64, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT COUNT(*) FROM channels WHERE server_id = $1 AND channel_type = $2")
                    .bind(server_id)
                    .bind(channel_type)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error comptant canals (Postgres): {}", e))?;
                Ok(row.get(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT COUNT(*) FROM channels WHERE server_id = ? AND type = ?")
                    .bind(server_id)
                    .bind(channel_type)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error comptant canals (SQLite): {}", e))?;
                Ok(row.get(0))
            }
        }
    }

    pub async fn count_owned_servers(&self, user_id: Uuid) -> Result<i64, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT COUNT(*) FROM servers WHERE owner_id = $1")
                    .bind(user_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error comptant servidors (Postgres): {}", e))?;
                Ok(row.get(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT COUNT(*) FROM servers WHERE owner_id = ?")
                    .bind(user_id)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error comptant servidors (SQLite): {}", e))?;
                Ok(row.get(0))
            }
        }
    }

    pub async fn count_owned_channels_by_type(&self, user_id: Uuid, channel_type: &str) -> Result<i64, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT COUNT(*) FROM channels c JOIN servers s ON c.server_id = s.id WHERE s.owner_id = $1 AND c.channel_type = $2",
                )
                .bind(user_id)
                .bind(channel_type)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("Error comptant canals propietari (Postgres): {}", e))?;
                Ok(row.get(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT COUNT(*) FROM channels c JOIN servers s ON c.server_id = s.id WHERE s.owner_id = ? AND c.type = ?",
                )
                .bind(user_id)
                .bind(channel_type)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("Error comptant canals propietari (SQLite): {}", e))?;
                Ok(row.get(0))
            }
        }
    }

    pub async fn channel_name_exists_in_server(&self, server_id: Uuid, name: &str) -> Result<bool, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM channels WHERE server_id = $1 AND name = $2)")
                    .bind(server_id)
                    .bind(name)
                    .fetch_one(pool)
                    .await?;
                Ok(row.get::<bool, _>(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM channels WHERE server_id = ? AND name = ?)")
                    .bind(server_id)
                    .bind(name)
                    .fetch_one(pool)
                    .await?;
                Ok(row.get::<bool, _>(0))
            }
        }
    }

    pub async fn count_members_in_owned_servers(&self, user_id: Uuid) -> Result<i64, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT COUNT(*) FROM server_members sm JOIN servers s ON sm.server_id = s.id WHERE s.owner_id = $1",
                )
                .bind(user_id)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("Error comptant membres en servidors propis (Postgres): {}", e))?;
                Ok(row.get(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT COUNT(*) FROM server_members sm JOIN servers s ON sm.server_id = s.id WHERE s.owner_id = ?",
                )
                .bind(user_id)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("Error comptant membres en servidors propis (SQLite): {}", e))?;
                Ok(row.get(0))
            }
        }
    }

    pub async fn count_user_messages_today(&self, user_id: Uuid) -> Result<i64, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT COUNT(*) FROM messages WHERE sender_user_id = $1 AND timestamp >= date_trunc('day', NOW()) AND timestamp < date_trunc('day', NOW()) + interval '1 day'",
                )
                .bind(user_id)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("Error comptant missatges d'avui (Postgres): {}", e))?;
                Ok(row.get(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT COUNT(*) FROM messages WHERE sender_user_id = ? AND DATE(timestamp) = DATE('now')",
                )
                .bind(user_id)
                .fetch_one(pool)
                .await
                .map_err(|e| format!("Error comptant missatges d'avui (SQLite): {}", e))?;
                Ok(row.get(0))
            }
        }
    }

    pub async fn find_user_auth_by_username(
        &self,
        username: &str,
    ) -> Result<Option<(Uuid, String, String, bool)>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT id, username, password_hash, role FROM users WHERE username = $1")
                    .bind(username)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| format!("Error PostgreSQL: {}", e))?;
                Ok(row.map(|r| {
                    let role: String = r.get(3);
                    (r.get(0), r.get(1), r.get(2), role == "admin")
                }))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT id, username, password_hash, role FROM users WHERE username = ?")
                    .bind(username)
                    .fetch_optional(pool)
                    .await
                    .map_err(|e| format!("Error SQLite: {}", e))?;
                Ok(row.map(|r| {
                    let role: String = r.get(3);
                    (r.get(0), r.get(1), r.get(2), role == "admin")
                }))
            }
        }
    }

    pub async fn create_invitation(
        &self,
        code: &str,
        created_by_user_id: Uuid,
        server_id: Option<Uuid>,
        max_uses: i32,
    ) -> Result<Uuid, String> {
        let id = Uuid::new_v4();
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO invitations (id, code, created_by_user_id, server_id, max_uses, uses_count, is_active) VALUES ($1, $2, $3, $4, $5, 0, true)",
                )
                .bind(id)
                .bind(code)
                .bind(created_by_user_id)
                .bind(server_id)
                .bind(max_uses)
                .execute(pool)
                .await
                .map_err(|e| format!("Error PostgreSQL: {}", e))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO invitations (id, code, created_by_user_id, server_id, max_uses, uses_count, is_active) VALUES (?, ?, ?, ?, ?, 0, 1)",
                )
                .bind(id)
                .bind(code)
                .bind(created_by_user_id)
                .bind(server_id)
                .bind(max_uses)
                .execute(pool)
                .await
                .map_err(|e| format!("Error SQLite: {}", e))?;
            }
        }
        Ok(id)
    }

    pub async fn find_active_invitation_by_code(
        &self,
        code: &str,
    ) -> Result<Option<(Uuid, Option<Uuid>, i32, i32, bool)>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, server_id, max_uses, uses_count, is_active FROM invitations WHERE code = $1",
                )
                .bind(code)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Error PostgreSQL: {}", e))?;

                Ok(row.map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4))))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, server_id, max_uses, uses_count, is_active FROM invitations WHERE code = ?",
                )
                .bind(code)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Error SQLite: {}", e))?;

                Ok(row.map(|r| {
                    let is_active: i64 = r.get(4);
                    (r.get(0), r.get(1), r.get(2), r.get(3), is_active != 0)
                }))
            }
        }
    }

    pub async fn increment_invitation_uses(&self, invitation_id: Uuid) -> Result<(), String> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE invitations SET uses_count = uses_count + 1 WHERE id = $1")
                    .bind(invitation_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error PostgreSQL: {}", e))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("UPDATE invitations SET uses_count = uses_count + 1 WHERE id = ?")
                    .bind(invitation_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error SQLite: {}", e))?;
            }
        }
        Ok(())
    }

    pub async fn list_invitations_admin(&self) -> Result<Vec<(Uuid, String, Option<Uuid>, i32, i32, bool, String)>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT i.id, i.code, i.server_id, i.max_uses, i.uses_count, i.is_active, u.username \
                     FROM invitations i \
                     JOIN users u ON u.id = i.created_by_user_id \
                     ORDER BY i.created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(|e| format!("Error PostgreSQL: {}", e))?;

                Ok(rows
                    .into_iter()
                    .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4), r.get(5), r.get(6)))
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT i.id, i.code, i.server_id, i.max_uses, i.uses_count, i.is_active, u.username \
                     FROM invitations i \
                     JOIN users u ON u.id = i.created_by_user_id \
                     ORDER BY i.created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(|e| format!("Error SQLite: {}", e))?;

                Ok(rows
                    .into_iter()
                    .map(|r| {
                        let is_active: i64 = r.get(5);
                        (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4), is_active != 0, r.get(6))
                    })
                    .collect())
            }
        }
    }

    pub async fn sync_one_admin_invitation_hash(&self, code_hash: &str) -> Result<(), String> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO admin_bootstrap_invitation (slot, code_hash, consumed_by_user_id, consumed_at, created_at, updated_at) \
                     VALUES (1, $1, NULL, NULL, NOW(), NOW()) \
                     ON CONFLICT (slot) DO UPDATE SET \
                         code_hash = CASE \
                             WHEN admin_bootstrap_invitation.code_hash = EXCLUDED.code_hash THEN admin_bootstrap_invitation.code_hash \
                             ELSE EXCLUDED.code_hash \
                         END, \
                         consumed_by_user_id = CASE \
                             WHEN admin_bootstrap_invitation.code_hash = EXCLUDED.code_hash THEN admin_bootstrap_invitation.consumed_by_user_id \
                             ELSE NULL \
                         END, \
                         consumed_at = CASE \
                             WHEN admin_bootstrap_invitation.code_hash = EXCLUDED.code_hash THEN admin_bootstrap_invitation.consumed_at \
                             ELSE NULL \
                         END, \
                         updated_at = NOW()",
                )
                .bind(code_hash)
                .execute(pool)
                .await
                .map_err(|e| format!("Error PostgreSQL: {}", e))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO admin_bootstrap_invitation (slot, code_hash, consumed_by_user_id, consumed_at, created_at, updated_at) \
                     VALUES (1, ?, NULL, NULL, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP) \
                     ON CONFLICT(slot) DO UPDATE SET \
                         code_hash = CASE \
                             WHEN admin_bootstrap_invitation.code_hash = excluded.code_hash THEN admin_bootstrap_invitation.code_hash \
                             ELSE excluded.code_hash \
                         END, \
                         consumed_by_user_id = CASE \
                             WHEN admin_bootstrap_invitation.code_hash = excluded.code_hash THEN admin_bootstrap_invitation.consumed_by_user_id \
                             ELSE NULL \
                         END, \
                         consumed_at = CASE \
                             WHEN admin_bootstrap_invitation.code_hash = excluded.code_hash THEN admin_bootstrap_invitation.consumed_at \
                             ELSE NULL \
                         END, \
                         updated_at = CURRENT_TIMESTAMP",
                )
                .bind(code_hash)
                .execute(pool)
                .await
                .map_err(|e| format!("Error SQLite: {}", e))?;
            }
        }

        Ok(())
    }

    pub async fn consume_one_admin_invitation_hash(
        &self,
        code_hash: &str,
        consumed_by_user_id: Uuid,
    ) -> Result<bool, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query(
                    "UPDATE admin_bootstrap_invitation \
                     SET consumed_by_user_id = $1, consumed_at = NOW(), updated_at = NOW() \
                     WHERE slot = 1 AND code_hash = $2 AND consumed_at IS NULL",
                )
                .bind(consumed_by_user_id)
                .bind(code_hash)
                .execute(pool)
                .await
                .map_err(|e| format!("Error PostgreSQL: {}", e))?;
                Ok(result.rows_affected() > 0)
            }
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query(
                    "UPDATE admin_bootstrap_invitation \
                     SET consumed_by_user_id = ?, consumed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP \
                     WHERE slot = 1 AND code_hash = ? AND consumed_at IS NULL",
                )
                .bind(consumed_by_user_id)
                .bind(code_hash)
                .execute(pool)
                .await
                .map_err(|e| format!("Error SQLite: {}", e))?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    /// Comprovar si un usuari ja existeix.
    pub async fn user_exists(&self, username: &str) -> Result<bool, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let exists = sqlx::query("SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)")
                    .bind(username)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error PostgreSQL: {}", e))?;
                Ok(exists.get::<bool, _>(0))
            }
            DatabasePool::Sqlite(pool) => {
                let exists = sqlx::query("SELECT EXISTS(SELECT 1 FROM users WHERE username = ?)")
                    .bind(username)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error SQLite: {}", e))?;
                Ok(exists.get::<bool, _>(0))
            }
        }
    }

    pub async fn list_friends_for_user(&self, user_id: Uuid) -> Result<Vec<(Uuid, String)>, sqlx::Error> {
        let query = "SELECT u.id, u.username FROM friendships f JOIN users u ON u.id = f.friend_user_id WHERE f.owner_user_id = $1 ORDER BY u.username ASC";

        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(query).bind(user_id).fetch_all(pool).await?;
                Ok(rows.into_iter().map(|row| (row.get(0), row.get(1))).collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(&query.replace("$1", "?")).bind(user_id).fetch_all(pool).await?;
                Ok(rows.into_iter().map(|row| (row.get(0), row.get(1))).collect())
            }
        }
    }

    pub async fn add_friend_for_user(&self, owner_user_id: Uuid, friend_user_id: Uuid) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO friendships (owner_user_id, friend_user_id) VALUES ($1, $2) ON CONFLICT (owner_user_id, friend_user_id) DO NOTHING",
                )
                .bind(owner_user_id)
                .bind(friend_user_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO friendships (owner_user_id, friend_user_id) VALUES (?, ?) ON CONFLICT(owner_user_id, friend_user_id) DO NOTHING",
                )
                .bind(owner_user_id)
                .bind(friend_user_id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn remove_friend_for_user(&self, owner_user_id: Uuid, friend_user_id: Uuid) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("DELETE FROM friendships WHERE owner_user_id = $1 AND friend_user_id = $2")
                    .bind(owner_user_id)
                    .bind(friend_user_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("DELETE FROM friendships WHERE owner_user_id = ? AND friend_user_id = ?")
                    .bind(owner_user_id)
                    .bind(friend_user_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn list_friend_owner_ids_for_user(&self, friend_user_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query("SELECT owner_user_id FROM friendships WHERE friend_user_id = $1")
                    .bind(friend_user_id)
                    .fetch_all(pool)
                    .await?;
                Ok(rows.into_iter().map(|row| row.get(0)).collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query("SELECT owner_user_id FROM friendships WHERE friend_user_id = ?")
                    .bind(friend_user_id)
                    .fetch_all(pool)
                    .await?;
                Ok(rows.into_iter().map(|row| row.get(0)).collect())
            }
        }
    }

    pub async fn search_users_for_user(
        &self,
        current_user_id: Uuid,
        query: &str,
        limit: i64,
    ) -> Result<Vec<(Uuid, String, bool)>, sqlx::Error> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }

        let like_query = format!("%{}%", trimmed);
        let sql_postgres = r#"
            SELECT u.id, u.username, (f.friend_user_id IS NOT NULL) AS is_friend
            FROM users u
            LEFT JOIN friendships f
              ON f.owner_user_id = $1
             AND f.friend_user_id = u.id
            WHERE u.id <> $1
              AND u.username ILIKE $2
            ORDER BY u.username ASC
            LIMIT $3
        "#;
        let sql_sqlite = r#"
            SELECT u.id, u.username, CASE WHEN f.friend_user_id IS NOT NULL THEN 1 ELSE 0 END AS is_friend
            FROM users u
            LEFT JOIN friendships f
              ON f.owner_user_id = ?
             AND f.friend_user_id = u.id
            WHERE u.id <> ?
              AND u.username LIKE ? COLLATE NOCASE
            ORDER BY u.username ASC
            LIMIT ?
        "#;

        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(sql_postgres)
                    .bind(current_user_id)
                    .bind(like_query)
                    .bind(limit)
                    .fetch_all(pool)
                    .await?;
                Ok(rows
                    .into_iter()
                    .map(|row| (row.get(0), row.get(1), row.get::<bool, _>(2)))
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(sql_sqlite)
                    .bind(current_user_id)
                    .bind(current_user_id)
                    .bind(like_query)
                    .bind(limit)
                    .fetch_all(pool)
                    .await?;
                Ok(rows
                    .into_iter()
                    .map(|row| {
                        let is_friend: i64 = row.get(2);
                        (row.get(0), row.get(1), is_friend != 0)
                    })
                    .collect())
            }
        }
    }

    /// Comprovar connexió.
    #[allow(dead_code)]
    pub async fn check_connection(&self) -> Result<(), String> {
        self.execute_query("SELECT 1").await
    }

    #[allow(dead_code)]
    pub async fn count_users(&self) -> Result<i64, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT COUNT(*) FROM users")
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error PostgreSQL: {}", e))?;
                Ok(row.get::<i64, _>(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT COUNT(*) FROM users")
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error SQLite: {}", e))?;
                Ok(row.get::<i64, _>(0))
            }
        }
    }

    pub async fn update_user_role_by_username(&self, username: &str, role: &str) -> Result<(), String> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE users SET role = $1 WHERE username = $2")
                    .bind(role)
                    .bind(username)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error PostgreSQL: {}", e))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("UPDATE users SET role = ? WHERE username = ?")
                    .bind(role)
                    .bind(username)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error SQLite: {}", e))?;
            }
        }
        Ok(())
    }

    pub async fn update_user_role_by_id(&self, user_id: Uuid, role: &str) -> Result<bool, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
                    .bind(role)
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error PostgreSQL: {}", e))?;
                Ok(result.rows_affected() > 0)
            }
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query("UPDATE users SET role = ? WHERE id = ?")
                    .bind(role)
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error SQLite: {}", e))?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    pub async fn update_user_password_hash_by_id(&self, user_id: Uuid, password_hash: &str) -> Result<bool, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
                    .bind(password_hash)
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error PostgreSQL: {}", e))?;
                Ok(result.rows_affected() > 0)
            }
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
                    .bind(password_hash)
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error SQLite: {}", e))?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    pub async fn delete_user_by_id(&self, user_id: Uuid) -> Result<bool, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query("DELETE FROM users WHERE id = $1")
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error PostgreSQL: {}", e))?;
                Ok(result.rows_affected() > 0)
            }
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query("DELETE FROM users WHERE id = ?")
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error SQLite: {}", e))?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

}
