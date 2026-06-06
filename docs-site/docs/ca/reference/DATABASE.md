# ChillGroup v2 — Base de Dades

## Estratègia Multi-DB

ChillGroup suporta **PostgreSQL** i **SQLite** amb la mateixa API de negoci.

| Aspecte | PostgreSQL | SQLite |
|---------|-----------|--------|
| Ús | Producció | Desenvolupament / instàncies petites |
| Pool | `deadpool-postgres` | `sqlx::SqlitePool` |
| Tipus UUID | `UUID` natiu | `TEXT` amb constraint UNIQUE |
| Timestamps | `TIMESTAMPTZ` | `DATETIME` (TEXT amb ISO 8601) |
| Conexions múltiples | ✅ Sí | ❌ No (single-process) |
| Concurrència escritura | Alta | Baixa (file lock) |
| Escalabilitat | Vertical i horizontal | Vertical només |
| Configuració | `DATABASE_URL=postgresql://...` | `DATABASE_URL=sqlite://chillgroup.db` |

## Implementació SQLx

### Configuració del Pool

```rust
// server/src/db.rs
use sqlx::{PgPool, SqlitePool, Pool};

pub enum DbType {
    Postgres,
    SQLite,
}

pub struct Database {
    pub pool: Pool<sqlx::Either<sqlx::Postgres, sqlx::Sqlite>>,
    pub db_type: DbType,
}

impl Database {
    pub async fn new(url: &str, db_type: DbType) -> Result<Self> {
        let pool = match db_type {
            DbType::Postgres => {
                PgPool::connect(url).await?
                    .into()
            }
            DbType::SQLite => {
                SqlitePool::connect(url).await?
                    .into()
            }
        };

        Ok(Self { pool, db_type })
    }

    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }
}
```

### Repositori Polimòrfic

```rust
// server/src/repositories/message_repo.rs
use sqlx::{Pool, Either};
use sqlx::postgres::PgQueryResult;
use sqlx::sqlite::SqliteQueryResult;
use crate::models::Message;
use crate::error::AppError;
use std::time::Duration;

pub struct MessageRepository {
    pool: Pool<Either<sqlx::Postgres, sqlx::Sqlite>>,
}

impl MessageRepository {
    pub fn new(pool: Pool<Either<sqlx::Postgres, sqlx::Sqlite>>) -> Self {
        Self { pool }
    }

    pub async fn create(&self, msg: &Message) -> Result<Message, AppError> {
        let result = sqlx::query_as!(
            Message,
            r#"
            INSERT INTO messages (id, channel_id, sender_user_id, sender_device_id,
                                  encrypted_payload, iv, timestamp, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
            msg.id,
            msg.channel_id,
            msg.sender_user_id,
            msg.sender_device_id,
            msg.encrypted_payload,
            msg.iv,
            msg.timestamp,
            msg.expires_at
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result)
    }

    pub async fn get_by_channel(
        &self,
        channel_id: Uuid,
        limit: usize,
        before: Option<Uuid>,
    ) -> Result<Vec<Message>, AppError> {
        let result = if let Some(before_id) = before {
            sqlx::query_as!(
                Message,
                r#"
                SELECT * FROM messages
                WHERE channel_id = $1 AND id < $2
                ORDER BY timestamp DESC
                LIMIT $3
                "#,
                channel_id,
                before_id,
                limit as i32
            )
            .fetch_all(&self.pool).await?
        } else {
            sqlx::query_as!(
                Message,
                r#"
                SELECT * FROM messages
                WHERE channel_id = $1
                ORDER BY timestamp DESC
                LIMIT $2
                "#,
                channel_id,
                limit as i32
            )
            .fetch_all(&self.pool).await?
        };

        Ok(result)
    }

    pub async fn delete_expired(&self) -> Result<usize, AppError> {
        let result: sqlx::QueryResult<sqlx::types::I64> = sqlx::query!(
            r#"DELETE FROM messages WHERE expires_at IS NOT NULL AND expires_at <= $1"#,
            chrono::Utc::now().timestamp_millis()
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as usize)
    }
}
```

## Migrations

### Migració 1 — Users

```sql
-- migrations/20260101000000_create_users.sql

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(50) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role VARCHAR(20) NOT NULL DEFAULT 'user',  -- 'user', 'admin'
    plan_id UUID,  -- Referència al plan SaaS
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Índex per cerca ràpida de username
CREATE INDEX idx_users_username ON users(username);
```

