//! Connexió a la base de dades amb suport per PostgreSQL i SQLite.

use sqlx::{Pool, Sqlite, Postgres, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::config::Config;
use crate::models::{Channel, ChannelType, EncryptionType, Message};
use shared::types::{ServerInfo, ServerFullInfo, ServerMember as SharedServerMember, ServerRole};
use tracing::{info, error};

/// Connexió a la base de dades amb comprovació de connectivitat.
pub async fn connect_db(config: &Config) -> Result<DatabasePool, String> {
    info!("🔌 Connexió a la base de dades: {}", config.database_url);

    if config.database_url.starts_with("postgres") || config.database_url.starts_with("postgresql") {
        info!("📦 Utilitzant PostgreSQL");
        connect_postgres(config).await
    } else if config.database_url.starts_with("sqlite") {
        info!("📦 Utilitzant SQLite");
        connect_sqlite(config).await
    } else {
        let msg = format!("URL de base de dades no suportada: {}", config.database_url);
        error!("❌ {}", msg);
        Err(msg)
    }
}

/// Connexió a PostgreSQL amb comprovació.
async fn connect_postgres(config: &Config) -> Result<DatabasePool, String> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .connect_lazy(&config.database_url)
        .map_err(|e| format!("Error connectant PostgreSQL: {}", e))?;

    // Comprovar connectivitat
    match sqlx::query("SELECT 1")
        .fetch_one(&pool)
        .await
    {
        Ok(_) => {
            info!("✅ PostgreSQL connectat correctament");
            Ok(DatabasePool::Postgres(pool))
        }
        Err(e) => {
            error!("❌ Error comprovant connexió PostgreSQL: {}", e);
            Err(format!("Error comprovant connexió: {}", e))
        }
    }
}

/// Connexió a SQLite amb comprovació i creació automàtica de taules.
async fn connect_sqlite(config: &Config) -> Result<DatabasePool, String> {
    // Extraure path del fitxer SQLite
    let db_path = config.database_url.strip_prefix("sqlite://").unwrap_or("chillgroup.db");

    info!("💾 SQLite utilitzarà el fitxer: {}", db_path);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(10)
        .connect_lazy(&config.database_url)
        .map_err(|e| format!("Error connectant SQLite: {}", e))?;

    // Comprovar connectivitat
    match sqlx::query("SELECT 1")
        .fetch_one(&pool)
        .await
    {
        Ok(_) => info!("✅ SQLite connectat correctament"),
        Err(e) => {
            error!("❌ Error comprovant connexió SQLite: {}", e);
            return Err(format!("Error comprovant connexió: {}", e));
        }
    }

    // Crear taules si no existeixen
    create_tables_sqlite(&pool).await?;

    info!("✅ SQLite taules creades/verificades correctament");
    Ok(DatabasePool::Sqlite(pool))
}

