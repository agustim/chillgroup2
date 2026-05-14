# ChillGroup v2 — Arquitectura del Sistema

## Diagrama d'Arquitectura

```
                    ┌─────────────────────────────────────────────────┐
                    │                  CLIENTS                        │
                    │  React SPA (TypeScript)  │  Mobile (futur)      │
                    └──────────────┬──────────────────────────────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              │                    │                     │
    ┌─────────▼────────┐  ┌──────▼───────┐  ┌──────────▼──────────┐
    │  WebSocket /     │  │  WebSocket / │  │  WebRTC (LiveKit)   │
    │  HTTP REST       │  │  HTTP        │  │  (Àudio/Vídeo E2EE) │
    │  (Socket.IO)     │  │  (Socket.IO) │  │                     │
    └─────────┬────────┘  └──────┬───────┘  └──────────┬──────────┘
              │                  │                     │
              │            ┌────▼───────────────────────▼────┐
              │            │      RUST SERVER (Axum)          │
              │            │  ┌─────────────────────────────┐ │
              │            │  │  API Layer (Axum routes)     │ │
              │            │  └──────────┬──────────────────┘ │
              │            │  ┌──────────▼──────────────────┐ │
              │            │  │  Service Layer               │ │
              │            │  │  - AuthService               │ │
              │            │  │  - ChannelService            │ │
              │            │  │  - MessageService            │ │
              │            │  │  - CryptoService             │ │
              │            │  │  - LiveKitService            │ │
              │            │  └──────────┬──────────────────┘ │
              │            │  ┌──────────▼──────────────────┐ │
              │            │  │  Repository Layer            │ │
              │            │  │  - UserRepository            │ │
              │            │  │  - ChannelRepository         │ │
              │            │  │  - MessageRepository         │ │
              │            │  │  - KeyRepository             │ │
              │            │  └──────────┬──────────────────┘ │
              │            │  ┌──────────▼──────────────────┐ │
              │            │  │  Storage Interface (SQLx)    │ │
              │            │  └──────────┬──────────────────┘ │
              │            └─────────────┬───────────────────┘ │
              │                      ┌───▼──┐  ┌─────────────┐ │
              │                      │Postgre│  │  SQLite     │ │
              │                      │  SQL  │  │  (dev)      │ │
              │                      └───────┘  └─────────────┘ │
              │                                                  │
              │              ┌────────────────────────────────┐  │
              │              │     LiveKit Server (extern)     │  │
              │              │  - SFU per àudio/vídeo          │  │
              │              │  - E2EE amb session keys        │  │
              │              │  - Recorde opcional             │  │
              │              └────────────────────────────────┘  │
              └──────────────────────────────────────────────────┘
```

## Components del Servidor Rust

### Estructura de Cargo Workspace

```
chillgroup/
├── Cargo.toml              # Workspace root
├── Cargo.lock
├── server/                 # Servidor principal
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs           # Entry point, init DB + routes
│   │   ├── config.rs         # Configuració (.env, variables)
│   │   ├── db.rs             # DB connection pool (PostgreSQL/SQLite)
│   │   │
│   │   ├── routes/           # Axum HTTP/WebSocket handlers
│   │   │   ├── auth.rs         # Register, login, JWT
│   │   │   ├── users.rs        # User profiles, devices
│   │   │   ├── servers.rs      # Server CRUD
│   │   │   ├── channels.rs     # Channel CRUD, encryption setup
│   │   │   ├── messages.rs     # Send, receive, history
│   │   │   ├── livekit.rs      # LiveKit room provisioning
│   │   │   └── ws.rs             # Socket.IO WebSocket handlers
│   │   │
│   │   ├── services/         # Business logic
│   │   │   ├── auth_service.rs
│   │   │   ├── channel_service.rs
│   │   │   ├── message_service.rs
│   │   │   ├── crypto_service.rs     # Encryption operations
│   │   │   ├── livekit_service.rs    # LiveKit integration
│   │   │   └── presence_service.rs   # WebSocket presence tracking
│   │   │
│   │   ├── repositories/     # Data access layer
│   │   │   ├── user_repo.rs
│   │   │   ├── channel_repo.rs
│   │   │   ├── message_repo.rs
│   │   │   ├── key_repo.rs
│   │   │   └── server_repo.rs
│   │   │
│   │   ├── models/           # Data models (sqlx queries + types)
│   │   │   ├── user.rs
│   │   │   ├── channel.rs
│   │   │   ├── message.rs
│   │   │   ├── server.rs
│   │   │   ├── device.rs
│   │   │   └── key.rs
│   │   │
│   │   ├── crypto/           # Cryptography module
│   │   │   ├── kyber.rs          # Kyber-1024 KEM
│   │   │   ├── aes_gcm.rs        # AES-256-GCM
│   │   │   ├── channel_keys.rs   # Channel key management
│   │   │   └── hash.rs           # Password hashing (argon2)
│   │   │
│   │   ├── middleware/       # Axum middleware
│   │   │   ├── auth.rs           # JWT extraction/verification
│   │   │   └── cors.rs           # CORS configuration
│   │   │
│   │   └── error.rs          # Unified error types
│   └── migrations/           # SQLx migrations
│       ├── 20260101000000_create_users.sql
│       ├── 20260101000001_create_devices.sql
│       └── ...
├── shared/                   # Crate compartit client-servidor
│   ├── Cargo.toml
│   └── src/
│       ├── types.rs            # Types compartits
│       └── constants.rs        # Constants del domini
└── migrations/               # SQLx migrate executable (CLI)
    └── ...
```