### Migració 2 — Devices

```sql
-- migrations/20260102000000_create_devices.sql

CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label VARCHAR(100),
    public_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX idx_devices_user ON devices(user_id);
CREATE INDEX idx_devices_public_key ON devices(public_key);
```

### Migració 3 — Servers

```sql
-- migrations/20260103000000_create_servers.sql

CREATE TABLE servers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    icon_url TEXT,
    owner_id UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_servers_owner ON servers(owner_id);
```

### Migració 4 — Server Members

```sql
-- migrations/20260104000000_create_server_members.sql

CREATE TABLE server_members (
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL DEFAULT 'member',
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (server_id, user_id)
);

-- Restricció: un usuari no pot ser el mateix owner dues vegades
-- (resolt a nivell d'aplicació)
```

### Migració 5 — Channels

```sql
-- migrations/20260105000000_create_channels.sql

CREATE TABLE channels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    channel_type VARCHAR(10) NOT NULL,
    encryption_type VARCHAR(10) NOT NULL DEFAULT 'none',
    message_ttl INTEGER,
    is_private BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_channels_server ON channels(server_id);

-- Verificar que el nom del canal sigui únic dins del mateix servidor
CREATE UNIQUE INDEX idx_channels_server_name ON channels(server_id, name);
```

### Migració 6 — Channel Keys

```sql
-- migrations/20260106000000_create_channel_keys.sql

CREATE TABLE channel_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    encrypted_key TEXT NOT NULL,
    kem_ciphertext TEXT NOT NULL,
    encryption_type VARCHAR(10) NOT NULL DEFAULT 'none',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(channel_id, device_id)
);

CREATE INDEX idx_channel_keys_channel ON channel_keys(channel_id);
CREATE INDEX idx_channel_keys_device ON channel_keys(device_id);
```

### Migració 7 — Messages

```sql
-- migrations/20260107000000_create_messages.sql

CREATE TABLE messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    sender_user_id UUID NOT NULL REFERENCES users(id),
    sender_device_id UUID NOT NULL REFERENCES devices(id),
    encrypted_payload TEXT NOT NULL,
    iv TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ,
    edited_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_messages_channel ON messages(channel_id, timestamp DESC);
CREATE INDEX idx_messages_expires ON messages(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_messages_sender ON messages(sender_user_id);
```

### Migració 20260123 — Attachments

```sql
-- migrations/20260123000000_create_attachments.sql

CREATE TABLE IF NOT EXISTS attachments (
    id UUID PRIMARY KEY,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    uploader_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    uploader_device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    object_key TEXT NOT NULL,
    status TEXT NOT NULL,          -- 'pending' | 'ready' | 'failed'
    upload_id TEXT NOT NULL,       -- S3 multipart upload ID
    chunk_size_bytes BIGINT NOT NULL,
    chunk_count INTEGER NOT NULL,
    algorithm TEXT,                -- 'aes-256-gcm'
    file_iv TEXT,                  -- Base64 nonce
    wrapped_file_key TEXT,         -- Base64 clau xifrada amb la clau del canal
    key_version_id UUID REFERENCES channel_key_versions(id),
    key_version INTEGER,
    ciphertext_sha256 TEXT,        -- SHA-256 del ciphertext per integritat
    completed_at TIMESTAMPTZ,
    thumbnail_attachment_id UUID REFERENCES attachments(id) -- Self-ref: thumbnail de la imatge
);

CREATE INDEX IF NOT EXISTS idx_attachments_channel_id ON attachments(channel_id);
CREATE INDEX IF NOT EXISTS idx_attachments_status ON attachments(status);
CREATE INDEX IF NOT EXISTS idx_attachments_key_version_id ON attachments(key_version_id);

-- Taula de relació N:M entre missatges i adjunts
CREATE TABLE IF NOT EXISTS message_attachments (
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    attachment_id UUID NOT NULL REFERENCES attachments(id) ON DELETE CASCADE,
    PRIMARY KEY (message_id, attachment_id)
);

CREATE INDEX IF NOT EXISTS idx_message_attachments_attachment_id ON message_attachments(attachment_id);
```

