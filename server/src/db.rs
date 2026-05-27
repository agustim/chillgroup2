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

    let db = if config.database_url.starts_with("postgres") || config.database_url.starts_with("postgresql") {
        info!("📦 Utilitzant PostgreSQL");
        connect_postgres(config).await?
    } else if config.database_url.starts_with("sqlite") {
        info!("📦 Utilitzant SQLite");
        connect_sqlite(config).await?
    } else {
        let msg = format!("URL de base de dades no suportada: {}", config.database_url);
        error!("❌ {}", msg);
        return Err(msg);
    };

    db.ensure_default_plans().await?;
    Ok(db)
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
        // Plans
        r#"
        CREATE TABLE IF NOT EXISTS plans (
            id TEXT PRIMARY KEY,
            name TEXT UNIQUE NOT NULL,
            display_name TEXT NOT NULL,
            description TEXT,
            max_servers INTEGER NOT NULL,
            max_channels_text_per_server INTEGER NOT NULL,
            max_channels_voice_per_server INTEGER NOT NULL,
            max_members_per_server INTEGER NOT NULL,
            api_calls_per_minute INTEGER NOT NULL,
            messages_per_day INTEGER NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_plans_name ON plans(name)"#,

        // Users
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            plan_id TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (plan_id) REFERENCES plans(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_users_plan_id ON users(plan_id)"#,

            // Friendships
            r#"
            CREATE TABLE IF NOT EXISTS friendships (
                owner_user_id TEXT NOT NULL,
                friend_user_id TEXT NOT NULL,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (owner_user_id, friend_user_id),
                FOREIGN KEY (owner_user_id) REFERENCES users(id) ON DELETE CASCADE,
                FOREIGN KEY (friend_user_id) REFERENCES users(id) ON DELETE CASCADE
            )
            "#,
            r#"CREATE INDEX IF NOT EXISTS idx_friendships_owner_user_id ON friendships(owner_user_id)"#,
            r#"CREATE INDEX IF NOT EXISTS idx_friendships_friend_user_id ON friendships(friend_user_id)"#,

        // Devices
        r#"
        CREATE TABLE IF NOT EXISTS devices (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            label TEXT,
            public_key TEXT NOT NULL,
            kem_public_key TEXT NOT NULL DEFAULT '',
            dsa_public_key TEXT NOT NULL DEFAULT '',
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

        // Channel Members (for private channels)
        r#"
        CREATE TABLE IF NOT EXISTS channel_members (
            id TEXT PRIMARY KEY,
            channel_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            joined_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (channel_id) REFERENCES channels(id),
            FOREIGN KEY (user_id) REFERENCES users(id),
            UNIQUE(channel_id, user_id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_channel_members_channel_id ON channel_members(channel_id)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_channel_members_user_id ON channel_members(user_id)"#,

        // Channels
        r#"
        CREATE TABLE IF NOT EXISTS channels (
            id TEXT PRIMARY KEY,
            server_id TEXT,
            name TEXT NOT NULL,
            type TEXT NOT NULL CHECK(type IN ('text', 'voice')),
            encryption_type TEXT NOT NULL DEFAULT 'none' CHECK(encryption_type IN ('none', 'symmetric', 'asymmetric')),
            scope TEXT NOT NULL DEFAULT 'server' CHECK(scope IN ('server', 'dm')),
            dm_user_a_id TEXT,
            dm_user_b_id TEXT,
            message_ttl INTEGER,
            is_private INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES servers(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_channels_server_id ON channels(server_id)"#,

        // Asymmetric channel key bundles (versioned, per device)
        r#"
        CREATE TABLE IF NOT EXISTS channel_key_device_bundles (
            id TEXT PRIMARY KEY,
            key_version_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            encrypted_key TEXT NOT NULL,
            kem_ciphertext TEXT NOT NULL,
            signature TEXT,
            signed_by_device_id TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE (key_version_id, device_id),
            FOREIGN KEY (key_version_id) REFERENCES channel_key_versions(id),
            FOREIGN KEY (device_id) REFERENCES devices(id),
            FOREIGN KEY (signed_by_device_id) REFERENCES devices(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_channel_key_device_bundles_key_version_id ON channel_key_device_bundles(key_version_id)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_channel_key_device_bundles_device_id ON channel_key_device_bundles(device_id)"#,

        // Channel key versions (Nivell 1: simètric, clau xifrada amb master key)
        r#"
        CREATE TABLE IF NOT EXISTS channel_key_versions (
            id TEXT PRIMARY KEY,
            channel_id TEXT NOT NULL,
            version INTEGER NOT NULL,
            encrypted_key TEXT NOT NULL,
            nonce TEXT NOT NULL,
            created_by TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            deprecated_at DATETIME,
            UNIQUE(channel_id, version),
            FOREIGN KEY (channel_id) REFERENCES channels(id),
            FOREIGN KEY (created_by) REFERENCES users(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_channel_key_versions_channel ON channel_key_versions(channel_id)"#,

        // Asymmetric channel bundles
        r#"
        CREATE TABLE IF NOT EXISTS channel_key_device_bundles (
            id TEXT PRIMARY KEY,
            key_version_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            encrypted_key TEXT NOT NULL,
            kem_ciphertext TEXT NOT NULL,
            signature TEXT,
            signed_by_device_id TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(key_version_id, device_id),
            FOREIGN KEY (key_version_id) REFERENCES channel_key_versions(id),
            FOREIGN KEY (device_id) REFERENCES devices(id),
            FOREIGN KEY (signed_by_device_id) REFERENCES devices(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_channel_key_device_bundles_key_version_id ON channel_key_device_bundles(key_version_id)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_channel_key_device_bundles_device_id ON channel_key_device_bundles(device_id)"#,

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
            key_version INTEGER,
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

        // Invitations
        r#"
        CREATE TABLE IF NOT EXISTS invitations (
            id TEXT PRIMARY KEY,
            code TEXT UNIQUE NOT NULL,
            created_by_user_id TEXT NOT NULL,
            max_uses INTEGER NOT NULL DEFAULT 1,
            uses_count INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE CASCADE
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_invitations_code ON invitations(code)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_invitations_active ON invitations(is_active)"#,

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

    migrate_sqlite_users_add_role_and_plan(pool).await?;
    migrate_sqlite_channels_for_dm(pool).await?;
    migrate_sqlite_messages_add_key_version(pool).await?;

    for idx in [
        "CREATE INDEX IF NOT EXISTS idx_channels_scope ON channels(scope)",
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_channels_dm_pair ON channels(dm_user_a_id, dm_user_b_id) WHERE scope = 'dm'",
    ] {
        sqlx::query(idx)
            .execute(pool)
            .await
            .map_err(|e| format!("Error creant índex DM SQLite: {}", e))?;
    }

    info!("✅ Taules creades/verificades correctament");
    Ok(())
}

async fn migrate_sqlite_users_add_role_and_plan(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let table_info = sqlx::query("PRAGMA table_info(users)")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error llegint schema de users a SQLite: {}", e))?;

    if table_info.is_empty() {
        return Ok(());
    }

    let mut has_role = false;
    let mut has_plan_id = false;

    for row in table_info {
        let name: String = row.get(1);
        match name.as_str() {
            "role" => has_role = true,
            "plan_id" => has_plan_id = true,
            _ => {}
        }
    }

    if !has_role {
        sqlx::query("ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user'")
            .execute(pool)
            .await
            .map_err(|e| format!("Error afegint role a users SQLite: {}", e))?;
    }

    if !has_plan_id {
        sqlx::query("ALTER TABLE users ADD COLUMN plan_id TEXT")
            .execute(pool)
            .await
            .map_err(|e| format!("Error afegint plan_id a users SQLite: {}", e))?;
    }

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_plan_id ON users(plan_id)")
        .execute(pool)
        .await
        .map_err(|e| format!("Error creant idx_users_plan_id a SQLite: {}", e))?;

    Ok(())
}

async fn migrate_sqlite_channels_for_dm(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let table_info = sqlx::query("PRAGMA table_info(channels)")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error llegint schema de channels a SQLite: {}", e))?;

    if table_info.is_empty() {
        return Ok(());
    }

    let mut has_scope = false;
    let mut has_dm_user_a_id = false;
    let mut has_dm_user_b_id = false;
    let mut server_id_notnull = false;

    for row in table_info {
        let name: String = row.get(1);
        let notnull: i64 = row.get(3);
        match name.as_str() {
            "scope" => has_scope = true,
            "dm_user_a_id" => has_dm_user_a_id = true,
            "dm_user_b_id" => has_dm_user_b_id = true,
            "server_id" => server_id_notnull = notnull != 0,
            _ => {}
        }
    }

    if has_scope && has_dm_user_a_id && has_dm_user_b_id && !server_id_notnull {
        return Ok(());
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Error iniciant migració DM SQLite: {}", e))?;

    sqlx::query(
        "CREATE TABLE channels_dm_migrated (
            id TEXT PRIMARY KEY,
            server_id TEXT,
            name TEXT NOT NULL,
            type TEXT NOT NULL CHECK(type IN ('text', 'voice')),
            encryption_type TEXT NOT NULL DEFAULT 'none' CHECK(encryption_type IN ('none', 'symmetric', 'asymmetric')),
            scope TEXT NOT NULL DEFAULT 'server' CHECK(scope IN ('server', 'dm')),
            dm_user_a_id TEXT,
            dm_user_b_id TEXT,
            message_ttl INTEGER,
            is_private INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (server_id) REFERENCES servers(id)
        )",
    )
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Error creant channels_dm_migrated: {}", e))?;

    let scope_expr = if has_scope { "COALESCE(scope, 'server')" } else { "'server'" };
    let dm_user_a_expr = if has_dm_user_a_id { "dm_user_a_id" } else { "NULL" };
    let dm_user_b_expr = if has_dm_user_b_id { "dm_user_b_id" } else { "NULL" };

    let copy_query = format!(
        "INSERT INTO channels_dm_migrated (id, server_id, name, type, encryption_type, scope, dm_user_a_id, dm_user_b_id, message_ttl, is_private, created_at)
         SELECT id, server_id, name, type, encryption_type, {scope_expr}, {dm_user_a_expr}, {dm_user_b_expr}, message_ttl, is_private, created_at
         FROM channels"
    );

    sqlx::query(&copy_query)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error copiant dades channels cap a schema DM: {}", e))?;

    sqlx::query("DROP TABLE channels")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error eliminant taula channels antiga: {}", e))?;

    sqlx::query("ALTER TABLE channels_dm_migrated RENAME TO channels")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error reanomenant taula channels migrada: {}", e))?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_channels_server_id ON channels(server_id)")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error recreant índex idx_channels_server_id: {}", e))?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_channels_scope ON channels(scope)")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error recreant índex idx_channels_scope: {}", e))?;

    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_channels_dm_pair ON channels(dm_user_a_id, dm_user_b_id) WHERE scope = 'dm'")
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Error recreant índex idx_channels_dm_pair: {}", e))?;

    tx.commit()
        .await
        .map_err(|e| format!("Error confirmant migració DM SQLite: {}", e))?;

    Ok(())
}

async fn migrate_sqlite_messages_add_key_version(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let table_info = sqlx::query("PRAGMA table_info(messages)")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error llegint schema de messages a SQLite: {}", e))?;

    if table_info.is_empty() {
        return Ok(());
    }

    let has_key_version = table_info.into_iter().any(|row| {
        let name: String = row.get(1);
        name == "key_version"
    });

    if has_key_version {
        return Ok(());
    }

    sqlx::query("ALTER TABLE messages ADD COLUMN key_version INTEGER")
        .execute(pool)
        .await
        .map_err(|e| format!("Error afegint key_version a messages SQLite: {}", e))?;

    Ok(())
}

/// Pool de base de dades unificat (PostgreSQL o SQLite).
#[derive(Clone)]
pub enum DatabasePool {
    Postgres(Pool<Postgres>),
    Sqlite(Pool<Sqlite>),
}

#[derive(Debug, Clone)]
pub struct DirectChannelSummary {
    pub channel_id: Uuid,
    pub peer_user_id: Uuid,
    pub peer_username: String,
    pub message_ttl: Option<i32>,
    pub last_message_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKeyBundleWriteResult {
    Inserted,
    Unchanged,
    Conflict,
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

        match self {
            DatabasePool::Postgres(pool) => {
                let insert = "INSERT INTO plans (id, name, display_name, description, max_servers, max_channels_text_per_server, max_channels_voice_per_server, max_members_per_server, api_calls_per_minute, messages_per_day) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) ON CONFLICT (name) DO NOTHING";
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
                    .execute(pool)
                    .await
                    .map_err(|e| format!("Error inserint plan enterprise (Postgres): {}", e))?;
            }
            DatabasePool::Sqlite(pool) => {
                let insert = "INSERT INTO plans (id, name, display_name, description, max_servers, max_channels_text_per_server, max_channels_voice_per_server, max_members_per_server, api_calls_per_minute, messages_per_day) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) ON CONFLICT(name) DO NOTHING";
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

    pub async fn count_channels_by_type_in_server(&self, server_id: Uuid, channel_type: &str) -> Result<i64, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query("SELECT COUNT(*) FROM channels WHERE server_id = $1 AND type = $2")
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
        max_uses: i32,
    ) -> Result<Uuid, String> {
        let id = Uuid::new_v4();
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO invitations (id, code, created_by_user_id, max_uses, uses_count, is_active) VALUES ($1, $2, $3, $4, 0, true)",
                )
                .bind(id)
                .bind(code)
                .bind(created_by_user_id)
                .bind(max_uses)
                .execute(pool)
                .await
                .map_err(|e| format!("Error PostgreSQL: {}", e))?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO invitations (id, code, created_by_user_id, max_uses, uses_count, is_active) VALUES (?, ?, ?, ?, 0, 1)",
                )
                .bind(id)
                .bind(code)
                .bind(created_by_user_id)
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
    ) -> Result<Option<(Uuid, i32, i32, bool)>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT id, max_uses, uses_count, is_active FROM invitations WHERE code = $1",
                )
                .bind(code)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Error PostgreSQL: {}", e))?;

                Ok(row.map(|r| (r.get(0), r.get(1), r.get(2), r.get(3))))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT id, max_uses, uses_count, is_active FROM invitations WHERE code = ?",
                )
                .bind(code)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Error SQLite: {}", e))?;

                Ok(row.map(|r| {
                    let is_active: i64 = r.get(3);
                    (r.get(0), r.get(1), r.get(2), is_active != 0)
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

    pub async fn list_invitations_admin(&self) -> Result<Vec<(Uuid, String, i32, i32, bool, String)>, String> {
        match self {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT i.id, i.code, i.max_uses, i.uses_count, i.is_active, u.username \
                     FROM invitations i \
                     JOIN users u ON u.id = i.created_by_user_id \
                     ORDER BY i.created_at DESC",
                )
                .fetch_all(pool)
                .await
                .map_err(|e| format!("Error PostgreSQL: {}", e))?;

                Ok(rows
                    .into_iter()
                    .map(|r| (r.get(0), r.get(1), r.get(2), r.get(3), r.get(4), r.get(5)))
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT i.id, i.code, i.max_uses, i.uses_count, i.is_active, u.username \
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
                        let is_active: i64 = r.get(4);
                        (r.get(0), r.get(1), r.get(2), r.get(3), is_active != 0, r.get(5))
                    })
                    .collect())
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
                let query = "SELECT c.id, c.server_id, c.name, c.type AS channel_type, c.encryption_type, c.message_ttl, c.is_private, c.created_at \
                                         FROM channels c \
                                         WHERE c.server_id = $1 \
                                             AND (c.is_private = 0 OR EXISTS (SELECT 1 FROM channel_members cm WHERE cm.channel_id = c.id AND cm.user_id = $2)) \
                                         ORDER BY c.type ASC, c.name ASC";
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
                    let is_private: i64 = row.get(6);
                    let created_at_str: String = row.get(7);
                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());
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
                        unread_count: 0,
                        key_version_id,
                        key_version,
                        created_at,
                    });
                }
            }
            DatabasePool::Sqlite(pool) => {
                                let query = "SELECT c.id, c.server_id, c.name, c.type AS channel_type, c.encryption_type, c.message_ttl, c.is_private, c.created_at \
                                                         FROM channels c \
                                                         WHERE c.server_id = ? \
                                                             AND (c.is_private = 0 OR EXISTS (SELECT 1 FROM channel_members cm WHERE cm.channel_id = c.id AND cm.user_id = ?)) \
                                                         ORDER BY c.type ASC, c.name ASC";
                let rows = sqlx::query(&query)
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
                    let is_private: i64 = row.get(6);
                    let created_at_str: String = row.get(7);
                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());
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
                        unread_count: 0,
                        key_version_id,
                        key_version,
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

    pub async fn add_channel_member(&self, channel_id: Uuid, user_id: Uuid) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO channel_members (id, channel_id, user_id, joined_at) VALUES ($1, $2, $3, NOW()) ON CONFLICT (channel_id, user_id) DO NOTHING"
                )
                .bind(id)
                .bind(channel_id)
                .bind(user_id)
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT OR IGNORE INTO channel_members (id, channel_id, user_id, joined_at) VALUES (?, ?, ?, ?)"
                )
                .bind(id)
                .bind(channel_id)
                .bind(user_id)
                .bind(&now)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

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

    pub async fn user_can_access_channel(&self, channel_id: Uuid, user_id: Uuid) -> Result<bool, sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                let row = sqlx::query(
                    "SELECT EXISTS(\
                        SELECT 1 \
                        FROM channels c \
                        LEFT JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = $2 \
                        LEFT JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = $2 \
                        WHERE c.id = $1 AND (\
                            (COALESCE(c.scope, 'server') = 'dm' AND cm.user_id IS NOT NULL) \
                            OR \
                            (COALESCE(c.scope, 'server') != 'dm' AND sm.user_id IS NOT NULL AND (c.is_private = 0 OR cm.user_id IS NOT NULL))\
                        )\
                    )"
                )
                .bind(channel_id)
                .bind(user_id)
                .fetch_one(pool)
                .await?;
                Ok(row.get::<bool, _>(0))
            }
            DatabasePool::Sqlite(pool) => {
                let row = sqlx::query(
                    "SELECT EXISTS(\
                        SELECT 1 \
                        FROM channels c \
                        LEFT JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = ? \
                        LEFT JOIN channel_members cm ON cm.channel_id = c.id AND cm.user_id = ? \
                        WHERE c.id = ? AND (\
                            (COALESCE(c.scope, 'server') = 'dm' AND cm.user_id IS NOT NULL) \
                            OR \
                            (COALESCE(c.scope, 'server') != 'dm' AND sm.user_id IS NOT NULL AND (c.is_private = 0 OR cm.user_id IS NOT NULL))\
                        )\
                    )"
                )
                .bind(user_id)
                .bind(user_id)
                .bind(channel_id)
                .fetch_one(pool)
                .await?;
                Ok(row.get::<bool, _>(0))
            }
        }
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
                    "INSERT INTO channels (id, server_id, name, type, encryption_type, scope, dm_user_a_id, dm_user_b_id, message_ttl, is_private, created_at)
                     VALUES ($1, NULL, $2, 'text', 'asymmetric', 'dm', $3, $4, $5, true, $6)",
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

        self.add_channel_member(channel_id, creator_user_id).await?;
        self.add_channel_member(channel_id, target_user_id).await?;

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
                        MAX(m.timestamp) AS last_message_at
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
                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());
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
                        unread_count: 0,
                        key_version_id,
                        key_version,
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
                    let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now());
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
                        unread_count: 0,
                        key_version_id,
                        key_version,
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
        key_version: Option<i32>,
        expires_at: Option<DateTime<Utc>>,
        timestamp: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        match self {
            DatabasePool::Postgres(pool) => {
                sqlx::query(
                    "INSERT INTO messages \
                     (id, channel_id, sender_user_id, sender_username, sender_device_id, \
                      encrypted_payload, iv, key_version, timestamp, expires_at, edited_at, deleted_at) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL, NULL)",
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
                .execute(pool)
                .await?;
            }
            DatabasePool::Sqlite(pool) => {
                sqlx::query(
                    "INSERT INTO messages \
                     (id, channel_id, sender_user_id, sender_username, sender_device_id, \
                      encrypted_payload, iv, key_version, timestamp, expires_at, edited_at, deleted_at) \
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, NULL)",
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
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    pub async fn get_message(&self, message_id: Uuid) -> Result<Option<Message>, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();
        let query = "SELECT id, channel_id, sender_user_id, sender_username, sender_device_id, \
                     encrypted_payload, iv, key_version, timestamp, expires_at, edited_at, deleted_at \
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
                        key_version: row.get(7),
                        timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>(8))
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                        expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                        edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                        deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
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
                        key_version: row.get(7),
                        timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>(8))
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                        expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                        edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                        deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
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
                     encrypted_payload, iv, key_version, timestamp, expires_at, edited_at, deleted_at \
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
                        key_version: row.get(7),
                        timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>(8))
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                        expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                        edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                        deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
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
                     encrypted_payload, iv, key_version, timestamp, expires_at, edited_at, deleted_at \
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
                        key_version: row.get(7),
                        timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>(8))
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| chrono::Utc::now()),
                        expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                        edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                        deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
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
                               encrypted_payload, iv, key_version, timestamp, expires_at, edited_at, deleted_at";
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
                    key_version: row.get(7),
                    timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>(8))
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                    edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                    deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
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
                    key_version: row.get(7),
                    timestamp: chrono::DateTime::parse_from_rfc3339(&row.get::<String, _>(8))
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    expires_at: parse_datetime_utc(&row.get::<Option<String>, _>(9)),
                    edited_at: parse_datetime_utc(&row.get::<Option<String>, _>(10)),
                    deleted_at: parse_datetime_utc(&row.get::<Option<String>, _>(11)),
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

    /// Retorna el primer dispositiu actiu de l'usuari (device_id, kem_public_key).
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
