//! Connexió a la base de dades amb suport per PostgreSQL i SQLite.

use sqlx::{Pool, Sqlite, Postgres, Row};
use uuid::Uuid;
use crate::config::Config;
use crate::models::{Channel, ChannelType, EncryptionType};
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

    pub async fn get_server_full_info(&self, server_id: Uuid) -> Result<Option<ServerFullInfo>, sqlx::Error> {
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

    pub async fn list_channels_for_server(&self, server_id: Uuid) -> Result<Vec<Channel>, sqlx::Error> {
        let query = "SELECT id, server_id, name, type, encryption_type, message_ttl, is_private, created_at FROM channels WHERE server_id = $1 ORDER BY type ASC, name ASC";
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
                        created_at,
                    });
                }
            }
        }
        Ok(channels)
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
}

fn parse_server_role(role: &str) -> ServerRole {
    match role {
        "owner" => ServerRole::Owner,
        "admin" => ServerRole::Admin,
        _ => ServerRole::Member,
    }
}
