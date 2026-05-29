# ChillGroup v2 — Pla de Desenvolupament

## Resum de Canvis respecte v1

| Aspecte | v1 | v2 |
|---------|-----|-----|
| Server | Node.js + TypeScript | Rust |
| Framework server | Express.js | Axum |
| Temps real | Socket.IO (Node) | Socket.IO (Rust) / Axum WS |
| Àudio/Vídeo | LiveKit (E2EE manual) | LiveKit (E2EE nativa) |
| Criptografia | Només E2EE (Kyber) | 3 nivells (none/symmetric/asymmetric) |
| BD | PostgreSQL + Durable Objects + SQLite | PostgreSQL + SQLite (via SQLx) |
| Projecte | Monorepo Node | Cargo workspace |

## Workspace Structure

```
chillgroup/
├── Cargo.toml              # Workspace root
├── server/                 # Servidor Rust
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── config.rs
│       ├── db.rs
│       ├── routes/         # Axum handlers
│       ├── services/       # Business logic
│       ├── repositories/   # Data access (SQLx)
│       ├── models/         # Data types
│       ├── crypto/         # Cryptography module
│       ├── middleware/     # Auth, CORS
│       └── error.rs
├── shared/                 # Crate compartit client-servidor
│   ├── Cargo.toml
│   └── src/
│       ├── types.rs
│       └── constants.rs
├── migrations/             # SQLx migrations
│   ├── 20260101000000_create_users.sql
│   ├── 20260102000000_create_devices.sql
│   ├── 20260103000000_create_servers.sql
│   ├── 20260104000000_create_server_members.sql
│   ├── 20260105000000_create_channels.sql
│   ├── 20260106000000_create_channel_keys.sql
│   └── 20260107000000_create_messages.sql
├── frontend/               # React + TypeScript (reutilitzable)
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       ├── components/
│       ├── hooks/
│       ├── lib/
│       └── pages/
└── docker-compose.yml      # PostgreSQL + LiveKit + Server
```

## Passos de Desenvolupament

### FASE 1: Infraestructura Base (Setmanes 1-2)

**Objectiu**: Servidor Rust compilat amb autenticació bàsica.

#### 1.1 Configurar el Workspace

```bash
# Crear l'estructura
mkdir chillgroup/{server,shared,migrations,frontend}
cd chillgroup

# Workspace Cargo.toml
cat > Cargo.toml << 'EOF'
[workspace]
resolver = "2"
members = ["server", "shared"]

[workspace.dependencies]
axum = { version = "0.8", features = ["ws", "macros"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio-rustls", "postgres", "sqlite", "uuid", "time", "json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
dotenvy = "0.15"
jsonwebtoken = "9"
argon2 = "0.5"
base64 = "0.22"
rand = "0.8"
thiserror = "2"
async-trait = "0.1"
tower-http = { version = "0.6", features = ["cors"] }
deadpool-sqlx = "0.9"
x25519-dilithium = "2.1"
aes-gcm = { version = "0.10", features = ["std"] }
hkdf = "0.12"
sha2 = "0.10"
hmac = "0.12"
livekit-server-sdk = "0.3"
EOF
```

#### 1.2 Configurar el Servidor

```rust
// server/src/main.rs
use axum::{Router, routing::{get, post}};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use std::sync::Arc;

mod config;
mod db;
mod routes;
mod services;
mod repositories;
mod models;
mod crypto;
mod middleware;
mod error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Inicialitzar tracing
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::init_with_global(subscriber)?;

    // Carregar configuració
    let config = config::Config::from_env()?;
    tracing::info!("Configuració carregada");

    // Connectar base de dades
    let db = db::Database::new(&config.database_url, config.database_type.clone()).await?;
    db.run_migrations().await?;
    tracing::info!("Base de dades connectada");

    // Crear router
    let app_state = AppState {
        db: Arc::new(db),
        config: Arc::new(config),
    };

    let app = Router::new()
        .route("/api/auth/register", post(routes::auth::register))
        .route("/api/auth/login", post(routes::auth::login))
        .route("/health", get(|| async { "OK" }))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // Executar TTL cleanup en background
    let cleanup = services::TtlCleanupService::new(
        Arc::clone(&app_state.db),
        app_state.config.ttl_cleanup_interval,
    );
    tokio::spawn(cleanup.run());

    // Iniciar servidor
    let addr = format!("{}:{}", app_state.config.server_host, app_state.config.server_port);
    tracing::info!("Servidor escoltant a {}", addr);

    axum::Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
```

#### 1.3 Taula Users + Registració