**Notes:**
- `thumbnail_attachment_id` és una FK a la mateixa taula `attachments`. El thumbnail s'emmagatzema com un adjunt independent (xifrat), i l'adjunt original hi apunta.
- Un thumbnail **no té** `thumbnail_attachment_id` (és `NULL`) per evitar recursió.
- El TTL cleanup esborra primer els thumbnails (via `thumbnail_attachment_id`) i després els adjunts originals.

---

### Migració 20260129 — Reply to Message

```sql
-- migrations/20260129000000_add_reply_to_message_id.sql

ALTER TABLE messages ADD COLUMN reply_to_message_id UUID REFERENCES messages(id) ON DELETE SET NULL;
```

---

### Migració 20260130 — Message Reactions

```sql
-- migrations/20260130000000_create_message_reactions.sql

CREATE TABLE message_reactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    username TEXT NOT NULL,
    emoji TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT uq_message_user_emoji UNIQUE (message_id, user_id, emoji)
);

CREATE INDEX idx_message_reactions_message ON message_reactions(message_id);
```

---

### Migració 8 — Plans (SaaS Tiers)

```sql
-- migrations/20260108000000_create_plans.sql

CREATE TABLE plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(50) UNIQUE NOT NULL,  -- 'free', 'pro', 'enterprise'
    display_name VARCHAR(50) NOT NULL,  -- 'Free', 'Professional', 'Enterprise'
    description TEXT,
    max_servers INT NOT NULL,
    max_channels_text_per_server INT NOT NULL,
    max_channels_voice_per_server INT NOT NULL,
    max_members_per_server INT NOT NULL,
    api_calls_per_minute INT NOT NULL,
    messages_per_day INT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Crear índex per lookup ràpid
CREATE INDEX idx_plans_name ON plans(name);

-- Inserir plans per defecte
INSERT INTO plans (
    id, name, display_name, description,
    max_servers, max_channels_text_per_server, max_channels_voice_per_server,
    max_members_per_server, api_calls_per_minute, messages_per_day
) VALUES
    -- Free Tier
    (
        '550e8400-e29b-41d4-a716-446655441001'::uuid,
        'free', 'Free',
        'Tier gratuïto per a usuaris individuals',
        1, 3, 2, 20, 60, 10000
    ),
    -- Pro Tier
    (
        '550e8400-e29b-41d4-a716-446655441002'::uuid,
        'pro', 'Professional',
        'Per a grups i petites organitzacions',
        5, 20, 10, 500, 600, -1  -- -1 = unlimited
    ),
    -- Enterprise Tier
    (
        '550e8400-e29b-41d4-a716-446655441003'::uuid,
        'enterprise', 'Enterprise',
        'Per a grans organitzacions amb suport personalitzat',
        -1, -1, -1, -1, -1, -1  -- Tots unlimited
    );
```

### Migració 9 — Add Plan Reference to Users

```sql
-- migrations/20260109000000_add_plan_to_users.sql

-- Afegir columna plan_id i relació amb plans
-- (La columna ja existeix, ara afegim la relació)
ALTER TABLE users
ADD CONSTRAINT fk_users_plan
    FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE SET NULL;

-- Assignar el plan 'free' per defecte a tots els usuaris existents
UPDATE users
SET plan_id = '550e8400-e29b-41d4-a716-446655441001'::uuid
WHERE plan_id IS NULL;

-- Fer la columna plan_id NOT NULL
ALTER TABLE users
ALTER COLUMN plan_id SET NOT NULL;

CREATE INDEX idx_users_plan ON users(plan_id);
```

### Migració 11 — Invitations

```sql
-- migrations/20260111000000_create_invitations.sql

CREATE TABLE invitations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(32) UNIQUE NOT NULL,  -- 32 chars alphanumeric + hyphens
    created_by_user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id UUID REFERENCES servers(id) ON DELETE SET NULL,
    max_uses INT NOT NULL DEFAULT 1,   -- -1 = unlimited, 0 = disabled
    uses_count INT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_invitations_code ON invitations(code);
CREATE INDEX idx_invitations_active ON invitations(is_active) WHERE is_active = true;
CREATE INDEX idx_invitations_created_by ON invitations(created_by_user_id);
CREATE INDEX idx_invitations_server_id ON invitations(server_id);
```

### Migració 19 — Invitation Server Binding