/// Crear totes les taules necessàries per a SQLite.
async fn create_tables_sqlite(pool: &sqlx::SqlitePool) -> Result<(), String> {
    info!("📋 Creant taules si no existeixen...");

    let queries = [
        // Users
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)"#,

        // Devices
        r#"
        CREATE TABLE IF NOT EXISTS devices (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            label TEXT,
            public_key TEXT NOT NULL,
            last_seen DATETIME,
            revoked INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_devices_user_id ON devices(user_id)"#,

        // Servers
        r#"
        CREATE TABLE IF NOT EXISTS servers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            icon_url TEXT,
            owner_id TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (owner_id) REFERENCES users(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_servers_owner_id ON servers(owner_id)"#,

        // Server Members
        r#"
        CREATE TABLE IF NOT EXISTS server_members (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'member',
            joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES servers(id),
            FOREIGN KEY (user_id) REFERENCES users(id),
            UNIQUE(server_id, user_id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_server_members_server_id ON server_members(server_id)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_server_members_user_id ON server_members(user_id)"#,

        // Channels
        r#"
        CREATE TABLE IF NOT EXISTS channels (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL,
            name TEXT NOT NULL,
            type TEXT NOT NULL CHECK(type IN ('text', 'voice')),
            encryption_type TEXT NOT NULL DEFAULT 'none' CHECK(encryption_type IN ('none', 'symmetric', 'asymmetric')),
            message_ttl INTEGER,
            is_private INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES servers(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_channels_server_id ON channels(server_id)"#,

        // Channel Keys
        r#"
        CREATE TABLE IF NOT EXISTS channel_keys (
            id TEXT PRIMARY KEY,
            channel_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            encrypted_key TEXT NOT NULL,
            kem_ciphertext TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (channel_id) REFERENCES channels(id),
            FOREIGN KEY (device_id) REFERENCES devices(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_channel_keys_channel_id ON channel_keys(channel_id)"#,

        // Messages
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            channel_id TEXT NOT NULL,
            sender_user_id TEXT NOT NULL,
            sender_username TEXT NOT NULL,
            sender_device_id TEXT NOT NULL,
            encrypted_payload TEXT NOT NULL,
            iv TEXT NOT NULL,
            timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            expires_at DATETIME,
            edited_at DATETIME,
            deleted_at DATETIME,
            FOREIGN KEY (channel_id) REFERENCES channels(id),
            FOREIGN KEY (sender_user_id) REFERENCES users(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_messages_channel_id ON messages(channel_id)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp)"#,

        // Channel read state (unread counters server-authoritative)
        r#"
        CREATE TABLE IF NOT EXISTS channel_read_state (
            user_id TEXT NOT NULL,
            channel_id TEXT NOT NULL,
            last_read_message_id TEXT,
            last_read_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (user_id, channel_id),
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (channel_id) REFERENCES channels(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_channel_read_state_user ON channel_read_state(user_id)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_channel_read_state_channel ON channel_read_state(channel_id)"#,
    ];

    for query in queries {
        sqlx::query(query)
            .execute(pool)
            .await
            .map_err(|e| format!("Error creant taula: {}", e))?;
    }

    info!("✅ Taules creades/verificades correctament");
    Ok(())
}

/// Pool de base de dades unificat (PostgreSQL o SQLite).
#[derive(Clone)]
pub enum DatabasePool {
    Postgres(Pool<Postgres>),
    Sqlite(Pool<Sqlite>),
}

impl DatabasePool {
    /// Executar una query sense resultat.
    #[allow(dead_code)]
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

    /// Crear un nou usuari. Retorna el user_id generat.
    pub async fn create_user(&self, username: &str, password_hash: &str) -> Result<Uuid, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING id")
                    .bind(username)
                    .bind(password_hash)
                    .fetch_one(pool)
                    .await
                    .map_err(|e| format!("Error PostgreSQL: {}", e))?;
                Ok(row.get(0))
            }
            DatabasePool::Sqlite(pool) => {
                let user_id = Uuid::new_v4();
                sqlx::query("INSERT INTO users (id, username, password_hash) VALUES (?, ?, ?)")
                    .bind(user_id)
                    .bind(username)
                    .bind(password_hash)
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error SQLite: {}", e))?;
                Ok(user_id)
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

    /// Comprovar connexió.
    #[allow(dead_code)]
    pub async fn check_connection(&self) -> Result<(), String> {
        self.execute_query("SELECT 1").await
    }

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
                let rows = sqlx::query(query).bind(user_id).fetch_all(pool).await?;
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
                sqlx::query("INSERT INTO server_members (id, server_id, user_id, role) VALUES ($1, $2, $3, $4)")
                    .bind(Uuid::new_v4())
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

    pub async fn get_server_full_info(&self, server_id: Uuid, user_id: Uuid) -> Result<Option<ServerFullInfo>, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let server_row = sqlx::query("SELECT id, name, icon_url, owner_id, created_at FROM servers WHERE id = $1")
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

                let my_role = my_role_row
                    .map(|row| parse_server_role(&row.get::<String, _>(0)))
                    .unwrap_or(ServerRole::Member);

                let members = {
                    let rows = sqlx::query("SELECT u.id, u.username, sm.role, sm.joined_at FROM server_members sm JOIN users u ON u.id = sm.user_id WHERE sm.server_id = $1 ORDER BY sm.joined_at ASC")
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
                    created_at: server_row.get::<String, _>(4),
                }))
            }
            DatabasePool::Sqlite(pool) => {
                let server_row = sqlx::query("SELECT id, name, icon_url, owner_id, created_at FROM servers WHERE id = ?")
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

                let my_role = my_role_row
                    .map(|row| parse_server_role(&row.get::<String, _>(0)))
                    .unwrap_or(ServerRole::Member);

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
                    created_at: server_row.get::<String, _>(4),
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

    pub async fn add_server_member(&self, server_id: Uuid, user_id: Uuid, role: &str) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO server_members (id, server_id, user_id, role) VALUES ($1, $2, $3, $4)")
                    .bind(Uuid::new_v4())
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

    pub async fn list_channels_for_server(&self, server_id: Uuid, user_id: Uuid) -> Result<Vec<Channel>, sqlx::Error> {
        let query = "SELECT id, server_id, name, type AS channel_type, encryption_type, message_ttl, is_private, created_at FROM channels WHERE server_id = $1 ORDER BY type ASC, name ASC";
        let mut channels = Vec::new();
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(query)
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
                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());
                    channels.push(Channel {
                        id: row.get(0),
                        server_id: row.get(1),
                        name: row.get(2),
                        channel_type,
                        encryption_type,
                        message_ttl: row.get(5),
                        is_private: is_private != 0,
                        unread_count: 0,
                        created_at,
                    });
                }
            }
            DatabasePool::Sqlite(pool) => {
                let query = query.replace("$1", "?");
                let rows = sqlx::query(&query)
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
                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());
                    channels.push(Channel {
                        id: row.get(0),
                        server_id: row.get(1),
                        name: row.get(2),
                        channel_type,
                        encryption_type,
                        message_ttl: row.get(5),
                        is_private: is_private != 0,
                        unread_count: 0,
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

    pub async fn create_channel(&self, channel_id: Uuid, server_id: Uuid, name: &str, channel_type: &str, encryption_type: &str, message_ttl: Option<i32>, is_private: bool) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query("INSERT INTO channels (id, server_id, name, type, encryption_type, message_ttl, is_private, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)")
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
                let row = sqlx::query(query)
                    .bind(channel_id)
                    .fetch_optional(pool)
                    .await?;
                row.map(|row| {
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
                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());
                    Ok(Channel {
                        id: row.get(0),
                        server_id: row.get(1),
                        name: row.get(2),
                        channel_type,
                        encryption_type,
                        message_ttl: row.get(5),
                        is_private: is_private != 0,
                        unread_count: 0,
                        created_at,
                    })
                }).transpose()
            }
            DatabasePool::Sqlite(pool) => {
                let query = query.replace("$1", "?");
                let row = sqlx::query(&query)
                    .bind(channel_id)
                    .fetch_optional(pool)
                    .await?;
                row.map(|row| {
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
                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());
                    Ok(Channel {
                        id: row.get(0),
                        server_id: row.get(1),
                        name: row.get(2),
                        channel_type,
                        encryption_type,
                        message_ttl: row.get(5),
                        is_private: is_private != 0,
                        unread_count: 0,
                        created_at,
                    })
                }).transpose()
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
                match (name, message_ttl) {
                    (Some(n), Some(mt)) => {
                        sqlx::query(
                            "UPDATE channels SET name=$1, type=$2, encryption_type=$3, message_ttl=$4, is_private=$5 WHERE id=$6 AND server_id=$7",
                        )
                        .bind(n)
                        .bind(channel_type)
                        .bind(encryption_type)
                        .bind(mt)
                        .bind(is_private as i32)
                        .bind(channel_id)
                        .bind(server_id)
                        .execute(pool).await?;
                    }
                    (Some(n), None) => {
                        sqlx::query(
                            "UPDATE channels SET name=$1, type=$2, encryption_type=$3, is_private=$4 WHERE id=$5 AND server_id=$6",
                        )
                        .bind(n)
                        .bind(channel_type)
                        .bind(encryption_type)
                        .bind(is_private as i32)
                        .bind(channel_id)
                        .bind(server_id)
                        .execute(pool).await?;
                    }
                    (None, Some(mt)) => {
                        sqlx::query(
                            "UPDATE channels SET type=$1, encryption_type=$2, message_ttl=$3, is_private=$4 WHERE id=$5 AND server_id=$6",
                        )
                        .bind(channel_type)
                        .bind(encryption_type)
                        .bind(mt)
                        .bind(is_private as i32)
                        .bind(channel_id)
                        .bind(server_id)
                        .execute(pool).await?;
                    }
                    (None, None) => {
                        sqlx::query(
                            "UPDATE channels SET type=$1, encryption_type=$2, is_private=$3 WHERE id=$4 AND server_id=$5",
                        )
                        .bind(channel_type)
                        .bind(encryption_type)
                        .bind(is_private as i32)
                        .bind(channel_id)
                        .bind(server_id)
                        .execute(pool).await?;
                    }
                }
            }
            DatabasePool::Sqlite(pool) => {
                match (name, message_ttl) {
                    (Some(n), Some(mt)) => {
                        sqlx::query(
                            "UPDATE channels SET name=?, type=?, encryption_type=?, message_ttl=?, is_private=? WHERE id=? AND server_id=?",
                        )
                        .bind(n)
                        .bind(channel_type)
                        .bind(encryption_type)
                        .bind(mt)
                        .bind(is_private as i32)
                        .bind(channel_id)
                        .bind(server_id)
                        .execute(pool).await?;
                    }
                    (Some(n), None) => {
                        sqlx::query(
                            "UPDATE channels SET name=?, type=?, encryption_type=?, is_private=? WHERE id=? AND server_id=?",
                        )
                        .bind(n)
                        .bind(channel_type)
                        .bind(encryption_type)
                        .bind(is_private as i32)
                        .bind(channel_id)
                        .bind(server_id)
                        .execute(pool).await?;
                    }
                    (None, Some(mt)) => {
                        sqlx::query(
                            "UPDATE channels SET type=?, encryption_type=?, message_ttl=?, is_private=? WHERE id=? AND server_id=?",
                        )
                        .bind(channel_type)
                        .bind(encryption_type)
                        .bind(mt)
                        .bind(is_private as i32)
                        .bind(channel_id)
                        .bind(server_id)
                        .execute(pool).await?;
                    }
                    (None, None) => {
                        sqlx::query(
                            "UPDATE channels SET type=?, encryption_type=?, is_private=? WHERE id=? AND server_id=?",
                        )
                        .bind(channel_type)
                        .bind(encryption_type)
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
                sqlx::query("DELETE FROM channels WHERE id = ?")
                    .bind(channel_id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

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
        expires_at: Option<DateTime<Utc>>,
        timestamp: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO messages \
                     (id, channel_id, sender_user_id, sender_username, sender_device_id, \
                      encrypted_payload, iv, timestamp, expires_at, edited_at, deleted_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL, NULL)",
                )
                .bind(message_id)
                .bind(channel_id)
                .bind(sender_user_id)
                .bind(sender_username)
                .bind(sender_device_id)
                .bind(payload)
                .bind(iv)
                .bind(timestamp.to_rfc3339())
                .bind(expires_at.map(|d| d.to_rfc3339()))
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO messages \
                     (id, channel_id, sender_user_id, sender_username, sender_device_id, \
                      encrypted_payload, iv, timestamp, expires_at, edited_at, deleted_at) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL)",
                )
                .bind(message_id)
                .bind(channel_id)
                .bind(sender_user_id)
                .bind(sender_username)
                .bind(sender_device_id)
                .bind(payload)
                .bind(iv)
                .bind(timestamp.to_rfc3339())
                .bind(expires_at.map(|d| d.to_rfc3339()))
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_message(&self, message_id: Uuid) -> Result<Option<Message>, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let query = "SELECT id, channel_id, sender_user_id, sender_username, sender_device_id, \
                     encrypted_payload, iv, timestamp, expires_at, edited_at, deleted_at \
                     FROM messages WHERE id = $1 AND (expires_at IS NULL OR expires_at > $2)";
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(query)
                    .bind(message_id)
                    .bind(&now)
                    .fetch_optional(pool)
                    .await?;
                row.map(|row| {
                    Ok(Message {
                        id: row.get(0),
                        channel_id: row.get(1),
                        sender_user_id: row.get(2),
                        sender_username: row.get(3),
                        sender_device_id: row.get(4),
                        encrypted_payload: row.get(5),
                        iv: row.get(6),
                        timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>(7))
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                        expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(8)),
                        edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                        deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                    })
                })
                .transpose()
            }
            DatabasePool::Sqlite(pool) => {
                let query = query.replace("$1", "?");
                let row = sqlx::query(&query)
                    .bind(message_id)
                    .bind(&now)
                    .fetch_optional(pool)
                    .await?;
                row.map(|row| {
                    Ok(Message {
                        id: row.get(0),
                        channel_id: row.get(1),
                        sender_user_id: row.get(2),
                        sender_username: row.get(3),
                        sender_device_id: row.get(4),
                        encrypted_payload: row.get(5),
                        iv: row.get(6),
                        timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>(7))
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                        expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(8)),
                        edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                        deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                    })
                })
                .transpose()
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
                conditions.push(format!("(expires_at IS NULL OR expires_at > $2)"));

                if let Some(a) = after {
                    conditions.push("id > $3".to_string());
                    let _ = a; // bound below
                }
                if let Some(b) = before {
                    let before_param = if after.is_some() { 4 } else { 3 };
                    conditions.push(format!("id < ${}", before_param));
                    let _ = b;
                }
                if let Some(s) = since {
                    let since_param = 3 + usize::from(after.is_some()) + usize::from(before.is_some());
                    conditions.push(format!("timestamp > ${}", since_param));
                    let _ = s;
                }

                conditions.push("deleted_at IS NULL".to_string());

                let order = if after.is_some() {
                    "ORDER BY timestamp DESC, id DESC"
                } else {
                    "ORDER BY timestamp ASC, id ASC"
                };

                let query = format!(
                    "SELECT id, channel_id, sender_user_id, sender_username, sender_device_id, \
                     encrypted_payload, iv, timestamp, expires_at, edited_at, deleted_at \
                     FROM messages WHERE {} {} LIMIT $999",
                    conditions.join(" AND "),
                    order
                );

                let query = query.replace("$1", "?").replace("$2", "?");
                let mut q = sqlx::query(&query);
                q = q.bind(channel_id);
                q = q.bind(&now);
                if let Some(a) = after { q = q.bind(a); }
                if let Some(b) = before { q = q.bind(b); }
                if let Some(s) = since { q = q.bind(s.to_rfc3339()); }
                q = q.bind((limit + 1) as i32);

                let rows = q.fetch_all(pool).await?;
                for row in rows {
                    msgs.push(Message {
                        id: row.get(0),
                        channel_id: row.get(1),
                        sender_user_id: row.get(2),
                        sender_username: row.get(3),
                        sender_device_id: row.get(4),
                        encrypted_payload: row.get(5),
                        iv: row.get(6),
                        timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>(7))
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                        expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(8)),
                        edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                        deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                    });
                }
            }
            DatabasePool::Sqlite(pool) => {
                let mut conditions = vec!["channel_id = ?".to_string(), "(expires_at IS NULL OR expires_at > ?)".to_string()];

                if let Some(a) = after {
                    conditions.push("id > ?".to_string());
                    let _ = a;
                }
                if let Some(b) = before {
                    conditions.push("id < ?".to_string());
                    let _ = b;
                }
                if let Some(s) = since {
                    conditions.push("timestamp > ?".to_string());
                    let _ = s;
                }

                conditions.push("deleted_at IS NULL".to_string());

                let order = if after.is_some() {
                    "ORDER BY timestamp DESC, id DESC"
                } else {
                    "ORDER BY timestamp ASC, id ASC"
                };

                let query = format!(
                    "SELECT id, channel_id, sender_user_id, sender_username, sender_device_id, \
                     encrypted_payload, iv, timestamp, expires_at, edited_at, deleted_at \
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
                for row in rows {
                    msgs.push(Message {
                        id: row.get(0),
                        channel_id: row.get(1),
                        sender_user_id: row.get(2),
                        sender_username: row.get(3),
                        sender_device_id: row.get(4),
                        encrypted_payload: row.get(5),
                        iv: row.get(6),
                        timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>(7))
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                        expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(8)),
                        edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                        deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
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
                     SET encrypted_payload = $1, iv = $2, edited_at = $3 \
                     WHERE id = $4 \
                     RETURNING id, channel_id, sender_user_id, sender_username, sender_device_id, \
                               encrypted_payload, iv, timestamp, expires_at, edited_at, deleted_at";
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(query)
                    .bind(payload)
                    .bind(iv)
                    .bind(edited_at.to_rfc3339())
                    .bind(message_id)
                    .fetch_one(pool)
                    .await?;
                Ok(Message {
                    id: row.get(0),
                    channel_id: row.get(1),
                    sender_user_id: row.get(2),
                    sender_username: row.get(3),
                    sender_device_id: row.get(4),
                    encrypted_payload: row.get(5),
                    iv: row.get(6),
                    timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>(7))
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(8)),
                    edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                    deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                })
            }
            DatabasePool::Sqlite(pool) => {
                let query = query.replace("$1", "?").replace("$2", "?").replace("$3", "?").replace("$4", "?");
                let row = sqlx::query(&query)
                    .bind(payload)
                    .bind(iv)
                    .bind(edited_at.to_rfc3339())
                    .bind(message_id)
                    .fetch_one(pool)
                    .await?;
                Ok(Message {
                    id: row.get(0),
                    channel_id: row.get(1),
                    sender_user_id: row.get(2),
                    sender_username: row.get(3),
                    sender_device_id: row.get(4),
                    encrypted_payload: row.get(5),
                    iv: row.get(6),
                    timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>(7))
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(8)),
                    edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                    deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                })
            }
        }
    }

    pub async fn delete_message(&self, message_id: Uuid) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "UPDATE messages SET deleted_at = $1 WHERE id = $2",
                )
                .bind(chrono::Utc::now().to_rfc3339())
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
    pub async fn delete_expired_messages(&self) -> Result<Vec<(Uuid, Uuid)>, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut deleted = Vec::new();
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "DELETE FROM messages WHERE expires_at IS NOT NULL AND expires_at <= $1 \
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
                    sqlx::query(
                        "DELETE FROM messages WHERE expires_at IS NOT NULL AND expires_at <= ?",
                    )
                    .bind(&now)
                    .execute(pool)
                    .await?;
                }
            }
        }
        Ok(deleted)
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
                     AND (expires_at IS NULL OR expires_at > $4) \
                     AND timestamp > $3",
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
                     VALUES ($1, $2, $3, $4, $4) \
                     ON CONFLICT (user_id, channel_id) DO UPDATE SET \
                     last_read_message_id = EXCLUDED.last_read_message_id, \
                     last_read_at = EXCLUDED.last_read_at, \
                     updated_at = EXCLUDED.updated_at",
                )
                .bind(user_id)
                .bind(channel_id)
                .bind(last_read_message_id)
                .bind(&now)
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
                                             AND (m.expires_at IS NULL OR m.expires_at > $3) \
                       AND m.timestamp > COALESCE(rs.last_read_at, '1970-01-01T00:00:00Z')",
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

fn parse_datetime_utc(val: &Option<String>) -> Option<DateTime<Utc>> {
    val.as_ref().and_then(|s| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|d| d.with_timezone(&Utc))
    })
}


fn parse_server_role(role: &str) -> ServerRole {
    match role {
        "owner" => ServerRole::Owner,
        "admin" => ServerRole::Admin,
        _ => ServerRole::Member,
    }
}
