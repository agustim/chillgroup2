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

## Models de Dades (Rust Types)

```rust
// server/src/models/mod.rs

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub server_master_key: Option<String>,  // Només per nivell 1
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