```rust
// server/src/routes/auth.rs
use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use argon2::{Argon2, PasswordHasher, PasswordHash, PasswordVerifier};
use crate::db::AppState;
use crate::error::AppError;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user_id: Uuid,
    pub username: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    // Verificar que el username no existeix
    if state.db.user_exists(&req.username).await? {
        return Err(AppError::Conflict("Username already exists".into()));
    }

    // Generar hash de password amb argon2
    let salt = argon2::SaltString::generate(&mut rand::rngs::OsRng);
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::0x13,
        argon2::Params::new(
            argon2::memory::MemorySize::new(65536).unwrap(),
            2, 2, None
        ).unwrap()
    );
    let hash = argon2.hash_password(req.password.as_bytes(), &salt)?.to_string();

    // Insertar usuari
    let user_id = Uuid::new_v4();
    state.db.create_user(
        user_id,
        &req.username,
        &hash,
    ).await?;

    // Generar token JWT
    let token = crate::middleware::auth::generate_token(user_id, &state.config)?;

    Ok((StatusCode::CREATED, Json(AuthResponse {
        token,
        user_id,
        username: req.username,
    })))
}
```

### FASE 2: Servers i Canals (Setmana 3)

**Objectiu**: CRUD de servidors i canals.

```rust
// server/src/routes/servers.rs

// POST /api/servers
pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateServerRequest>,
    claims: Claims,
) -> Result<(StatusCode, Json<ServerResponse>), AppError> {
    let server_id = Uuid::new_v4();

    // Crear servidor
    let server = state.db.servers.create(
        server_id,
        &req.name,
        req.icon_url.as_deref(),
        claims.user_id,
    ).await?;

    // Afegir owner com a membre amb rol "owner"
    state.db.server_members.insert(
        server_id,
        claims.user_id,
        "owner",
    ).await?;

    Ok((StatusCode::CREATED, Json(server.into())))
}

// GET /api/servers
pub async fn list(
    State(state): State<AppState>,
    claims: Claims,
) -> Result<Json<Vec<ServerResponse>>>, AppError> {
    let members = state.db.server_members.list_for_user(claims.user_id).await?;
    let servers = state.db.servers.list_by_member_ids(&members).await?;

    Ok(Json(servers.into_iter().map(Into::into).collect()))
}
```

### FASE 3: Missatgeria (Setmanes 4-5)

**Objectiu**: Enviar, rebre i historial de missatges.

```rust
// server/src/routes/messages.rs

// POST /api/channels/:id/messages
pub async fn send(
    State(state): State<AppState>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
    claims: Claims,
) -> Result<(StatusCode, Json<MessageResponse>), AppError> {
    // Verificar accés al canal
    if !state.db.channels.has_access(channel_id, claims.user_id).await? {
        return Err(AppError::Forbidden("No access to this channel".into()));
    }

    // Encriptar missatge si el canal té xifratge
    let (encrypted_payload, iv) = match state.db.channels.get_encryption_type(channel_id).await? {
        EncryptionType::None => (req.text.clone(), String::new()),
        EncryptionType::Symmetric => {
            let channel_key = state.db.channel_keys.get_symmetric_key(channel_id, claims.device_id).await?;
            let payload = crypto::aes_gcm::encrypt(&channel_key, &req.text)?;
            (payload.data, payload.iv)
        }
        EncryptionType::Asymmetric => {
            let channel_key = state.db.channel_keys.get_asymmetric_key(channel_id, claims.device_id).await?;
            let payload = crypto::aes_gcm::encrypt(&channel_key, &req.text)?;
            (payload.data, payload.iv)
        }
    };

    // Guardar a DB
    let msg_id = Uuid::new_v4();
    let message = state.db.messages.create(
        msg_id,
        channel_id,
        claims.user_id,
        claims.device_id,
        &encrypted_payload,
        &iv,
        req.expires_at,
    ).await?;

    // Emitir via WebSocket
    state.ws.broadcast(
        format!("channel:{}", channel_id),
        serde_json::to_string(&MessageResponse::from(message)).unwrap(),
    ).await;

    Ok((StatusCode::CREATED, Json(MessageResponse::from(message))))
}

// GET /api/channels/:id/messages
pub async fn list(
    State(state): State<AppState>,
    Path(channel_id): Path<Uuid>,
    QueryParams(params): QueryParams,
    claims: Claims,
) -> Result<Json<Vec<MessageResponse>>, AppError> {
    let limit = params.limit.unwrap_or(50) as i32;
    let before = params.before;

    let messages = state.db.messages.get_by_channel(channel_id, limit as usize, before).await?;
    Ok(Json(messages.into_iter().map(MessageResponse::from).collect()))
}
```

### FASE 4: Criptografia Asimètrica (Setmanes 6-7)

**Objectiu**: Implementar Kyber-1024 KEM i gestió de claus de canal.

