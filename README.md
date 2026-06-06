# ChillGroup v2

Missatgeria segura amb encriptació End-to-End (E2EE).

## 📋 Tecnologia

- **Backend**: Rust + Axum + SQLx
- **Frontend**: React + TypeScript + Vite
- **Base de dades**: PostgreSQL o SQLite
- **Vei**: LiveKit
- **Tests**: Vitest + Playwright (frontend), cargo test (backend)

## 🏗️ Estructura del projecte

```
QuantumTeam/
├── frontend/         # Client React (Vite + TypeScript)
│   ├── src/
│   │   ├── components/   # Components UI
│   │   ├── contexts/     # React Context (Auth)
│   │   ├── lib/          # API, crypto, storage
│   │   ├── styles/       # CSS variables
│   │   └── types/        # Types TypeScript
│   ├── tests/e2e/        # Tests E2E (Playwright)
│   └── package.json
├── server/           # Servidor Rust (Axum)
│   ├── src/
│   │   ├── routes/       # Rutes API (auth, servers, channels...)
│   │   ├── crypto/       # AES-GCM-256, Kyber-1024, hashing
│   │   ├── middleware/   # Autenticació JWT
│   │   ├── models/       # Models DB
│   │   └── main.rs
│   ├── migrations/       # SQL migrations (PostgreSQL)
│   └── Cargo.toml
├── shared/           # Codi compartit backend/frontend
│   └── src/types.rs
└── definitions/      # Documentació de disseny
```

## 🚀 Com arrancar

### 1. Prerequisits

- **Rust**: `rustup install stable`
- **Node.js 20+**: `nvm install 20`
- **Docker i Docker Compose**: per arrencar tota la pila amb una sola comanda
- **PostgreSQL 16+** (opcional): `sudo apt install postgresql`
- **SQLite** (per defecte): `sudo apt install sqlite3`

### 2. Configurar variables d'entorn

Crear fitxer `.env` a l'arrel del projecte:

```env
# Servidor
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# Base de dades (PostgreSQL o SQLite)
DATABASE_URL=sqlite://chillgroup.db?mode=rwc
# o DATABASE_URL=postgres://user:pass@localhost:5432/chillgroup

# Si uses SQLite, el backend afegeix `?mode=rwc` automàticament si no el poses.

# LiveKit (per a veu)
LIVEKIT_HOST=http://localhost:7880
LIVEKIT_API_KEY=devkey
LIVEKIT_API_SECRET=secret

# JWT
JWT_SECRET=el-teu-secret-aqui-canvia-m aixxo
JWT_EXPIRATION_DAYS=7

# Registre
OPEN_REGISTER=true
# Si OPEN_REGISTER=false, cal ADMIN_USER i ADMIN_PASSWORD
# ADMIN_USER=admin
# ADMIN_PASSWORD=SuperSecurePass123

# Codi one-shot opcional per promoure el següent registre a admin
# ONE_ADMIN_INVITATION=CODI-UNIC-ADMIN
```

### 3. Arrencada amb Docker

El projecte té dos fitxers Compose:

| Fitxer | Ús |
|--------|----|
| `docker-compose.yml` | **Producció** — usa la imatge precompilada de GitHub Container Registry |
| `docker-compose.dev.yml` | **Desenvolupament** — frontend Vite + backend `cargo watch` amb hot-reload |

**Mode producció — wizard de desplegament:**

La imatge es publica com a **manifest multi-arch** (`linux/amd64` + `linux/arm64`): Docker selecciona automàticament l'arquitectura correcta del host en fer `docker compose pull`. Funciona en servidors x86_64 i màquines ARM (Raspberry Pi, AWS Graviton…) sense cap canvi al `docker-compose.yml`.

El projecte inclou `setup-deploy.sh`, un wizard interactiu que genera el `docker-compose.yml` i el `.env.compose` adaptats a la teva infraestructura (base de dades, LiveKit, S3, secrets…).

```bash
# Sense clonar el repositori:
curl -fsSL https://raw.githubusercontent.com/agustim/chillgroup2/main/setup-deploy.sh -o setup-deploy.sh
bash setup-deploy.sh

# O amb el repositori clonat:
./setup-deploy.sh
```

El wizard pregunta:
- **BD**: PostgreSQL local · PostgreSQL extern · SQLite
- **LiveKit**: container local · servidor remot
- **S3**: RustFS local · S3 extern (AWS, R2, MinIO…)
- **App**: port, logs, registre obert/tancat, credencials admin, `ONE_ADMIN_INVITATION`, secrets JWT i master key

Després d'executar el wizard:

```bash
cd deploy
docker compose pull
docker compose up -d
```

Guia completa: [docs/ca/deploy-docker](docs-site/docs/ca/deploy-docker.md)

---

**Mode desenvolupament (recomanat per contribuir):**

```bash
docker compose -f docker-compose.dev.yml up
```

Aixeca PostgreSQL, LiveKit, RustFS, el backend amb `cargo watch` (port 8080) i el frontend amb `pnpm dev` (port 5173). La primera arrencada compila `cargo-watch`; les següents reutilitzen la caché del volum.

Config d'entorn per a dev (recomanat per equip):

1. Fitxer base compartit: `.env.compose`
2. Overrides locals (no versionats): `.env.compose.local`
3. Plantilla de referència: `.env.compose.example`

`docker-compose.dev.yml` carrega primer `.env.compose` i després `.env.compose.local`. Si una variable existeix als dos fitxers, preval `.env.compose.local`.

Configuració recomanada per adjunts S3 en local:

