//! Connexió a la base de dades amb suport per PostgreSQL i SQLite.

use sqlx::{Pool, Sqlite, Postgres, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::config::Config;
use shared::types::ServerRole;
use tracing::{info, error};

#[derive(Debug, Clone)]
pub struct ServerLiveKitOverride {
    pub host: String,
    pub api_key: String,
    pub api_secret: String,
}

/// Connexió a la base de dades amb comprovació de connectivitat.
pub async fn connect_db(config: &Config) -> Result<DatabasePool, String> {
    let database_url = if config.is_sqlite() {
        config.sqlite_database_url()
    } else {
        config.database_url.clone()
    };

    info!("🔌 Connexió a la base de dades: {}", database_url);

    let db = if database_url.starts_with("postgres") || database_url.starts_with("postgresql") {
        info!("📦 Utilitzant PostgreSQL");
        connect_postgres(config).await?
    } else if database_url.starts_with("sqlite") {
        info!("📦 Utilitzant SQLite");
        connect_sqlite(&database_url).await?
    } else {
        let msg = format!("URL de base de dades no suportada: {}", database_url);
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
            sqlx::migrate!("./migrations")
                .run(&pool)
                .await
                .map_err(|e| format!("Error aplicant migrations PostgreSQL: {}", e))?;
            info!("✅ Migrations PostgreSQL aplicades correctament");
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
async fn connect_sqlite(database_url: &str) -> Result<DatabasePool, String> {
    // Extraure path del fitxer SQLite
    let db_path = database_url
        .strip_prefix("sqlite://")
        .unwrap_or("chillgroup.db")
        .split('?')
        .next()
        .unwrap_or("chillgroup.db");

    info!("💾 SQLite utilitzarà el fitxer: {}", db_path);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(10)
        .connect_lazy(database_url)
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
            max_storage_bytes INTEGER NOT NULL DEFAULT -1,
            max_transfer_bytes_monthly INTEGER NOT NULL DEFAULT -1,
            max_streaming_hours_monthly INTEGER NOT NULL DEFAULT -1,
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
            livekit_host TEXT,
            livekit_api_key TEXT,
            livekit_api_secret TEXT,
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
            permission_level INTEGER NOT NULL DEFAULT 2,
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

        // Attachments (encrypted file metadata)
        r#"
        CREATE TABLE IF NOT EXISTS attachments (
            id TEXT PRIMARY KEY,
            channel_id TEXT NOT NULL,
            uploader_user_id TEXT NOT NULL,
            uploader_device_id TEXT NOT NULL,
            file_name TEXT NOT NULL,
            mime_type TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            object_key TEXT NOT NULL,
            status TEXT NOT NULL,
            upload_id TEXT NOT NULL,
            chunk_size_bytes INTEGER NOT NULL,
            chunk_count INTEGER NOT NULL,
            algorithm TEXT,
            file_iv TEXT,
            wrapped_file_key TEXT,
            key_version_id TEXT,
            key_version INTEGER,
            ciphertext_sha256 TEXT,
            completed_at DATETIME,
            thumbnail_attachment_id TEXT,
            FOREIGN KEY (channel_id) REFERENCES channels(id),
            FOREIGN KEY (uploader_user_id) REFERENCES users(id),
            FOREIGN KEY (thumbnail_attachment_id) REFERENCES attachments(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_attachments_channel_id ON attachments(channel_id)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_attachments_status ON attachments(status)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_attachments_key_version_id ON attachments(key_version_id)"#,

        // Link table between messages and attachments
        r#"
        CREATE TABLE IF NOT EXISTS message_attachments (
            message_id TEXT NOT NULL,
            attachment_id TEXT NOT NULL,
            PRIMARY KEY (message_id, attachment_id),
            FOREIGN KEY (message_id) REFERENCES messages(id),
            FOREIGN KEY (attachment_id) REFERENCES attachments(id)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_message_attachments_attachment_id ON message_attachments(attachment_id)"#,

        // Invitations
        r#"
        CREATE TABLE IF NOT EXISTS invitations (
            id TEXT PRIMARY KEY,
            code TEXT UNIQUE NOT NULL,
            created_by_user_id TEXT NOT NULL,
            server_id TEXT,
            max_uses INTEGER NOT NULL DEFAULT 1,
            uses_count INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (created_by_user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY (server_id) REFERENCES servers(id) ON DELETE SET NULL
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_invitations_code ON invitations(code)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_invitations_active ON invitations(is_active)"#,

        // One-shot admin bootstrap invitation (single slot)
        r#"
        CREATE TABLE IF NOT EXISTS admin_bootstrap_invitation (
            slot INTEGER PRIMARY KEY,
            code_hash TEXT NOT NULL,
            consumed_by_user_id TEXT,
            consumed_at DATETIME,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_admin_bootstrap_invitation_code_hash ON admin_bootstrap_invitation(code_hash)"#,

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

        // Streaming usage per user per month
        r#"
        CREATE TABLE IF NOT EXISTS user_streaming_usage_monthly (
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            year_month TEXT NOT NULL,
            streaming_seconds INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (user_id, year_month)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_streaming_usage_user ON user_streaming_usage_monthly(user_id)"#,

        // Storage usage per user per month
        r#"
        CREATE TABLE IF NOT EXISTS user_storage_usage_monthly (
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            year_month TEXT NOT NULL,
            stored_bytes INTEGER NOT NULL DEFAULT 0,
            transfer_bytes INTEGER NOT NULL DEFAULT 0,
            warning_sent_at_80 DATETIME,
            warning_sent_at_90 DATETIME,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (user_id, year_month)
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_storage_usage_user ON user_storage_usage_monthly(user_id)"#,

        // Server invitations (flux d'acceptació)
        r#"
        CREATE TABLE IF NOT EXISTS server_invitations (
            id TEXT PRIMARY KEY,
            server_id TEXT NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
            inviter_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            invitee_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            status TEXT NOT NULL DEFAULT 'pending'
                CHECK (status IN ('pending', 'accepted', 'declined')),
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            expires_at DATETIME
        )
        "#,
        r#"CREATE INDEX IF NOT EXISTS idx_server_invitations_invitee ON server_invitations(invitee_id, status)"#,
        r#"CREATE INDEX IF NOT EXISTS idx_server_invitations_server ON server_invitations(server_id, status)"#,
    ];

    for query in queries {
        sqlx::query(query)
            .execute(pool)
            .await
            .map_err(|e| format!("Error creant taula: {}", e))?;
    }

    migrate_sqlite_devices_add_kem_dsa_keys(pool).await?;
    migrate_sqlite_users_add_role_and_plan(pool).await?;
    migrate_sqlite_channels_for_dm(pool).await?;
    migrate_sqlite_channel_members_permissions(pool).await?;
    migrate_sqlite_messages_add_key_version(pool).await?;
    migrate_sqlite_messages_add_sender_username(pool).await?;
    migrate_sqlite_invitations_add_server_id(pool).await?;
    migrate_sqlite_servers_add_livekit_override(pool).await?;
    migrate_sqlite_plans_add_s3_quotas(pool).await?;
    migrate_sqlite_plans_add_streaming_quota(pool).await?;
    migrate_sqlite_messages_add_reply_to(pool).await?;
    migrate_sqlite_create_message_reactions(pool).await?;
    migrate_sqlite_create_plan_change_requests(pool).await?;

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

async fn migrate_sqlite_servers_add_livekit_override(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let table_info = sqlx::query("PRAGMA table_info(servers)")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error llegint schema de servers a SQLite: {}", e))?;

    if table_info.is_empty() {
        return Ok(());
    }

    let mut has_livekit_host = false;
    let mut has_livekit_api_key = false;
    let mut has_livekit_api_secret = false;

    for row in table_info {
        let name: String = row.get(1);
        match name.as_str() {
            "livekit_host" => has_livekit_host = true,
            "livekit_api_key" => has_livekit_api_key = true,
            "livekit_api_secret" => has_livekit_api_secret = true,
            _ => {}
        }
    }

    if !has_livekit_host {
        sqlx::query("ALTER TABLE servers ADD COLUMN livekit_host TEXT")
            .execute(pool)
            .await
            .map_err(|e| format!("Error afegint livekit_host a servers SQLite: {}", e))?;
    }

    if !has_livekit_api_key {
        sqlx::query("ALTER TABLE servers ADD COLUMN livekit_api_key TEXT")
            .execute(pool)
            .await
            .map_err(|e| format!("Error afegint livekit_api_key a servers SQLite: {}", e))?;
    }

    if !has_livekit_api_secret {
        sqlx::query("ALTER TABLE servers ADD COLUMN livekit_api_secret TEXT")
            .execute(pool)
            .await
            .map_err(|e| format!("Error afegint livekit_api_secret a servers SQLite: {}", e))?;
    }

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

async fn migrate_sqlite_channel_members_permissions(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let table_info = sqlx::query("PRAGMA table_info(channel_members)")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error llegint schema de channel_members a SQLite: {}", e))?;

    if table_info.is_empty() {
        return Ok(());
    }

    let has_permission_level = table_info.into_iter().any(|row| {
        let name: String = row.get(1);
        name == "permission_level"
    });

    if has_permission_level {
        return Ok(());
    }

    sqlx::query("ALTER TABLE channel_members ADD COLUMN permission_level INTEGER NOT NULL DEFAULT 2")
        .execute(pool)
        .await
        .map_err(|e| format!("Error afegint permission_level a channel_members SQLite: {}", e))?;

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

async fn migrate_sqlite_invitations_add_server_id(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let table_info = sqlx::query("PRAGMA table_info(invitations)")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error llegint schema de invitations a SQLite: {}", e))?;

    if table_info.is_empty() {
        return Ok(());
    }

    let has_server_id = table_info.into_iter().any(|row| {
        let name: String = row.get(1);
        name == "server_id"
    });

    if !has_server_id {
        sqlx::query("ALTER TABLE invitations ADD COLUMN server_id TEXT")
            .execute(pool)
            .await
            .map_err(|e| format!("Error afegint server_id a invitations SQLite: {}", e))?;
    }

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_invitations_server_id ON invitations(server_id)")
        .execute(pool)
        .await
        .map_err(|e| format!("Error creant idx_invitations_server_id a SQLite: {}", e))?;

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

pub const CHANNEL_PERMISSION_READ: i32 = 1;
pub const CHANNEL_PERMISSION_WRITE: i32 = 2;
pub const CHANNEL_PERMISSION_MANAGE: i32 = 3;

pub const SERVER_PERMISSION_VIEW: i32 = 1;
pub const SERVER_PERMISSION_MANAGE_PROFILE: i32 = 2;
pub const SERVER_PERMISSION_MANAGE_MEMBERS: i32 = 3;


pub mod users;
pub mod servers;
pub mod channels;
pub mod attachments;
pub mod messages;
pub mod devices;
pub mod quotas;

async fn migrate_sqlite_plans_add_streaming_quota(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let table_info = sqlx::query("PRAGMA table_info(plans)")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error llegint schema de plans SQLite: {}", e))?;

    if table_info.is_empty() {
        return Ok(());
    }

    let has_col = table_info
        .iter()
        .any(|r| r.get::<String, _>(1) == "max_streaming_hours_monthly");

    if !has_col {
        sqlx::query(
            "ALTER TABLE plans ADD COLUMN max_streaming_hours_monthly INTEGER NOT NULL DEFAULT -1",
        )
        .execute(pool)
        .await
        .map_err(|e| format!("Error afegint max_streaming_hours_monthly: {}", e))?;
    }

    Ok(())
}

async fn migrate_sqlite_plans_add_s3_quotas(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let table_info = sqlx::query("PRAGMA table_info(plans)")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error llegint schema de plans a SQLite: {}", e))?;

    if table_info.is_empty() {
        return Ok(());
    }

    let mut has_max_storage = false;
    let mut has_max_transfer = false;

    for row in &table_info {
        let name: String = row.get(1);
        match name.as_str() {
            "max_storage_bytes" => has_max_storage = true,
            "max_transfer_bytes_monthly" => has_max_transfer = true,
            _ => {}
        }
    }

    if !has_max_storage {
        sqlx::query("ALTER TABLE plans ADD COLUMN max_storage_bytes INTEGER NOT NULL DEFAULT -1")
            .execute(pool)
            .await
            .map_err(|e| format!("Error afegint max_storage_bytes a plans SQLite: {}", e))?;
    }

    if !has_max_transfer {
        sqlx::query(
            "ALTER TABLE plans ADD COLUMN max_transfer_bytes_monthly INTEGER NOT NULL DEFAULT -1",
        )
        .execute(pool)
        .await
        .map_err(|e| {
            format!("Error afegint max_transfer_bytes_monthly a plans SQLite: {}", e)
        })?;
    }

    Ok(())
}

async fn migrate_sqlite_messages_add_reply_to(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let table_info = sqlx::query("PRAGMA table_info(messages)")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error llegint schema de messages SQLite: {}", e))?;

    if table_info.is_empty() {
        return Ok(());
    }

    let has_col = table_info
        .iter()
        .any(|r| r.get::<String, _>(1) == "reply_to_message_id");

    if !has_col {
        sqlx::query("ALTER TABLE messages ADD COLUMN reply_to_message_id TEXT REFERENCES messages(id) ON DELETE SET NULL")
            .execute(pool)
            .await
            .map_err(|e| format!("Error afegint reply_to_message_id a messages SQLite: {}", e))?;
    }

    Ok(())
}

async fn migrate_sqlite_create_message_reactions(pool: &sqlx::SqlitePool) -> Result<(), String> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS message_reactions (
            id TEXT PRIMARY KEY,
            message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            username TEXT NOT NULL,
            emoji TEXT NOT NULL,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CONSTRAINT uq_message_user_emoji UNIQUE (message_id, user_id, emoji)
        )"#,
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Error creant message_reactions SQLite: {}", e))?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_message_reactions_message ON message_reactions(message_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Error creant índex message_reactions SQLite: {}", e))?;

    Ok(())
}

async fn migrate_sqlite_create_plan_change_requests(pool: &sqlx::SqlitePool) -> Result<(), String> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS plan_change_requests (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            requested_plan_id TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
            message TEXT,
            status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected')),
            admin_note TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        )"#,
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Error creant plan_change_requests SQLite: {}", e))?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_plan_change_requests_user ON plan_change_requests(user_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Error creant índex plan_change_requests user SQLite: {}", e))?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_plan_change_requests_status ON plan_change_requests(status)",
    )
    .execute(pool)
    .await
    .map_err(|e| format!("Error creant índex plan_change_requests status SQLite: {}", e))?;

    Ok(())
}

async fn migrate_sqlite_devices_add_kem_dsa_keys(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let table_info = sqlx::query("PRAGMA table_info(devices)")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error llegint schema de devices SQLite: {}", e))?;

    if table_info.is_empty() {
        return Ok(());
    }

    let mut has_kem = false;
    let mut has_dsa = false;
    for row in &table_info {
        match row.get::<String, _>(1).as_str() {
            "kem_public_key" => has_kem = true,
            "dsa_public_key" => has_dsa = true,
            _ => {}
        }
    }

    if !has_kem {
        sqlx::query("ALTER TABLE devices ADD COLUMN kem_public_key TEXT NOT NULL DEFAULT ''")
            .execute(pool)
            .await
            .map_err(|e| format!("Error afegint kem_public_key a devices SQLite: {}", e))?;
    }

    if !has_dsa {
        sqlx::query("ALTER TABLE devices ADD COLUMN dsa_public_key TEXT NOT NULL DEFAULT ''")
            .execute(pool)
            .await
            .map_err(|e| format!("Error afegint dsa_public_key a devices SQLite: {}", e))?;
    }

    Ok(())
}

async fn migrate_sqlite_messages_add_sender_username(pool: &sqlx::SqlitePool) -> Result<(), String> {
    let table_info = sqlx::query("PRAGMA table_info(messages)")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error llegint schema de messages SQLite: {}", e))?;

    if table_info.is_empty() {
        return Ok(());
    }

    let has_col = table_info
        .iter()
        .any(|r| r.get::<String, _>(1) == "sender_username");

    if !has_col {
        sqlx::query("ALTER TABLE messages ADD COLUMN sender_username TEXT NOT NULL DEFAULT ''")
            .execute(pool)
            .await
            .map_err(|e| format!("Error afegint sender_username a messages SQLite: {}", e))?;

        sqlx::query(
            "UPDATE messages SET sender_username = (SELECT username FROM users WHERE users.id = messages.sender_user_id) WHERE sender_username = ''",
        )
        .execute(pool)
        .await
        .map_err(|e| format!("Error omplint sender_username a messages SQLite: {}", e))?;
    }

    // Add position column to channels if not exists
    let has_position_col = sqlx::query("PRAGMA table_info(channels)")
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Error comprovant columnes channels: {}", e))?
        .iter()
        .any(|r| r.get::<String, _>(1) == "position");

    if !has_position_col {
        sqlx::query("ALTER TABLE channels ADD COLUMN position INTEGER DEFAULT 0")
            .execute(pool)
            .await
            .map_err(|e| format!("Error afegint position a channels SQLite: {}", e))?;

        sqlx::query(
            "UPDATE channels SET position = (SELECT COUNT(*) FROM channels c2 WHERE c2.server_id = channels.server_id AND c2.created_at <= channels.created_at) - 1",
        )
        .execute(pool)
        .await
        .map_err(|e| format!("Error omplint position a channels SQLite: {}", e))?;
    }

    Ok(())
}

fn normalize_pg_dt(s: &str) -> String {
    let mut norm = s.replace(' ', "T");
    if let Some(t_pos) = norm.find('T') {
        if let Some(tz_rel) = norm[t_pos..].rfind(['+', '-']) {
            let tz_abs = t_pos + tz_rel;
            if norm[tz_abs..].len() == 3 {
                norm.push_str(":00");
            }
        }
    }
    norm
}

fn parse_datetime_required(s: &str) -> DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .or_else(|_| chrono::DateTime::parse_from_rfc3339(&normalize_pg_dt(s)))
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn parse_datetime_utc(val: &Option<String>) -> Option<DateTime<Utc>> {
    val.as_ref().map(|s| parse_datetime_required(s))
}


fn parse_server_role(role: &str) -> ServerRole {
    match role {
        "owner" => ServerRole::Owner,
        "admin" => ServerRole::Admin,
        _ => ServerRole::Member,
    }
}