```rust
// server/src/crypto/kyber.rs

use x25519_dilithium::{KeyPair, EncapsulatingKey, DecapsulatingKey};
use crate::crypto::aes_gcm;

/// Generar un parell de claus Kyber-1024
pub fn generate_keypair() -> (Vec<u8>, KeyPair) {
    let keypair = KeyPair::generate(&mut rand::rngs::OsRng);
    let encapsulating: EncapsulatingKey = keypair.encapsulating_key();

    // Public key: 1568 bytes (es guarda al servidor)
    let public_key_bytes: Vec<u8> = (&encapsulating).into();

    (public_key_bytes, keypair)
}

/// Encapsular una clau compartida amb una clau pública
pub fn encapsulate(
    public_key: &[u8],
    channel_key: &[u8; 32],
) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    let encapsulating: EncapsulatingKey = public_key.try_into()?;
    let (shared_secret, ciphertext) = encapsulating.encapsulate(&mut rand::rngs::OsRng)?;

    // Derivar KEK del shared secret
    let kek = derive_kek(&shared_secret);

    // Encriptar channel key amb KEK
    let encrypted_key = aes_gcm::encrypt(&kek, channel_key)?;

    Ok((encrypted_key.data, ciphertext))
}

/// Desencapsular una clau compartida amb una clau privada
pub fn decapsulate(
    keypair: &KeyPair,
    ciphertext: &[u8],
    encrypted_key: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let decapsulating: DecapsulatingKey = keypair.decapsulating_key();
    let shared_secret = decapsulating.decapsulate(ciphertext)?;

    // Derivar KEK
    let kek = derive_kek(&shared_secret);

    // Desencriptar channel key
    aes_gcm::decrypt(&kek, encrypted_key)
}

fn derive_kek(shared_secret: &[u8]) -> [u8; 32] {
    use hkdf::Hkdf;
    use hmac::Hmac;
    use sha2::Sha256;

    type HmacSha256 = Hkdf<Hmac<Sha256>>;
    let hkdf = HmacSha256::new(b"chillgroup-channel-key");
    let mut kek = [0u8; 32];
    hkdf.expand(b"channel-key-derive", &mut kek)
        .expect("HKDF expand failed");
    kek
}
```

### FASE 5: LiveKit Integration (Setmana 8)

**Objectiu**: Connexió amb LiveKit per a àudio/vídeo amb E2EE.

```rust
// server/src/routes/livekit.rs

use livekit_server_sdk as livekit;

pub async fn generate_token(
    State(state): State<AppState>,
    Json(req): Json<GenerateTokenRequest>,
    claims: Claims,
) -> Result<Json<LiveKitTokenResponse>, AppError> {
    // Obtenir canal
    let channel = state.db.channels.get(req.channel_id).await?;
    let server = state.db.servers.get(channel.server_id).await?;

    // Verificar que l'usuari pertany al servidor
    if !state.db.server_members.is_member(server.id, claims.user_id).await? {
        return Err(AppError::Forbidden("Not a member of this server".into()));
    }

    // Construir room name
    let room_name = format!("chill-{}-{}", server.id, channel.id);

    // Crear token LiveKit
    let token = livekit::Token::new(
        &state.config.livekit_api_key,
        &room_name,
    )
    .set_participant_identity(claims.user_id.to_string())
    .set_name(claims.username)
    .set_can_publish(true)
    .set_can_subscribe(true)
    .set_can_publish_data(true)
    .set_can_subscribe_data(true)
    .set_hidden(false)
    .sign(&state.config.livekit_api_key, &state.config.livekit_api_secret)?;

    Ok(Json(LiveKitTokenResponse {
        token: token.claims.jwt,
        room: room_name,
        livekit_host: state.config.livekit_host.clone(),
        e2ee_enabled: channel.encryption_type == EncryptionType::Asymmetric
            || channel.encryption_type == EncryptionType::Symmetric,
    }))
}
```

### FASE 6: Frontend (Setmanes 9-11)

**Objectiu**: Refactoritzar el frontend actual per funcionar amb el nou servidor Rust.

#### Canvis principals al frontend:

```
frontend/
├── package.json
├── vite.config.ts
├── src/
│   ├── lib/
│   │   ├── api.ts            # API client (Axum backend)
│   │   ├── websocket.ts      # Socket.IO client
│   │   ├── livekit.ts        # LiveKit integration
│   │   └── crypto.ts         # E2EE client-side crypto
│   │
│   ├── components/
│   │   ├── ChannelList.tsx   # Llista de canals
│   │   ├── ChannelViewer.tsx # Visualitzador de canal
│   │   ├── MessageList.tsx   # Llista de missatges
│   │   ├── MessageInput.tsx  # Input de missatges
│   │   ├── VoiceRoom.tsx     # Sala de veu (LiveKit)
│   │   ├── InviteModal.tsx   # Modal d'invitació
│   │   └── EncryptionSettings.tsx  # Configuració criptografia
│   │
│   ├── hooks/
│   │   ├── useAuth.ts        # Auth state
│   │   ├── useServer.ts      # Server data
│   │   ├── useChannel.ts     # Channel data
│   │   ├── useMessages.ts    # Messages
│   │   └── useChannelKey.ts  # Channel key management (E2EE)
│   │
│   └── pages/
│       ├── Login.tsx
│       ├── Register.tsx
│       ├── ServerPage.tsx
│       └── ChannelPage.tsx
```