```sql
-- migrations/20260119000000_add_server_id_to_invitations.sql

ALTER TABLE invitations
ADD COLUMN IF NOT EXISTS server_id UUID REFERENCES servers(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_invitations_server_id ON invitations(server_id);
```

### Migració 18 — Channel Member Permissions

```sql
-- migrations/20260118000000_add_channel_member_permissions.sql

ALTER TABLE channel_members
        ADD COLUMN IF NOT EXISTS permission_level INTEGER NOT NULL DEFAULT 2;

UPDATE channel_members
SET permission_level = 2
WHERE permission_level IS NULL;

-- 1 = read, 2 = write, 3 = manage
ALTER TABLE channel_members
        ADD CONSTRAINT chk_channel_members_permission_level
        CHECK (permission_level BETWEEN 1 AND 3);
```

Permisos explícits utilitzats per backend:

- Canal:
    - `1`: read
    - `2`: write
    - `3`: manage
- Servidor:
    - `1`: view
    - `2`: manage_profile
    - `3`: manage_members

## Models de Dades (Rust Types)

```rust
// server/src/models/mod.rs

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: String,  // "user", "admin"
    pub plan_id: Uuid, // Referència al plan SaaS
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Plan {
    pub id: Uuid,
    pub name: String,                                    // 'free', 'pro', 'enterprise'
    pub display_name: String,
    pub description: Option<String>,
    pub max_servers: i32,                              // -1 = unlimited
    pub max_channels_text_per_server: i32,
    pub max_channels_voice_per_server: i32,
    pub max_members_per_server: i32,
    pub api_calls_per_minute: i32,
    pub messages_per_day: i32,
    pub created_at: DateTime<Utc>,
}

/// Límits actuals d'un usuari (combinació de plan + usage actual)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserLimits {
    pub plan: Plan,
    pub usage: UsageStats,
    pub can_create_server: bool,
    pub can_create_text_channel: bool,
    pub can_create_voice_channel: bool,
    pub remaining_servers: i32,
    pub remaining_text_channels: i32,
    pub remaining_voice_channels: i32,
}

/// Estadístiques de uso actuals d'un usuari
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_servers: i32,
    pub total_text_channels: i32,
    pub total_voice_channels: i32,
    pub total_members_across_servers: i32,
    pub messages_today: i32,
    pub api_calls_this_minute: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invitation {
    pub id: Uuid,
    pub code: String,                          // Codi únic (32 chars)
    pub created_by_user_id: Uuid,              // Qui va crear la invitació
    pub max_uses: i32,                         // -1 = unlimited, 0 = disabled
    pub uses_count: i32,                       // Vegades que s'ha usat
    pub is_active: bool,                       // Si és activa o no
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "admin")]
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Device {
    pub id: Uuid,
    pub user_id: Uuid,
    pub label: Option<String>,
    pub public_key: String,  // Base64 encoded Kyber-1024
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Server {
    pub id: Uuid,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ServerMember {
    pub server_id: Uuid,
    pub user_id: Uuid,
    pub role: String,  // "owner", "admin", "member"
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChannelMember {
    pub channel_id: Uuid,
    pub user_id: Uuid,
    pub permission_level: i32, // 1=read, 2=write, 3=manage
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, PartialEq)]
pub struct Channel {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub channel_type: ChannelType,
    pub encryption_type: EncryptionType,
    pub message_ttl: Option<i32>,
    pub is_private: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "voice")]
    Voice,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EncryptionType {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "symmetric")]
    Symmetric,
    #[serde(rename = "asymmetric")]
    Asymmetric,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChannelKey {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub device_id: Uuid,
    pub encrypted_key: String,   // Base64
    pub kem_ciphertext: String,  // Base64
    pub encryption_type: EncryptionType,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub sender_user_id: Uuid,
    pub sender_device_id: Uuid,
    pub encrypted_payload: String,  // Base64 AES-GCM
    pub iv: String,                 // Base64 nonce
    pub timestamp: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

## TTL Cleanup Service

```rust
// server/src/services/ttl_cleanup.rs
use tokio::time::{interval, Duration};
use tracing::{info, warn};
use crate::db::Database;

pub struct TTLCleanupService {
    db: Database,
    interval: Duration,
}

impl TTLCleanupService {
    pub fn new(db: Database, interval_minutes: u64) -> Self {
        Self {
            db,
            interval: Duration::from_minutes(interval_minutes),
        }
    }

