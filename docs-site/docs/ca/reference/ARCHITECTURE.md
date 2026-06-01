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
│   │   │   ├── presence_service.rs   # WebSocket presence tracking
│   │   │   ├── admin_service.rs      # Admin user management
│   │   │   ├── plan_service.rs       # SaaS plans CRUD
│   │   │   └── limit_service.rs      # Feature limit enforcement
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

## Adjunts Xifrats (S3-Compatible)

### Principis

1. El backend és zero-knowledge per contingut: el fitxer es xifra al client abans d'arribar a S3.
2. L'objecte guardat a S3 és sempre ciphertext.
3. El backend guarda metadades i permisos, però no pot desxifrar el contingut.
4. L'upload es fa per chunks (multipart) per suportar fitxers grans i reintents parcials.

### Components

```
Client (Web/Mobile)
    - xifra el fitxer localment
    - divideix en chunks
    - puja parts a S3 amb URLs signades

Rust Server
    - valida permisos de canal
    - crea uploads i signa URLs S3
    - persisteix metadades i envelope criptogràfic

S3-compatible storage (RustFS en dev / S3 en prod)
    - guarda objectes ciphertext
    - no veu claus de desxifrat
```

### Flux d'Upload

1. `init`: el client demana iniciar adjunt per un canal.
2. `sign-part`: el backend retorna URL signada per cada chunk.
3. `put part`: el client puja chunks ja xifrats directament a S3.
4. `complete`: el backend tanca multipart i guarda metadades + dades de xifrat.
5. `send message`: el missatge referencia `attachment_id`.

### Metadades persistides de fitxer

- `file_name`
- `mime_type`
- `size_bytes`
- `created_at`

### Metadades criptogràfiques persistides

- `algorithm` (ex: `aes-256-gcm` o esquema per chunks)
- `file_iv` o metadades de nonce per chunks
- `wrapped_file_key`
- `key_version_id`
- `key_version`
- `chunk_size_bytes`
- `chunk_count`
- `ciphertext_sha256`

### Rotació de claus

Els adjunts segueixen la mateixa estratègia de versioning que missatges:

1. Cada adjunt queda lligat a una versió concreta de clau (`key_version_id`, `key_version`).
2. Si el client no té la clau d'aquella versió, la demana explícitament com amb missatges.
3. La rotació només afecta nous adjunts/missatges; l'històric manté la seva versió.

### Antivirus i miniatures

1. No hi ha antivirus server-side sobre plaintext (per disseny zero-knowledge).
2. Si es vol miniatura, la genera el client i la puja com un segon adjunt relacionat.
3. El missatge pot referenciar `attachment_id` principal i `thumbnail_attachment_id` opcional.

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

### Model de Permisos Explícits

Permisos de servidor (resolts per backend):

- `1` -> `view` (accedir a informació i llistats)
- `2` -> `manage_profile` (editar nom/icona)
- `3` -> `manage_members` (convidar i gestionar rols)

Mapeig actual per rol:

- `owner` -> `3`
- `admin` -> `3`
- `member` -> `1`

Permisos de canal (resolts per backend):

- `1` -> `read`
- `2` -> `write`
- `3` -> `manage`

Si existeix `channel_members.permission_level`, aquest valor és la font de veritat tant en canals privats com públics.
Si no existeix override explícit, en canals públics `member` té `write` i `owner/admin` tenen `manage`.
En canals privats sense override explícit, l'usuari no té accés al canal.
En canals `scope=dm`, ambdós membres tenen `manage`.

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

## Sistema d'Administració

### Configuració

El sistema pot funcionar en dos modes configurables:

#### Modo 1: Registre Obert (`OPEN_REGISTER=true`)
- Els usuaris es poden registrar lliurement via `POST /api/auth/register`
- No hi ha rol d'administrador necessari
- Ideal per a comunitats públiques

#### Modo 2: Registre Restringit (`OPEN_REGISTER=false`)
- L'endpoint `POST /api/auth/register` retorna 403 Forbidden
- Només els administradors poden crear usuaris via `POST /api/admin/users`
- Al iniciar, es crea automàticament un administrador inicial amb les credencials:
  - Username: valor de variable `ADMIN_USER` (.env)
  - Password: valor de variable `ADMIN_PASSWORD` (.env)
- Ideal per a instàncies privades o corporatives

### Autenticació d'Administrador

1. El usuari admin fa login normalment via `POST /api/auth/login`
2. El JWT retorna `isAdmin: true` a la resposta
3. Els endpoints `/api/admin/**` verifiquen que el token tingui `isAdmin: true`
4. Si no, retornen 403 Forbidden