#### Crypto Client-Side

```typescript
// frontend/src/lib/crypto.ts

import { MLKEM1024 } from '@noble/post-quantum'

// Generar parell de claus Kyber-1024 per al dispositiu
export async function generateKyberKeyPair(): Promise<{
    publicKey: string
    secretKey: Uint8Array
}> {
    const keypair = await MLKEM1024.keygen()
    const publicKey = btoa(String.fromCharCode(...keypair.publicKey))

    // Guardar secretKey a IndexedDB
    await storeSecretKey(keypair.secretKey)

    return { publicKey, secretKey: keypair.secretKey }
}

// Encriptar missatge amb AES-GCM
export async function encryptMessage(
    channelKey: CryptoKey,
    text: string
): Promise<{ encrypted: string; iv: string }> {
    const encoder = new TextEncoder()
    const iv = crypto.getRandomValues(new Uint8Array(12))
    const encrypted = await crypto.subtle.encrypt(
        { name: 'AES-GCM', iv },
        channelKey,
        encoder.encode(text)
    )

    return {
        encrypted: btoa(String.fromCharCode(...new Uint8Array(encrypted))),
        iv: btoa(String.fromCharCode(...iv)),
    }
}

// Desencriptar missatge amb AES-GCM
export async function decryptMessage(
    channelKey: CryptoKey,
    encrypted: string,
    iv: string
): Promise<string> {
    const encryptedBytes = new Uint8Array(atob(encrypted).split('').map(c => c.charCodeAt(0)))
    const ivBytes = new Uint8Array(atob(iv).split('').map(c => c.charCodeAt(0)))

    const decrypted = await crypto.subtle.decrypt(
        { name: 'AES-GCM', iv: ivBytes },
        channelKey,
        encryptedBytes
    )

    return new TextDecoder().decode(decrypted)
}

// KEM Encapsulate (xifrar channelKey)
export async function kemEncapsulate(
    publicKeyBase64: string,
    channelKey: Uint8Array
): Promise<{
    encryptedKey: string
    ciphertext: string
}> {
    const publicKey = new Uint8Array(atob(publicKeyBase64).split('').map(c => c.charCodeAt(0)))
    const { sharedSecret, ciphertext } = await MLKEM1024.encapsulate(publicKey)

    // Derivar KEK
    const kekBuffer = await crypto.subtle.digest(
        'SHA-256',
        sharedSecret
    )
    const kek = await crypto.subtle.importKey(
        'raw',
        kekBuffer,
        { name: 'AES-GCM' },
        false,
        ['encrypt']
    )

    // Encriptar channelKey
    const encryptedKey = await crypto.subtle.encrypt(
        { name: 'AES-GCM' },
        kek,
        channelKey
    )

    return {
        encryptedKey: btoa(String.fromCharCode(...new Uint8Array(encryptedKey))),
        ciphertext: btoa(String.fromCharCode(...ciphertext)),
    }
}

// KEM Decapsulate (desxifrar channelKey)
export async function kemDecapsulate(
    secretKey: Uint8Array,
    ciphertextBase64: string,
    encryptedKeyBase64: string
): Promise<Uint8Array> {
    const ciphertext = new Uint8Array(atob(ciphertextBase64).split('').map(c => c.charCodeAt(0)))

    const sharedSecret = await MLKEM1024.decapsulate(secretKey, ciphertext)

    // Derivar KEK
    const kekBuffer = await crypto.subtle.digest(
        'SHA-256',
        sharedSecret
    )
    const kek = await crypto.subtle.importKey(
        'raw',
        kekBuffer,
        { name: 'AES-GCM' },
        false,
        ['decrypt']
    )

    // Desencriptar channelKey
    const encryptedKey = new Uint8Array(atob(encryptedKeyBase64).split('').map(c => c.charCodeAt(0)))
    const decrypted = await crypto.subtle.decrypt(
        { name: 'AES-GCM' },
        kek,
        encryptedKey
    )

    return new Uint8Array(decrypted)
}
```

### FASE 7: Tests i Poliment (Setmana 12)

#### Tests d'Integració