### Dependències Principals (Cargo.toml)

```toml
[workspace]
resolver = "2"
members = ["server", "shared"]

[workspace.dependencies]
# Web
axum = { version = "0.8", features = ["ws", "macros"] }
tokio = { version = "1", features = ["full"] }
socketioxide = "0.20"  # Socket.IO server natiu per Rust
hyper = "1"
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }

# Database
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "sqlite", "uuid", "time", "json"] }
deadpool-sqlx = "0.9"  # Connection pool amb deadpool

# Cryptography
x25519-dilithium = "2.1"      # ML-KEM-1024 (Kyber) de RustCrypto
aes-gcm = { version = "0.10", features = ["std"] }
argon2 = "0.5"                   # Password hashing
rand = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }

# Auth
jsonwebtoken = "9"
base64 = "0.22"

# Config
dotenvy = "0.15"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# LiveKit
livekit-server-sdk = "0.3"

# Utilities
async-trait = "0.1"
thiserror = "2"
```

## Capes de l'Aplicació

### 1. API Layer (Axum + Socket.IO)

Responsabilitats:
- Ruteig HTTP i WebSocket
- Extracció i validació de JWT
- Serialització/deserialització JSON
- Rate limiting

```rust
// Exemple de rutes
fn create_router() -> Router {
    Router::new()
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/servers", get(servers::list).post(servers::create))
        .route("/api/servers/{id}", get(servers::get))
        .route("/api/servers/{id}/channels", get(channels::list).post(channels::create))
        .route("/api/channels/{id}/messages", get(messages::list).post(messages::send))
        .route("/api/livekit/token", post(livekit::token))
        .with_state(app_state)
}
```

### 2. Service Layer

Cada servei conté la lògica de negoci específica:

```rust
#[async_trait]
pub trait ChannelService: Send + Sync {
    // Crear canal amb diferents nivells de xifratge
    async fn create_channel(
        &self,
        server_id: Uuid,
        name: &str,
        channel_type: ChannelType,
        encryption: EncryptionType,
        creator_device_id: Uuid,
    ) -> Result<Channel>;

    // Convidar membre amb encriptació simètrica o asimètrica
    async fn invite_member(
        &self,
        channel_id: Uuid,
        target_device_ids: Vec<Uuid>,
    ) -> Result<()>;

    // Verificar si un dispositiu té accés a un canal
    async fn has_access(
        &self,
        channel_id: Uuid,
        device_id: Uuid,
    ) -> Result<bool>;
}
```

### 3. Repository Layer (SQLx)

Abstracció sobre SQLx amb suport PostgreSQL + SQLite:

```rust
#[async_trait]
pub trait MessageRepository: Send + Sync {
    async fn create(&self, msg: &Message) -> Result<Message>;
    async fn get_by_channel(
        &self,
        channel_id: Uuid,
        limit: usize,
        before: Option<Uuid>,
    ) -> Result<Vec<Message>>;
    async fn delete_expired(&self) -> Result<usize>;
}

// Implementació PostgreSQL
impl MessageRepository for PgMessageRepo { ... }

// Implementació SQLite
impl MessageRepository for SqliteMessageRepo { ... }
```

### 4. Storage Layer (SQLx)