### Operacions d'Administració

Els administradors poden:

- **Llistar usuaris**: `GET /api/admin/users` (amb paginació)
- **Crear usuaris**: `POST /api/admin/users` (independent de `OPEN_REGISTER`)
- **Modificar usuaris**: `PUT /api/admin/users/:userId` (username, password, role)
- **Esborrar usuaris**: `DELETE /api/admin/users/:userId` (cascada total)

### Cascada d'Esborrat

Quan un administrador esborra un usuari, es suprimeixen:
- Tots els seus dispositius
- Tots els seus servidors (transferencia de propiedad opcional en futur)
- Tots els seus canals dins de servidors
- Tots els seus missatges
- Totes les seves relacions d'amics
- Totes les seves claus de canal

### Seguretat d'Admin

- Els tokens d'administrador són JWT normals amb flag `isAdmin`
- Els administradors no tenen accés especial a dades xifrades (mantenen la privacesa E2EE)
- Els logs d'operacions d'administrador es registren per auditoria (futur)
- La contrasenya d'admin inicial es recommana canviar immediatament

## Sistema de Plans/Tiers i Límits de Features

### Arquitectura de Plans (SaaS)

ChillGroup suporta 3 tiers de SaaS predefinits amb límits configurables per tier:

#### Tiers Per Defecte

| Feature | Free | Pro | Enterprise |
|---------|------|-----|------------|
| **Servidors** | 1 | 5 | Unlimited |
| **Canals Text/Servidor** | 3 | 20 | Unlimited |
| **Canals Veu/Servidor** | 2 | 10 | Unlimited |
| **Members/Servidor** | 20 | 500 | Unlimited |
| **API Calls/min** | 60 | 600 | Unlimited |
| **Missatges/dia** | 10,000 | Unlimited | Unlimited |

**Nota:** `-1` (o `null`) significa "sense límit" en la BD.

### Estructura de BD

**Taula `plans`**: Conté la definició de cada tier
- **PK**: `id` (UUID)
- **Unique**: `name` ('free', 'pro', 'enterprise')
- **Camps de límit**: `max_servers`, `max_channels_text_per_server`, etc.

**Relació `users.plan_id → plans.id`**: Cada usuari apartat a un plan al crear-se (por defecte "free")

### Verificació de Límits (Hard Limits)

Quan un usuari intenta crear un recurs (servidor, canal, etc), es verifica:

1. **Obtenció del plan de l'usuari**: `SELECT * FROM plans WHERE id = user.plan_id`
2. **Recompte de recursos actuals**: `SELECT COUNT(*) FROM servers WHERE owner_id = user_id`
3. **Comparació**: Si `current >= limit`, retorna **429 Too Many Requests**

**Fluxe d'exemple** (crear servidor):
```
POST /api/servers (nom: "My Server")
  ├─ Autenticar usuari (JWT)
  ├─ Get plan: user.plan_id = "free" → max_servers = 1
  ├─ Count: SELECT COUNT(*) FROM servers WHERE owner_id = user_id → 1
  ├─ Check: 1 >= 1? SÍ ❌
  └─ Response 429: "Has assolit el límit de servidors (1/1)"
```

### Services de Límits

#### `PlanService`
- `get_all_plans()` — Obtenir tots els plans
- `get_plan_by_id(uuid)` — Plan específic
- `get_plan_by_name(name: &str)` — Plan per name ('free', 'pro', etc)
- `create_plan(input)` — Admin only: crear plans personalitzats
- `update_plan(id, input)` — Admin only: modificar plans personalitzats
- `delete_plan(id)` — Admin only: eliminar plans personalitzats

Regles de protecció:
- Plans del sistema (`free`, `pro`, `enterprise`) no es poden modificar ni eliminar.
- Un plan no es pot eliminar si té usuaris assignats.

#### `LimitService`
- `get_user_limits(user_id)` — Obtenir plan + usage stats
- `can_create_server(user_id)` — Retorna bool
- `can_create_text_channel(user_id, server_id)` — Retorna bool
- `can_create_voice_channel(user_id, server_id)` — Retorna bool
- `check_limit(user_id, resource_type, extra_context)` — Generic check
- `check_api_rate_limit(user_id)` — Verificar calls/min
- `check_message_daily_limit(user_id)` — Verificar messages/dia

### Middleware de Rate Limiting

Per a `api_calls_per_minute` i `messages_per_day`, s'usa:
- **Redis (producció)**: Contador en temps real per user_id
- **In-memory cache (dev)**: Simple HashMap amb TTL