```rust
// server/tests/integration.rs
use axum::{body::Body, http::Request, Router};
use axum::body::Body as AxumBody;
use axum::http::StatusCode;
use axum_test::Test;
use sqlx::{SqlitePool, Row};
use uuid::Uuid;

#[tokio::test]
async fn test_register_and_login() {
    // Setup
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let app = create_test_app(pool);

    // Register
    let resp = Test::new(app.clone())
        .post("/api/auth/register")
        .json(&serde_json::json!({
            "username": "testuser",
            "password": "testpass123"
        }))
        .await;

    assert_eq!(resp.status(), StatusCode::CREATED);

    // Login
    let login_resp = Test::new(app)
        .post("/api/auth/login")
        .json(&serde_json::json!({
            "username": "testuser",
            "password": "testpass123"
        }))
        .await;

    assert_eq!(login_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_symmetric_channel_encryption() {
    // Test: registrar -> crear canal simètric -> enviar missatge xifrat -> rebre
}

#[tokio::test]
async fn test_asymmetric_channel_encryption() {
    // Test: registrar 2 usuaris -> crear canal asimètric -> convidar -> xifrar/desxifrar
}
```

#### Tests E2E Frontend

```typescript
// frontend/e2e/channel-encryption.spec.ts
import { test, expect } from '@playwright/test'

test('can send encrypted message in asymmetric channel', async ({ page }) => {
    // Crear dos usuaris
    // Crear canal asimètric
    // Convidar segon usuari
    // Enviar missatge xifrat
    // Verificar que es pot desxifrar
})
```

## Docker Compose

```yaml
# docker-compose.yml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_DB: chillgroup
      POSTGRES_USER: chillgroup
      POSTGRES_PASSWORD: chillgroup
    volumes:
      - pgdata:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  livekit:
    image: livekit/livekit-server:latest
    ports:
      - "7880:7880"
      - "7881:7881"
      - "10000-20000/udp:10000-20000/udp"
    command: >
      --bind 0.0.0.0
      --room {
        "enabled": true,
        "names": ["chill-*"]
      }
      --keys
        {
          "chillgroup-key": "chillgroup-secret"
        }
    environment:
      LIVEKIT_KEYS: chillgroup-key=chillgroup-secret
    volumes:
      - ./livekit.yaml:/livekit-server.yaml

volumes:
  pgdata:
```

## Com Executar

### Desenvolupament

```bash
# 1. Base de dades
docker compose up -d postgres livekit

# 2. Executar migracions
cd server && sqlx migrate run && cd ..

# 3. Servidor Rust
cd server
cargo run
# → Servidor escoltant a 0.0.0.0:8080

# 4. Frontend (altre terminal)
cd frontend
npm run dev
# → Frontend a localhost:5173
```

### Producció

```bash
# Build
cd server
cargo build --release

# Deploy
docker compose -f docker-compose.prod.yml up -d
```

## Mètriques i Health Check

```rust
// server/src/routes/health.rs

#[derive(serde::Serialize)]
struct HealthResponse {
    status: String,
    database: String,
    livekit: String,
    uptime_seconds: u64,
}

pub async fn health(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, AppError> {
    let db_status = if state.db.ping().await.is_ok() {
        "ok"
    } else {
        "error"
    };

    let livekit_status = if state.db.ping().await.is_ok() {
        "ok"  // Verificar livekit en el futur
    } else {
        "error"
    };

    Ok(Json(HealthResponse {
        status: if db_status == "ok" && livekit_status == "ok" {
            "healthy"
        } else {
            "degraded"
        }.to_string(),
        database: db_status.into(),
        livekit: livekit_status.into(),
        uptime_seconds: start_time.elapsed().as_secs(),
    }))
}
```

## Resum de Mètriques Objectiu

| Mètrica | Objectiu |
|---------|----------|
| Request latency p50 | < 50ms |
| Request latency p99 | < 200ms |
| WebSocket connect/disconnect | < 100ms |
| Missatge latency (local) | < 10ms |
| Missatge latency (remote) | < 50ms |
| Concurrent WebSocket connections | 10,000+ |
| Concurrent LiveKit participants | 50/channel |

## Future Improvements

### Environment Variable-based Debug Logging

Actualment els logs es controlen via `tracing` (backend) i `console` (frontend) sense nivells configurables. Per millorar la debuggabilitat en producció i desenvolupament, seria útil afegir:

- `BACKEND_DEBUG=info|debug|warn|error` — Controlar els nivells de logs de `tracing` (server-side) via variable d'entorn
- `FRONTEND_DEBUG=info|debug|warn|error` — Crear un logger custom per al frontend que filtri els `console.*` calls per nivell

Això evitaria comentar/descomentar logs manuals i permetria activar/desactivar logging en temps d'execució sense recompilacions.

**Implementació sugerida:**
- **Backend**: Llegir `BACKEND_DEBUG` a `config.rs` i configurar dinàmicament el layer de `tracing` a `main.rs`
- **Frontend**: Crear `lib/logger.ts` que wrappegi `console.log/warn/error` i filtri per nivell