- `S3_ENDPOINT=http://rustfs:9000` (endpoint intern que veu el contenidor `app`)
- `S3_PUBLIC_ENDPOINT=http://localhost:9000` (endpoint públic que veu el navegador)
- `SERVER_PROXY_S3=false` (per defecte: frontend puja/baixa directament contra RustFS amb URL signada)

Mode alternatiu de proxy per backend:

- `SERVER_PROXY_S3=true`: el frontend puja/baixa fitxers passant pel backend, i el backend reenvia a RustFS. Útil quan el navegador no pot resoldre l'host S3 o hi ha restriccions CORS.

| Escenari | Valor recomanat |
|----------|-----------------|
| Browser veu `localhost:9000` | `false` |
| Browser no resol l'host S3 | `true` |
| Restriccions CORS/xarxa | `true` |
| Màxim rendiment de transferència | `false` |
| Auditoria centralitzada de pujades | `true` |

### 4. Arrancar el backend manualment

```bash
cd server
cargo run
```

Per defecte el servidor carrega `.env` del directori actual.
També pots indicar ruta de directori o fitxer:

```bash
cargo run -- -c /etc/chillgroup
# o
cargo run -- --config /etc/chillgroup/.env
```

Per generar només un `.env` d'exemple (sense arrencar el servidor):

```bash
cargo run -- --generate-env-example
# o amb ruta explícita
cargo run -- --generate-env-example /etc/chillgroup/.env
# si el fitxer ja existeix
cargo run -- --generate-env-example /etc/chillgroup/.env --force
```

Nota: si el fitxer de sortida ja existeix, la comanda falla per defecte.
Per sobreescriure, cal `-f` o `--force`.

El servidor escoltarà a `http://localhost:8080`.

### 4.1 Bootstrap admin one-shot (opcional)

Si necessites promoure un únic registre a admin sense SQL manual:

```bash
echo "ONE_ADMIN_INVITATION=CODI-UNIC-ADMIN" >> .env
```

Després, registra el nou usuari enviant `admin_invitation_code` al registre.
El codi només funciona una vegada.

### 5. Arrancar el frontend

```bash
cd frontend
pnpm install
pnpm dev
```

El frontend estarà a `http://localhost:5173`.

Nota sobre variables d'entorn del frontend: amb Vite es resolen en temps de compilació (build-time), no en temps d'execució del binari.

## 📊 Base de dades

### Suport

| Base de dades | Estat | Notes |
|---------------|-------|-------|
| **PostgreSQL 16+** | ✅ Complet | Requereix migrations |
| **SQLite** | ✅ Automàtic | Taules creades automàticament |

### Configuració PostgreSQL

```env
DATABASE_URL=postgres://chillgroup:pass@localhost:5432/chillgroup
```

Aplicar migrations:
```bash
cd server
cargo install sqlx-cli
sqlx database create
sqlx migrate run
```

### Configuració SQLite (per defecte)

```env
DATABASE_URL=sqlite://chillgroup.db?mode=rwc
```

Si no afegeixes `?mode=rwc`, el backend el normalitza automàticament abans de connectar.
Les taules es creen automàticament en iniciar el servidor.

## 🧪 Tests

### Frontend

```bash
cd frontend

# Tests unitaris (95 tests passant)
pnpm test:run

# Tests E2E (5 tests passant)
pnpm test:e2e

# Build
pnpm build
```

### Backend

```bash
cd server

# Tests
cargo test

# Build
cargo build
```

## 📡 API endpoints

### Autenticació

| Mètodo | Endpoint | Descripció |
|--------|----------|------------|
| POST | `/api/auth/register` | Registrar nou usuari |
| POST | `/api/auth/login` | Iniciar sessió |
| POST | `/api/auth/refresh` | Renovar token JWT |

### Servidors

| Mètodo | Endpoint | Descripció |
|--------|----------|------------|
| GET | `/api/servers` | Llista servidors |
| POST | `/api/servers` | Crear servidor |
| GET | `/api/servers/{id}` | Info servidor |
| DELETE | `/api/servers/{id}` | Eliminar servidor |

### Missatges

| Mètodo | Endpoint | Descripció |
|--------|----------|------------|
| GET | `/api/channels/{id}/messages` | Llista missatges |
| POST | `/api/channels/{id}/messages` | Enviar missatge |
| PUT | `/api/messages/{id}` | Editar missatge |
| DELETE | `/api/messages/{id}` | Eliminar missatge |

## 🔒 Seguretat

- **Hashing de contrasenya**: Argon2
- **Missatges encriptats**: AES-GCM-256
- **Intercanvi de claus**: Kyber-1024 (NIST Level 5)
- **Autenticació**: JWT amb HS256
- **E2EE**: Només els participants poden desxifrar missatges
- **Protecció local de claus**: les claus de canal es guarden xifrades en repòs a IndexedDB (vault local del client)
- **Desbloqueig de dispositiu**: després de l'inici de sessió, el client pot requerir clau local de desbloqueig
- **Logout flexible**: backup (xifrat opcional) i neteja local són opcions independents

## 📝 Desenvolupament

### Afegir una nova taula (SQLite)

Afegir query a `server/src/db.rs::create_tables_sqlite()`:

```rust
r#"
CREATE TABLE IF NOT EXISTS nova_taula (
    id TEXT PRIMARY KEY,
    nom TEXT NOT NULL
)
"#,
```

### Afegir una nova ruta API

1. Crear fitxer a `server/src/routes/nova_ruta.rs`
2. Afegir a `server/src/routes/mod.rs`
3. Registrar a `server/src/main.rs`