```rust
pub struct Database {
    pub pg_pool: Option<PgPool>,
    pub sqlite_pool: Option<SqlitePool>,
}

impl Database {
    pub async fn new(config: &Config) -> Result<Self> {
        match config.db_type {
            DbType::Postgres => {
                let pool = PgPool::connect(&config.database_url).await?;
                Ok(Self {
                    pg_pool: Some(pool),
                    sqlite_pool: None,
                })
            }
            DbType::SQLite => {
                let pool = SqlitePool::connect(&config.database_url).await?;
                Ok(Self {
                    pg_pool: None,
                    sqlite_pool: Some(pool),
                })
            }
        }
    }
}
```

## Integració LiveKit

### Arquitectura LiveKit

```
┌─────────────────────────────────────────────────────────────┐
│                    Rust Server                              │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  LiveKitService                                        │  │
│  │  - Crea room a LiveKit                                   │  │
│  │  - Genera Access Token per participant                   │  │
│  │  - Configura E2EE settings                               │  │
│  └──────────────────────────────────────────────────────┘  │
└──────────────────┬──────────────────────────────────────────┘
                   │ REST API (LiveKit SDK)
                   │ GenerateToken(room, participant)
┌──────────────────▼──────────────────────────────────────────┐
│                   LiveKit Server                             │
│  ┌─────────────┐  ┌──────────────┐  ┌────────────────────┐ │
│  │  SFU Router  │  │  E2EE Engine │  │  Recorder          │ │
│  │  (mix/split) │  │  Session Key │  │  (opcional)        │ │
│  └─────────────┘  └──────────────┘  └────────────────────┘ │
└──────────────────┬──────────────────────────────────────────┘
                   │ WebRTC
    ┌──────────────┼──────────────┐
    │              │              │
 ▼  ▼  ▼      ▼  ▼  ▼      ▼  ▼  ▼
Client 1   Client 2   Client 3  ...  (E2EE amb session keys)
```

### Flux de Connexió a Veu

```rust
// 1. Frontend demana token a Rust server
POST /api/livekit/token
{
    "channel_id": "uuid",
    "server_id": "uuid",
    "user_id": "uuid",
    "device_id": "uuid"
}

// 2. Rust server genera token amb LiveKit SDK
pub async fn generate_token(
    channel_id: Uuid,
    user_id: Uuid,
    participant_name: String,
) -> Result<LiveKitToken> {
    // Verificar accés al canal
    let channel = channel_repo.get(channel_id).await?;
    let server = server_repo.get(channel.server_id).await?;
    if !server.is_member(user_id) && !server.is_owner(user_id) {
        return Err(Error::Forbidden);
    }

    // Generar token LiveKit amb E2EE habilitat
    let room_name = format!("server-{}-channel-{}", server.id, channel.id);
    let mut token = Token::new(livekit_api_key, &room_name)
        .set_participant_identity(participant_name)
        .set_name(participant_name)
        .set_can_publish(true)
        .set_can_subscribe(true)
        .set_can_publish_data(true);

    if channel.encryption_type == EncryptionType::E2EE {
        token.set_can_subscribe(true);
        // El session key es gestiona al client amb LiveKit E2EE
    }

    Ok(token.sign())
}
```

### LiveKit E2EE

LiveKit suporta encriptació E2EE nativa amb **session keys**:

```typescript
// Frontend TypeScript — LiveKit E2EE
import { Room, RoomEvent } from '@livekit/components-react'

room.on(RoomEvent.E2EEStateStatus, (state, participant) => {
    if (state === 'Enabled') {
        // Session key generada al client
        console.log('E2EE actiu per al participant:', participant.identity)
    }
})

// Configurar session key per encriptar àudio/vídeo
await room.setE2EE(true, {
    key: channelSessionKey,  // Clau compartida entre membres del canal
    keyStore: cryptoKeyStore, // IndexedDB local
})
```

El Rust server només gestiona la distribució inicial de session keys (via canal de text encriptat), no veu el trànsit real d'àudio/vídeo.

## Criptografia

### Nivell 0: Sense Criptografia

Missatges en text pla. Ideal per canals públics.

```
Client → POST /messages → Servidor guarda text pla → Socket.IO broadcast
```

### Nivell 1: Clau Simètrica (AES-GCM-256)

Tots els membres comparteixen la mateixa clau AES-256.

```
1. Creator genera: channelKey = AES-256 random key
2. Servidor guarda: { channel_id, encrypted_key: AES-GCM(channelKey, admin_key) }
3. Membre obté channelKey → desencripta amb admin_key → cauca a IndexedDB
4. Missatges: AES-GCM(channelKey, message) → Servidor guarda xifrat → broadcast
5. Receptor: recupera channelKey → AES-GCM decrypt
```