---

## Sistema d'Administració — Configuració

### Setup Inicial

Quan desplegar ChillGroup en **mode restringit** (`OPEN_REGISTER=false`), cal configurar les variables d'entorn:

```bash
# .env (o variables del sistema)

# Mode de registre
OPEN_REGISTER=false                  # Desactiva el registre públic

# Credencials d'administrador
ADMIN_USER=admin                     # Username inicial del administrador
ADMIN_PASSWORD=SuperSecurePass123    # Contrasenya inicial (recomanació: generar amb `openssl rand -base64 32`)
```

### Inicialització Automàtica

Al llançar el servidor amb `OPEN_REGISTER=false`, el sistema ha de:

1. Verificar si l'usuari `ADMIN_USER` ja existeix a la BD
2. Si NO existeix: crear-lo automàticament amb role `admin`
3. Si SÍ existeix: continuar sense fer res (idempotent)

**Código Rust** (pseudocodi a `config.rs` o `main.rs`):

```rust
// server/src/main.rs
async fn init_admin_user(db: &Database, config: &Config) -> Result<()> {
    if !config.open_register {
        let existing = db.get_user_by_username(&config.admin_user).await?;
        
        if existing.is_none() {
            tracing::info!("Creant administrador inicial: {}", config.admin_user);
            let password_hash = hash_password(&config.admin_password)?;
            db.create_user(&config.admin_user, &password_hash, "admin").await?;
            tracing::info!("Administrador inicial creat");
        }
    }
    Ok(())
}
```

### Setup per a Desenvolupament

Per a provar el sistema d'administració en local:

```bash
# 1. Configurar .env
echo "OPEN_REGISTER=false" >> .env
echo "ADMIN_USER=admin" >> .env
echo "ADMIN_PASSWORD=admin123" >> .env

# 2. Iniciar el servidor
cargo run --bin server

# 3. Fer login com admin
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}'

# 4. Guardar el JWT de la resposta
export JWT="eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9..."

# 5. Llistar usuaris (sols amb rol admin)
curl http://localhost:8080/api/admin/users \
  -H "Authorization: Bearer $JWT"

# 6. Crear un nou usuari
curl -X POST http://localhost:8080/api/admin/users \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{
    "username": "newuser",
    "password": "temppass123",
    "role": "user"
  }'
```

### Frontend — Pantalla de Login d'Admin

El frontend ha de detectar si `OPEN_REGISTER=false` i mostrar:

- Campo de login normal si és **registre obert**
- Només formulari de login si és **registre restringit**
- Banner informatiu: "El registre està desactivat. Si ets administrador, fes login aquí."

Configuració actual implementada:

- El valor es llegeix des del `.env` de l'arrel (`OPEN_REGISTER`) com a font única.
- `frontend/vite.config.ts` injecta aquest valor al client com a constant de compilació (`__OPEN_REGISTER__`).
- No s'ha de duplicar `OPEN_REGISTER` en un `frontend/.env` separat.

Per la connexió de veu passa el mateix:

- La URL de LiveKit també prové de l'arrel (`LIVEKIT_HOST`).
- Vite la injecta com a `__LIVEKIT_HOST__`.
- S'evita la duplicació amb `VITE_LIVEKIT_URL` al frontend.

Endpoints a cridardi frontend:

```typescript
// 1. Verificar si registre està obert (nova ruta)
GET /api/config/open-register  →  { "open_register": boolean }

// 2. Login normal
POST /api/auth/login

// 3. Si l'usuari té isAdmin=true, mostrar menú d'administrador
// Accés als endpoints /api/admin/users/*
```

### Security Best Practices

1. **Canviar la contrasenya del admin**: Al primer login, recomanar canviar la contrasenya temporal
2. **Rate limiting**: Els endpoints d'admin (`/api/admin/**`) han de tenir rate limiting més estricte (3 intents/min per IP)
3. **Auditoria**: Registrar totes les operacions d'admin a logs separats
4. **Tokens JWT**: Els tokens d'admin amb `isAdmin: true` han de ser invalidats en cas de banning
5. **2FA (futur)**: Afegir autenticació de dos factors per a comptes d'admin

---

## Sistema de Plans/Tiers — Configuració

### Inicialització Automàtica de Plans

Els plans (free, pro, enterprise) es **creen automàticament** al primer startup si no existeixen:

