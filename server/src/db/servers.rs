use super::*;
use sqlx::Row;
use uuid::Uuid;
use shared::types::{ServerInfo, ServerFullInfo, ServerLiveKitConfig, ServerMember as SharedServerMember, ServerRole};

impl DatabasePool {
    pub async fn list_servers_for_user(&self, user_id: Uuid) -> Result<Vec<ServerInfo>, sqlx::Error> {
        let query = "SELECT s.id, s.name, s.icon_url, s.owner_id, COUNT(sm2.user_id) as member_count, sm.role as my_role, s.created_at
            FROM servers s
            JOIN server_members sm ON sm.server_id = s.id AND sm.user_id = $1
            JOIN server_members sm2 ON sm2.server_id = s.id
            GROUP BY s.id, s.name, s.icon_url, s.owner_id, sm.role, s.created_at
            ORDER BY s.created_at DESC";

        let mut servers = Vec::new();
        match self {
            DatabasePool::Postgres(pool) => {
                let query_pg = "SELECT s.id, s.name, s.icon_url, s.owner_id, COUNT(sm2.user_id) as member_count, sm.role as my_role, s.created_at::text
                    FROM servers s
                    JOIN server_members sm ON sm.server_id = s.id AND sm.user_id = $1
                    JOIN server_members sm2 ON sm2.server_id = s.id
                    GROUP BY s.id, s.name, s.icon_url, s.owner_id, sm.role, s.created_at
                    ORDER BY s.created_at DESC";
                let rows = sqlx::query(query_pg).bind(user_id).fetch_all(pool).await?;
                for row in rows {
                    let my_role = row.get::<String, _>(5);
                    servers.push(ServerInfo {
                        server_id: row.get(0),
                        name: row.get(1),
                        icon_url: row.get(2),
                        owner_id: row.get(3),
                        member_count: row.get::<i64, _>(4) as u32,
                        my_role: parse_server_role(&my_role),
                        created_at: row.get::<String, _>(6),
                    });
                }
            }
            DatabasePool::Sqlite(pool) => {
                let query = query.replace("$1", "?");
                let rows = sqlx::query(&query).bind(user_id).fetch_all(pool).await?;
                for row in rows {
                    let my_role = row.get::<String, _>(5);
                    servers.push(ServerInfo {
                        server_id: row.get(0),
                        name: row.get(1),
                        icon_url: row.get(2),
                        owner_id: row.get(3),
                        member_count: row.get::<i64, _>(4) as u32,
                        my_role: parse_server_role(&my_role),
                        created_at: row.get::<String, _>(6),
                    });
                }
            }
        }
        Ok(servers)
    }

    pub async fn list_all_servers_admin(&self) -> Result<Vec<(Uuid, String, Option<String>, Uuid, u32, Option<String>, Option<String>, String)>, sqlx::Error> {
        let query = "SELECT s.id, s.name, s.icon_url, s.owner_id, COUNT(sm.user_id) as member_count, s.livekit_host, s.livekit_api_key, s.created_at
            FROM servers s
            LEFT JOIN server_members sm ON sm.server_id = s.id
            GROUP BY s.id, s.name, s.icon_url, s.owner_id, s.livekit_host, s.livekit_api_key, s.created_at
            ORDER BY s.created_at DESC";

        let mut servers = Vec::new();
        match self {
            DatabasePool::Postgres(pool) => {
                let query_pg = "SELECT s.id, s.name, s.icon_url, s.owner_id, COUNT(sm.user_id) as member_count, s.livekit_host, s.livekit_api_key, s.created_at::text
                    FROM servers s
                    LEFT JOIN server_members sm ON sm.server_id = s.id
                    GROUP BY s.id, s.name, s.icon_url, s.owner_id, s.livekit_host, s.livekit_api_key, s.created_at
                    ORDER BY s.created_at DESC";
                let rows = sqlx::query(query_pg).fetch_all(pool).await?;
                for row in rows {
                    servers.push((
                        row.get(0),
                        row.get(1),
                        row.get(2),
                        row.get(3),
                        row.get::<i64, _>(4) as u32,
                        row.get(5),
                        row.get(6),
                        row.get::<String, _>(7),
                    ));
                }
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(query).fetch_all(pool).await?;
                for row in rows {
                    servers.push((
                        row.get(0),
                        row.get(1),
                        row.get(2),
                        row.get(3),
                        row.get::<i64, _>(4) as u32,
                        row.get(5),
                        row.get(6),
                        row.get::<String, _>(7),
                    ));
                }
            }
        }

        Ok(servers)
    }