    pub async fn run(&self) {
        let mut ticker = interval(self.interval);
        loop {
            ticker.tick().await;
            match self.cleanup_expired().await {
                Ok(count) => {
                    info!("TTL Cleanup: {} expired messages deleted", count);
                }
                Err(e) => {
                    warn!("TTL Cleanup error: {}", e);
                }
            }
        }
    }

    async fn cleanup_expired(&self) -> Result<usize, AppError> {
        let deleted = self.db.pool().delete_expired_messages().await?;
        Ok(deleted)
    }
}
```

## Configuració SQLite vs PostgreSQL

### .env

```bash
# Servidor
SERVER_HOST=0.0.0.0
SERVER_PORT=8080
JWT_SECRET=your-secret
JWT_EXPIRATION_DAYS=7

# Registre
OPEN_REGISTER=true             # true = registre obert, false = només admin pot crear usuaris
# Si OPEN_REGISTER=false, requereix aquestes credencials:
ADMIN_USER=admin               # Username del administrador
ADMIN_PASSWORD=changeme        # Contrasenya del administrador

# Database
DATABASE_TYPE=postgres          # o "sqlite"
DATABASE_URL=postgresql://user:pass@localhost:5432/chillgroup
# Per SQLite: DATABASE_URL=sqlite://chillgroup.db

# TTL Cleanup
TTL_CLEANUP_INTERVAL_MINUTES=5

# LiveKit
LIVEKIT_HOST=wss://livekit.example.com
LIVEKIT_API_KEY=your-key
LIVEKIT_API_SECRET=your-secret

# Server Master Key (per encriptar claus simètriques)
SERVER_MASTER_KEY=hex-encoded-32-byte-key
```

### Càrrega de Config

```rust
// server/src/config.rs
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub database_type: DbType,
    pub database_url: String,
    pub ttl_cleanup_interval: u64,
    pub livekit_host: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    pub jwt_secret: String,
    pub jwt_expiration_days: u32,
    pub server_master_key: Option<String>,
    pub open_register: bool,     // Si false, només admin pot crear usuaris
    pub admin_user: String,      // Username inicial del admin
    pub admin_password: String,  // Contrasenya inicial del admin
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        Ok(Self {
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            server_port: env::var("SERVER_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            database_type: match env::var("DATABASE_TYPE").as_deref() {
                Ok("sqlite") => DbType::SQLite,
                _ => DbType::Postgres,
            },
            database_url: env::var("DATABASE_URL")
                .expect("DATABASE_URL is required"),
            ttl_cleanup_interval: env::var("TTL_CLEANUP_INTERVAL_MINUTES")
                .ok()
                .and_then(|i| i.parse().ok())
                .unwrap_or(5),
            livekit_host: env::var("LIVEKIT_HOST")
                .expect("LIVEKIT_HOST is required"),
            livekit_api_key: env::var("LIVEKIT_API_KEY")
                .expect("LIVEKIT_API_KEY is required"),
            livekit_api_secret: env::var("LIVEKIT_API_SECRET")
                .expect("LIVEKIT_API_SECRET is required"),
            jwt_secret: env::var("JWT_SECRET")
                .expect("JWT_SECRET is required"),
            jwt_expiration_days: env::var("JWT_EXPIRATION_DAYS")
                .ok()
                .and_then(|d| d.parse().ok())
                .unwrap_or(7),
            server_master_key: env::var("SERVER_MASTER_KEY").ok(),
            open_register: env::var("OPEN_REGISTER")
                .unwrap_or_else(|_| "true".into())
                .parse()
                .unwrap_or(true),
            admin_user: env::var("ADMIN_USER")
                .unwrap_or_else(|_| "admin".into()),
            admin_password: env::var("ADMIN_PASSWORD")
                .expect("ADMIN_PASSWORD is required when OPEN_REGISTER=false"),
        })
    }
}
```

## Notes sobre Migracions SQLite

Per a SQLite, cal adaptar algunes migracions:

```sql
-- Per a SQLite, les funcions gen_random_uuid() no existeixen.
-- Solució: usar CREATE TABLE amb trigger o usar una funció auxiliar.

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

En producció, la migració es pot gestionar amb una visió condicional o un script de migració que injecti UUIDs.