```rust
// server/src/main.rs
async fn init_plans(db: &Database) -> Result<()> {
    let free_plan = Plan {
        id: Uuid::parse_str("550e8400-e29b-41d4-a716-446655441001").unwrap(),
        name: "free".into(),
        display_name: "Free".into(),
        max_servers: 1,
        max_channels_text_per_server: 3,
        max_channels_voice_per_server: 2,
        max_members_per_server: 20,
        api_calls_per_minute: 60,
        messages_per_day: 10000,
        // ...
    };
    
    // Crear si no existeix
    if db.get_plan_by_name("free").await?.is_none() {
        db.create_plan(free_plan).await?;
        tracing::info!("Plan 'free' creat");
    }
}
```

### Setup per a Desenvolupament

Per a provar el sistema de plans en local:

```bash
# 1. Els plans es creen automàticament al startup

# 2. Crear un usuari (default: plan "free")
curl -X POST http://localhost:8080/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"username":"testuser","password":"pass12345"}'

# 3. Guardar el JWT
export JWT="eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9..."

# 4. Verificar el plan actual
curl http://localhost:8080/api/user/me/plan \
  -H "Authorization: Bearer $JWT"

# 5. Intentar crear 2 servidors (hauria de fallar al segon)
curl -X POST http://localhost:8080/api/servers \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"name":"Server1"}'  # ✅ OK

curl -X POST http://localhost:8080/api/servers \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"name":"Server2"}'  # ❌ 429 Too Many Requests

# 6. Com admin, canviar el plan a "pro"
curl -X PUT http://localhost:8080/api/admin/users/:userId/plan/550e8400-e29b-41d4-a716-446655441002 \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json"

# 7. Ara pot crear més servidors (fin a 5)
curl -X POST http://localhost:8080/api/servers \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"name":"Server2"}'  # ✅ OK (ja ha upgradat a pro)
```

### Endpoints de Plans per a Testing

```bash
# Llistar tots els plans
curl http://localhost:8080/api/plans

# Obtenir limít del usuari actual
curl http://localhost:8080/api/user/me/plan \
  -H "Authorization: Bearer $JWT"

# Verificar si pot crear servidor (sense crear-lo)
curl -X POST http://localhost:8080/api/user/me/check-limits \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"action":"create_server"}'

# Verificar si pot crear canal text en un servidor
curl -X POST http://localhost:8080/api/user/me/check-limits \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
  -d '{"action":"create_text_channel","serverId":"550e8400-e29b-41d4-a716-446655440010"}'
```

### Gestió de Plans (Admin)

Proposta adoptada per al menú d'administració: **"Gestió de plans"**.

Objectiu operatiu:
- Permetre a admins crear/editar/esborrar plans personalitzats sense tocar BD manualment.
- Mantenir els plans base (`free`, `pro`, `enterprise`) protegits.
- Evitar esborrat de plans en ús (cal reassignar usuaris primer).

Com provar el flux complet en local:

```bash
# 1) Llistar plans com admin
curl http://localhost:8080/api/admin/plans \
    -H "Authorization: Bearer $ADMIN_JWT"

# 2) Crear plan custom
curl -X POST http://localhost:8080/api/admin/plans \
    -H "Authorization: Bearer $ADMIN_JWT" \
    -H "Content-Type: application/json" \
    -d '{
        "name": "team_plus",
        "displayName": "Team Plus",
        "description": "Plan per equips",
        "maxServers": 8,
        "maxChannelsTextPerServer": 30,
        "maxChannelsVoicePerServer": 15,
        "maxMembersPerServer": 800,
        "apiCallsPerMinute": 1200,
        "messagesPerDay": -1
    }'

# 3) Modificar plan custom
curl -X PUT http://localhost:8080/api/admin/plans/:planId \
    -H "Authorization: Bearer $ADMIN_JWT" \
    -H "Content-Type: application/json" \
    -d '{"displayName":"Team Plus Updated","maxServers":10}'

# 4) Assignar-lo a un usuari
curl -X PUT http://localhost:8080/api/admin/users/:userId/plan/:planId \
    -H "Authorization: Bearer $ADMIN_JWT"

# 5) Intent d'esborrat mentre està en ús (ha de fallar amb 409)
curl -X DELETE http://localhost:8080/api/admin/plans/:planId \
    -H "Authorization: Bearer $ADMIN_JWT"
```

### Frontend — Mostrar Plans i Límits

El frontend ha de:

1. **Carregar plans públics**: `GET /api/plans` (al mount)
2. **Mostrar plan actual**: `GET /api/user/me/plan` (al profile/dashboard)
3. **Mostrar warnings de límits**: Si `remainingServers <= 0`, mostrar banner "Has assolit el límit. Upgrade ara."
4. **Botó de Upgrade**: Redireccionar a plan selection (futur: integrar amb Stripe)

### Comportament amb Limits

#### Hard Limit (Bloqueig)
- Crear recurs quan limit assolit → 429 Too Many Requests
- Client ha de mostrar error clara al usuari