    pub async fn server_name_exists(&self, name: &str) -> Result<bool, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM servers WHERE name = $1)")
                    .bind(name)
                    .fetch_one(pool)
                    .await?;
                Ok(row.get::<bool, _>(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM servers WHERE name = ?)")
                    .bind(name)
                    .fetch_one(pool)
                    .await?;
                Ok(row.get::<bool, _>(0))
            }
        }
    }

    pub async fn create_server_with_owner(&self, server_id: Uuid, name: &str, icon_url: Option<&String>, owner_id: Uuid) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO servers (id, name, icon_url, owner_id) VALUES ($1, $2, $3, $4)")
                    .bind(server_id)
                    .bind(name)
                    .bind(icon_url)
                    .bind(owner_id)
                    .execute(pool)
                    .await?;
                sqlx::query("INSERT INTO server_members (server_id, user_id, role) VALUES ($1, $2, $3)")
                    .bind(server_id)
                    .bind(owner_id)
                    .bind("owner")
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("INSERT INTO servers (id, name, icon_url, owner_id) VALUES (?, ?, ?, ?)")
                    .bind(server_id)
                    .bind(name)
                    .bind(icon_url)
                    .bind(owner_id)
                    .execute(pool)
                    .await?;
                sqlx::query("INSERT INTO server_members (id, server_id, user_id, role) VALUES (?, ?, ?, ?)")
                    .bind(Uuid::new_v4())
                    .bind(server_id)
                    .bind(owner_id)
                    .bind("owner")
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn update_server_metadata(&self, server_id: Uuid, name: Option<&str>, icon_url: Option<Option<&str>>) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                match (name, icon_url) {
                    (Some(n), Some(icon)) => {
                        sqlx::query("UPDATE servers SET name = $1, icon_url = $2 WHERE id = $3")
                            .bind(n)
                            .bind(icon)
                            .bind(server_id)
                            .execute(pool)
                            .await?;
                    }
                    (Some(n), None) => {
                        sqlx::query("UPDATE servers SET name = $1 WHERE id = $2")
                            .bind(n)
                            .bind(server_id)
                            .execute(pool)
                            .await?;
                    }
                    (None, Some(icon)) => {
                        sqlx::query("UPDATE servers SET icon_url = $1 WHERE id = $2")
                            .bind(icon)
                            .bind(server_id)
                            .execute(pool)
                            .await?;
                    }
                    (None, None) => {}
                }
            }
            DatabasePool::Sqlite(pool) => {
                match (name, icon_url) {
                    (Some(n), Some(icon)) => {
                        sqlx::query("UPDATE servers SET name = ?, icon_url = ? WHERE id = ?")
                            .bind(n)
                            .bind(icon)
                            .bind(server_id)
                            .execute(pool)
                            .await?;
                    }
                    (Some(n), None) => {
                        sqlx::query("UPDATE servers SET name = ? WHERE id = ?")
                            .bind(n)
                            .bind(server_id)
                            .execute(pool)
                            .await?;
                    }
                    (None, Some(icon)) => {
                        sqlx::query("UPDATE servers SET icon_url = ? WHERE id = ?")
                            .bind(icon)
                            .bind(server_id)
                            .execute(pool)
                            .await?;
                    }
                    (None, None) => {}
                }
            }
        }

        Ok(())
    }

    pub async fn set_server_livekit_override(
        &self,
        server_id: Uuid,
        host: Option<&str>,
        api_key: Option<&str>,
        api_secret: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE servers SET livekit_host = $1, livekit_api_key = $2, livekit_api_secret = $3 WHERE id = $4",
                )
                .bind(host)
                .bind(api_key)
                .bind(api_secret)
                .bind(server_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE servers SET livekit_host = ?, livekit_api_key = ?, livekit_api_secret = ? WHERE id = ?",
                )
                .bind(host)
                .bind(api_key)
                .bind(api_secret)
                .bind(server_id)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    pub async fn get_server_livekit_override(
        &self,
        server_id: Uuid,
    ) -> Result<Option<ServerLiveKitOverride>, sqlx::Error> {
        let values = match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT livekit_host, livekit_api_key, livekit_api_secret FROM servers WHERE id = $1",
                )
                .bind(server_id)
                .fetch_optional(pool)
                .await?;

                row.map(|row| {
                    (
                        row.get::<Option<String>, _>(0),
                        row.get::<Option<String>, _>(1),
                        row.get::<Option<String>, _>(2),
                    )
                })
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT livekit_host, livekit_api_key, livekit_api_secret FROM servers WHERE id = ?",
                )
                .bind(server_id)
                .fetch_optional(pool)
                .await?;

                row.map(|row| {
                    (
                        row.get::<Option<String>, _>(0),
                        row.get::<Option<String>, _>(1),
                        row.get::<Option<String>, _>(2),
                    )
                })
            }
        };

        let Some((host, api_key, api_secret)) = values else {
            return Ok(None);
        };

        Ok(match (host, api_key, api_secret) {
            (Some(host), Some(api_key), Some(api_secret)) => Some(ServerLiveKitOverride {
                host,
                api_key,
                api_secret,
            }),
            _ => None,
        })
    }

    pub async fn delete_server(&self, server_id: Uuid) -> Result<bool, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query("DELETE FROM servers WHERE id = $1")
                    .bind(server_id)
                    .execute(pool)
                    .await?;
                Ok(result.rows_affected() > 0)
            }
            DatabasePool::Sqlite(pool) => {
                // SQLite schema històric no garanteix cascades homogènies per totes les FK.
                // Eliminem dependències explícitament en una transacció.
                let mut tx = pool.begin().await?;

                sqlx::query(
                    "DELETE FROM channel_read_state WHERE channel_id IN (SELECT id FROM channels WHERE server_id = ?)",
                )
                .bind(server_id)
                .execute(&mut *tx)
                .await?;

                sqlx::query("DELETE FROM messages WHERE channel_id IN (SELECT id FROM channels WHERE server_id = ?)")
                    .bind(server_id)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query("DELETE FROM channel_members WHERE channel_id IN (SELECT id FROM channels WHERE server_id = ?)")
                    .bind(server_id)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query(
                    "DELETE FROM channel_key_device_bundles \
                     WHERE key_version_id IN (\
                        SELECT id FROM channel_key_versions WHERE channel_id IN (SELECT id FROM channels WHERE server_id = ?)\
                     )",
                )
                .bind(server_id)
                .execute(&mut *tx)
                .await?;

                sqlx::query("DELETE FROM channel_key_versions WHERE channel_id IN (SELECT id FROM channels WHERE server_id = ?)")
                    .bind(server_id)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query("DELETE FROM channels WHERE server_id = ?")
                    .bind(server_id)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query("DELETE FROM server_members WHERE server_id = ?")
                    .bind(server_id)
                    .execute(&mut *tx)
                    .await?;

                let result = sqlx::query("DELETE FROM servers WHERE id = ?")
                    .bind(server_id)
                    .execute(&mut *tx)
                    .await?;

                tx.commit().await?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    pub async fn get_server_full_info(&self, server_id: Uuid, user_id: Uuid, is_admin: bool) -> Result<Option<ServerFullInfo>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let server_row = sqlx::query("SELECT id, name, icon_url, owner_id, livekit_host, livekit_api_key, created_at::text FROM servers WHERE id = $1")
                    .bind(server_id)
                    .fetch_optional(pool)
                    .await?;

                let server_row = match server_row {
                    Some(row) => row,
                    None => return Ok(None),
                };

                // Get user's role in this server
                let my_role_row = sqlx::query("SELECT role FROM server_members WHERE server_id = $1 AND user_id = $2")
                    .bind(server_id)
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?;

                let my_role = match my_role_row {
                    Some(row) => parse_server_role(&row.get::<String, _>(0)),
                    None => if is_admin { ServerRole::Admin } else { ServerRole::Member },
                };

                let members = {
                    let rows = sqlx::query("SELECT u.id, u.username, sm.role, sm.joined_at::text FROM server_members sm JOIN users u ON u.id = sm.user_id WHERE sm.server_id = $1 ORDER BY sm.joined_at ASC")
                        .bind(server_id)
                        .fetch_all(pool)
                        .await?;
                    rows.into_iter().map(|row| SharedServerMember {
                        user_id: row.get(0),
                        username: row.get(1),
                        role: parse_server_role(&row.get::<String, _>(2)),
                        joined_at: row.get::<String, _>(3),
                    }).collect()
                };

                Ok(Some(ServerFullInfo {
                    server_id: server_row.get(0),
                    name: server_row.get(1),
                    icon_url: server_row.get(2),
                    owner_id: server_row.get(3),
                    my_role,
                    members,
                    livekit_config: match (
                        server_row.get::<Option<String>, _>(4),
                        server_row.get::<Option<String>, _>(5),
                    ) {
                        (Some(host), Some(api_key)) => Some(ServerLiveKitConfig {
                            host,
                            api_key,
                            is_override: true,
                        }),
                        _ => None,
                    },
                    created_at: server_row.get::<String, _>(6),
                }))
            }
            DatabasePool::Sqlite(pool) => {
                let server_row = sqlx::query("SELECT id, name, icon_url, owner_id, livekit_host, livekit_api_key, created_at FROM servers WHERE id = ?")
                    .bind(server_id)
                    .fetch_optional(pool)
                    .await?;

                let server_row = match server_row {
                    Some(row) => row,
                    None => return Ok(None),
                };

                // Get user's role in this server
                let my_role_row = sqlx::query("SELECT role FROM server_members WHERE server_id = ? AND user_id = ?")
                    .bind(server_id)
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?;

                let my_role = match my_role_row {
                    Some(row) => parse_server_role(&row.get::<String, _>(0)),
                    None => if is_admin { ServerRole::Admin } else { ServerRole::Member },
                };

                let members = {
                    let rows = sqlx::query("SELECT u.id, u.username, sm.role, sm.joined_at FROM server_members sm JOIN users u ON u.id = sm.user_id WHERE sm.server_id = ? ORDER BY sm.joined_at ASC")
                        .bind(server_id)
                        .fetch_all(pool)
                        .await?;
                    rows.into_iter().map(|row| SharedServerMember {
                        user_id: row.get(0),
                        username: row.get(1),
                        role: parse_server_role(&row.get::<String, _>(2)),
                        joined_at: row.get::<String, _>(3),
                    }).collect()
                };

                Ok(Some(ServerFullInfo {
                    server_id: server_row.get(0),
                    name: server_row.get(1),
                    icon_url: server_row.get(2),
                    owner_id: server_row.get(3),
                    my_role,
                    members,
                    livekit_config: match (
                        server_row.get::<Option<String>, _>(4),
                        server_row.get::<Option<String>, _>(5),
                    ) {
                        (Some(host), Some(api_key)) => Some(ServerLiveKitConfig {
                            host,
                            api_key,
                            is_override: true,
                        }),
                        _ => None,
                    },
                    created_at: server_row.get::<String, _>(6),
                }))
            }
        }
    }

    pub async fn is_server_member(&self, server_id: Uuid, user_id: Uuid) -> Result<Option<String>, sqlx::Error> {
        let role = match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT role FROM server_members WHERE server_id = $1 AND user_id = $2")
                    .bind(server_id)
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?;
                row.map(|r| r.get(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT role FROM server_members WHERE server_id = ? AND user_id = ?")
                    .bind(server_id)
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await?;
                row.map(|r| r.get(0))
            }
        };
        Ok(role)
    }

    pub async fn get_server_permission_level(&self, server_id: Uuid, user_id: Uuid) -> Result<Option<i32>, sqlx::Error> {
        let role = self.is_server_member(server_id, user_id).await?;
        Ok(role.map(|r| match r.as_str() {
            "owner" => SERVER_PERMISSION_MANAGE_MEMBERS,
            "admin" => SERVER_PERMISSION_MANAGE_MEMBERS,
            "member" => SERVER_PERMISSION_VIEW,
            _ => 0,
        }))
    }

    pub async fn add_server_member(&self, server_id: Uuid, user_id: Uuid, role: &str) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO server_members (server_id, user_id, role) VALUES ($1, $2, $3)")
                    .bind(server_id)
                    .bind(user_id)
                    .bind(role)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("INSERT INTO server_members (id, server_id, user_id, role) VALUES (?, ?, ?, ?)")
                    .bind(Uuid::new_v4())
                    .bind(server_id)
                    .bind(user_id)
                    .bind(role)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn update_server_member_role(&self, server_id: Uuid, user_id: Uuid, role: &str) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("UPDATE server_members SET role = $1 WHERE server_id = $2 AND user_id = $3")
                    .bind(role)
                    .bind(server_id)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query("UPDATE server_members SET role = ? WHERE server_id = ? AND user_id = ?")
                    .bind(role)
                    .bind(server_id)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    pub async fn count_server_admins(&self, server_id: Uuid) -> Result<i64, sqlx::Error> {
        let count = match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT COUNT(*) FROM server_members WHERE server_id = $1 AND role = 'admin'")
                    .bind(server_id)
                    .fetch_one(pool)
                    .await?;
                row.get::<i64, _>(0)
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query("SELECT COUNT(*) FROM server_members WHERE server_id = ? AND role = 'admin'")
                    .bind(server_id)
                    .fetch_one(pool)
                    .await?;
                row.get::<i64, _>(0)
            }
        };
        Ok(count)
    }

    pub async fn remove_server_member(&self, server_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let result = sqlx::query("DELETE FROM server_members WHERE server_id = $1 AND user_id = $2")
                    .bind(server_id)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
                Ok(result.rows_affected() > 0)
            }
            DatabasePool::Sqlite(pool) => {
                let result = sqlx::query("DELETE FROM server_members WHERE server_id = ? AND user_id = ?")
                    .bind(server_id)
                    .bind(user_id)
                    .execute(pool)
                    .await?;
                Ok(result.rows_affected() > 0)
            }
        }
    }

    pub async fn create_server_invitation(
        &self,
        invitation_id: Uuid,
        server_id: Uuid,
        inviter_id: Uuid,
        invitee_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO server_invitations (id, server_id, inviter_id, invitee_id) \
                     VALUES ($1, $2, $3, $4) \
                     ON CONFLICT DO NOTHING"
                )
                .bind(invitation_id)
                .bind(server_id)
                .bind(inviter_id)
                .bind(invitee_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT OR IGNORE INTO server_invitations (id, server_id, inviter_id, invitee_id) \
                     VALUES (?, ?, ?, ?)"
                )
                .bind(invitation_id)
                .bind(server_id)
                .bind(inviter_id)
                .bind(invitee_id)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_server_invitation(
        &self,
        invitation_id: Uuid,
    ) -> Result<Option<(Uuid, Uuid, Uuid, Uuid, String)>, sqlx::Error> {
        // Returns: (id, server_id, inviter_id, invitee_id, status)
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, server_id, inviter_id, invitee_id, status \
                     FROM server_invitations WHERE id = $1"
                )
                .bind(invitation_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4))))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, server_id, inviter_id, invitee_id, status \
                     FROM server_invitations WHERE id = ?"
                )
                .bind(invitation_id)
                .fetch_optional(pool)
                .await?;
                Ok(row.map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4))))
            }
        }
    }

    pub async fn update_server_invitation_status(
        &self,
        invitation_id: Uuid,
        status: &str,
    ) -> Result<bool, sqlx::Error> {
        let rows = match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE server_invitations SET status = $1 WHERE id = $2 AND status = 'pending'"
                )
                .bind(status)
                .bind(invitation_id)
                .execute(pool)
                .await?
                .rows_affected()
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "UPDATE server_invitations SET status = ? WHERE id = ? AND status = 'pending'"
                )
                .bind(status)
                .bind(invitation_id)
                .execute(pool)
                .await?
                .rows_affected()
            }
        };
        Ok(rows > 0)
    }

    pub async fn list_pending_server_invitations_for_user(
        &self,
        invitee_id: Uuid,
    ) -> Result<Vec<(Uuid, Uuid, String, Uuid, String)>, sqlx::Error> {
        // Returns: (invitation_id, server_id, server_name, inviter_id, inviter_username)
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT si.id, si.server_id, s.name, si.inviter_id, u.username \
                     FROM server_invitations si \
                     JOIN servers s ON s.id = si.server_id \
                     JOIN users u ON u.id = si.inviter_id \
                     WHERE si.invitee_id = $1 AND si.status = 'pending' \
                     ORDER BY si.created_at DESC"
                )
                .bind(invitee_id)
                .fetch_all(pool)
                .await?;
                Ok(rows.into_iter().map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4))).collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT si.id, si.server_id, s.name, si.inviter_id, u.username \
                     FROM server_invitations si \
                     JOIN servers s ON s.id = si.server_id \
                     JOIN users u ON u.id = si.inviter_id \
                     WHERE si.invitee_id = ? AND si.status = 'pending' \
                     ORDER BY si.created_at DESC"
                )
                .bind(invitee_id)
                .fetch_all(pool)
                .await?;
                Ok(rows.into_iter().map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4))).collect())
            }
        }
    }

}