### Nivell 2: Clau Asimètrica (Kyber-1024 + AES-GCM)

E2EE veritable, zero-knowledge.

```
1. Creator genera: channelKey (AES-256)
2. Per cada membre (amb la seva publicKey Kyber):
   - KEM.Encrypt(member.publicKey) → (sharedSecret, ciphertext)
   - KEK = HKDF(sharedSecret)
   - encryptedChannelKey = AES-GCM(KEK, channelKey)
3. Servidor guarda: { channel_id, device_id, encrypted_channel_key, ciphertext }
4. Membre rep missatge:
   - KEM.Decrypt(ciphertext, member.secretKey) → sharedSecret
   - KEK = HKDF(sharedSecret)
   - channelKey = AES-GCM.Decrypt(KEK, encrypted_channel_key)
   - message = AES-GCM.Decrypt(channelKey, encrypted_message)
5. Servidor NO pot llegir res
```

## Base de Dades

### Model Relacional

```sql
-- Totes les taules existents però reestructurades per a Rust/SQLx

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(50) UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    label VARCHAR(100),
    public_key TEXT NOT NULL,       -- Base64 Kyber-1024 publicKey
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen TIMESTAMPTZ NOT NULL DEFAULT now(),
    revoked BOOLEAN NOT NULL DEFAULT false
);

CREATE TABLE servers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    icon_url TEXT,
    owner_id UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE server_members (
    server_id UUID REFERENCES servers(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(20) NOT NULL DEFAULT 'member',  -- owner | admin | member
    joined_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (server_id, user_id)
);

CREATE TABLE channels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    channel_type VARCHAR(10) NOT NULL CHECK (channel_type IN ('text', 'voice')),
    encryption_type VARCHAR(10) NOT NULL DEFAULT 'none'
        CHECK (encryption_type IN ('none', 'symmetric', 'asymmetric')),
    message_ttl INTEGER,  -- NULL = permanent
    is_private BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE channel_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    encrypted_key TEXT NOT NULL,    -- Base64 ciphertext (Kyber o AES)
    kem_ciphertext TEXT NOT NULL,   -- Base64 KEM ciphertext
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(channel_id, device_id)
);

CREATE TABLE messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    sender_user_id UUID NOT NULL REFERENCES users(id),
    sender_device_id UUID NOT NULL REFERENCES devices(id),
    encrypted_payload TEXT NOT NULL,   -- Base64 encrypted message
    iv TEXT NOT NULL,                  -- Base64 initialization vector
    timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ,
    edited_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_messages_channel ON messages(channel_id, timestamp DESC);
CREATE INDEX idx_messages_expires ON messages(expires_at) WHERE expires_at IS NOT NULL;
CREATE INDEX idx_channel_keys_channel ON channel_keys(channel_id);
CREATE INDEX idx_channel_keys_device ON channel_keys(device_id);
CREATE INDEX idx_server_members_user ON server_members(user_id);
```

### SQLite Compatibility

Amb SQLx, les migracions són SQL pur. Per a SQLite:

```sql
-- Diferències clau per a SQLite:
-- 1. No hi ha CHECK constraints (a SQLite 3.36+)
-- 2. TIMESTAMPTZ → DATETIME
-- 3. gen_random_uuid() → replace(lower(hex(randomblob(4))),'-','')
-- 4. No hi ha ON DELETE CASCADE en versions antigues
```

SQLx resolt aquestes diferències amb feature flags i queries condicional.

## Seguretat

### Passwords
- `argon2` amb salt random de 16 bytes
- `time_cost=2`, `mem_cost=65536`, `parallelism=1`
- Hash guardat a `users.password_hash`

### JWT
- Algoritme: RS256 (clau pública/privada) o HS256 (simple)
- Expiració: 7 dies (access) + 30 dies (refresh)
- Conté: `user_id`, `device_id`, `exp`, `iat`, `jti`

### Rate Limiting
- Login: 5 intents / 15 min per IP
- Missatges: 30/min per canal per usuari
- Registre: 3 comptes / 24h per IP

### WebSocket
- Conexions per usuari: 5 (màxim)
- Ping/pong cada 30s
- Timeout de inactivitat: 60s

## Observabilitat

- **Logging**: `tracing` + `tracing-subscriber` (JSON a prod, console a dev)
- **Metrics**: Prometheus client (`prometheus` crate) per request latency, errors, presència
- **Health check**: `GET /health` (DB connection, LiveKit connectivity)
- **Tracing**: Request ID propagat via headers (`X-Request-ID`)