#### Soft Limit (Avís, futur)
- Quan resta <= 1 recurs → Warning ("Aproximat-te al límit")
- Suggeriements d'upgrade

#### Downgrade (No degradation forçat)
- Si admin degrada plan (pro → free) i usuari té 3 servidors
- Els 3 servidors romanen, però no pot crear més
- "Estàs en modo pro limitat" (UI avís)

## Invitacions

### Setup i Configuració

Invitacions permeten registre de nous usuaris per codi, fins i tot quan `OPEN_REGISTER=false`.

#### Variables d'Entorn (Opcional)

```bash
# Si voleu deshabilitar registre obert:
OPEN_REGISTER=false

# Admins pots crear invitacions:
# - No necessàries env vars per a invitacions
# - Es configuren via BD i API
```

#### Generar Invitacions (Admin)

```bash
# 1. Login com admin
JWT=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin_password"}' \
  | jq -r '.data.token')

# 2. Crear invitació amb múltiples usos i servidor opcional
curl -X POST http://localhost:8080/api/invitations \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
    -d '{"max_uses": 5, "server_id": "550e8400-e29b-41d4-a716-446655440010"}'

# Response:
# {
#   "success": true,
#   "data": {
#     "invitationId": "550e8400-...",
#     "code": "ABC123-DEF456-GHI789-XYZ000",
#     "serverId": "550e8400-e29b-41d4-a716-446655440010",
#     "maxUses": 5,
#     "usesCount": 0,
#     "isActive": true,
#     "createdBy": "admin"
#   }
# }

# 3. Llistar invitacions
curl http://localhost:8080/api/invitations \
  -H "Authorization: Bearer $JWT"

# 4. Invalidar invitació
curl -X DELETE http://localhost:8080/api/invitations/550e8400-e29b-41d4-a716-446655440200 \
  -H "Authorization: Bearer $JWT"
```

### Registre amb Invitació

```bash
# Registrar nou usuari amb codi:
curl -X POST http://localhost:8080/api/auth/register-with-invitation \
  -H "Content-Type: application/json" \
  -d '{
    "code": "ABC123-DEF456-GHI789-XYZ000",
    "username": "newuser",
    "password": "secure_password"
  }'

# Response:
# {
#   "success": true,
#   "data": {
#     "userId": "550e8400-e29b-41d4-a716-446655440005",
#     "username": "newuser",
#     "token": "eyJhbGciOiJSUzI1NiIs...",
#     "deviceId": "550e8400-e29b-41d4-a716-446655440101"
#   }
# }

# Si la invitació té server_id, el nou usuari entra automàticament
# com a member al servidor vinculat.

# Error si codi invàlid/exhausted:
# {
#   "success": false,
#   "error": {
#     "code": 404 | 410,
#     "message": "Codi d'invitació no vàlid" | "Límit d'usos assolit"
#   }
# }
```

### Escenaris de Testing

#### Escenari 1: OPEN_REGISTER=true (defecte)
- Ambdós endpoints funcionen: `/api/auth/register` i `/api/auth/register-with-invitation`
- Invitacions són opcionals (feature adicional)

#### Escenari 2: OPEN_REGISTER=false (registre tancat)
- Només `/api/auth/register-with-invitation` funciona
- `/api/auth/register` retorna 403 Forbidden
- Admins generen codis d'invitació per compartir

#### Escenari 3: Múltiples usos (-1 = unlimited)
```bash
# Crear invitació unlimited:
curl -X POST http://localhost:8080/api/invitations \
  -H "Authorization: Bearer $JWT" \
  -H "Content-Type: application/json" \
    -d '{"max_uses": -1}'

# Es pot usar indefinides vegades
```

#### Escenari 4: Invitació exhausted
```bash
# Crear amb maxUses=1
curl -X POST http://localhost:8080/api/invitations \
  -H "Authorization: Bearer $JWT" \
    -d '{"max_uses": 1}'
# → code: "ABC123-..."

# Primer use: SUCCESS
curl -X POST http://localhost:8080/api/auth/register-with-invitation \
  -d '{"code":"ABC123-...","username":"user1","password":"pwd"}'
# → 201 Created

# Segon use: FAILED
curl -X POST http://localhost:8080/api/auth/register-with-invitation \
  -d '{"code":"ABC123-...","username":"user2","password":"pwd"}'
# → 410 Gone ("Límit d'usos assolit")
```

### Frontend — Registre amb Invitació

Si `OPEN_REGISTER=false`:
1. **Mostrar**: "Necessites una invitació per registrar-te"
2. **Input**: Camp per codi d'invitació
3. **Flow**:
   - User introdueix codi + username + password
   - Frontend POST `/api/auth/register-with-invitation`
   - Si 404: "Codi invàlid"
   - Si 410: "Invitació exhausted"
   - Si 201: Redirigir a chat dashboard