Sense limits, la BD és interrogada al request. Amb Redis, es cache de 1 min en 1 min.

### Cambio de Plan (Admin)

Els administradors poden canviar el plan d'un usuari via:
```
PUT /api/admin/users/:userId/plan/:planId
```

**Efectes immediats:**
- Noves creacions de recursos respecten el nou límit
- Els recursos existents NO es suprimeixen (no degrading forçat)
- Exemple: Downgrade de "pro" a "free" amb 3 servidors → Els 3 servidors romanen, però no es pot crear més

### Endpoints de Plans per a Usuaris

- `GET /api/plans` — Llistar plans (usuari autenticat)
- `GET /api/user/me/plan` — Plan actual + limits + usage
- `POST /api/user/me/check-limits` — Check generic sense crear res

### Endpoints d'Administració de Plans

- `GET /api/admin/plans` — Llistar tots els plans amb marca `isSystem`
- `POST /api/admin/plans` — Crear un plan custom
- `PUT /api/admin/plans/:planId` — Actualitzar un plan custom
- `DELETE /api/admin/plans/:planId` — Eliminar un plan custom (si no està en ús)

### Futur: Webhook Charges & Billing

Els limits actualment és per **feature gating**, no per **billing**. En futur:
- Stripe/Paddle integration per cobrar per upgrades
- Webhooks per track "overages" (ex: 1000 missatges extra/mes)
- Escalabilitat de tiers dinàmics per empresa

### Invitation Service

#### Responsabilitats
- Generar codis d'invitació únics (32 chars alphanumeric + hyphens)
- Validar codis d'invitació al registre
- Comptar usos i enforçar límits
- Invalidar invitacions
- Administrar vigència (no expiran automàticament, es controla manualment amb `is_active`)

#### Interfície Pública

```rust
pub trait InvitationService {
    // Crear invitació
    async fn create_invitation(&self, admin_id: Uuid, max_uses: i32) -> Result<Invitation>;
    
    // Validar codi i obté invitació
    async fn validate_code(&self, code: &str) -> Result<Invitation>;
    
    // Incrementar compte d'usos
    async fn use_invitation(&self, code: &str) -> Result<()>;
    
    // Invalidar invitació
    async fn invalidate(&self, id: Uuid, admin_id: Uuid) -> Result<()>;
    
    // Llistar invitacions (admin only)
    async fn list_invitations(&self, admin_id: Uuid, limit: i64, offset: i64) -> Result<Vec<Invitation>>;
    
    // Obtenir invitació per ID
    async fn get_invitation(&self, id: Uuid) -> Result<Invitation>;
}
```

#### Flux de Registre amb Invitació

```
1. Frontend: POST /api/auth/register-with-invitation
   {
       "code": "abc123-...",
       "username": "newuser",
       "password": "secret"
   }

2. Backend:
   ├─ Validar codi: InvitationService::validate_code(code)
   │  └─ SELECT FROM invitations WHERE code = code AND is_active = true
   │     └─ Si no existeix: Return 404
   │     └─ Si no activa: Return 404
   │
   ├─ Comprovar límit d'usos:
   │  └─ Si max_uses != -1 && uses_count >= max_uses: Return 410
   │
   ├─ Crear usuari (igual que register normal)
   │  └─ Hash password, crear JWT, device inicial
   │
   ├─ Incrementar uses_count:
   │  └─ UPDATE invitations SET uses_count = uses_count + 1 WHERE id = invitation.id
   │
   └─ Response 201 amb token
```

#### Codi d'Invitació

- **Format**: 32 chars, [A-Z0-9-]
- **Exemples**: `ABC123-DEF456-GHI789-XYZ000-001`, `TEMP-ADMIN-INIT-CODE-999999999999`
- **Generació**: `rand::thread_rng().gen_string(32)` o similar
- **Unicitat**: UNIQUE constraint a BD

#### Administració

Admins poden:
- Crear invitacions amb `maxUses` (default 1, -1 = unlimited)
- Llistar invitacions actives i estadístiques
- Invalidar invitacions manualment
- No hi ha expiració automàtica; es control manualment via `is_active`

#### Comportament Esqecial

- **OPEN_REGISTER=true**: Endpoint POST /api/auth/register-with-invitation funciona igualment
- **OPEN_REGISTER=false**: POST /api/auth/register-with-invitation és l'ÚNICA forma de registrarse
- **Usuari inicial**: Si no existeix cap admin, s'assigna rol `admin` al primer usuari que es registra (amb o sense invitació)

