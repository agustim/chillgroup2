# ChillGroup — Development Guide

## Changes vs v1

| Aspect | v1 | v2 |
|--------|----|----|
| Server | Node.js + TypeScript | Rust |
| Server framework | Express.js | Axum |
| Real-time | Socket.IO (Node) | Socket.IO (Rust) / Axum WS |
| Audio/Video | LiveKit (manual E2EE) | LiveKit (native E2EE) |
| Cryptography | E2EE only (Kyber) | 3 levels (none/symmetric/asymmetric) |
| Database | PostgreSQL + Durable Objects + SQLite | PostgreSQL + SQLite (via SQLx) |
| Project | Node monorepo | Cargo workspace |

## Workspace Structure

```
chillgroup/
├── Cargo.toml              # Workspace root
├── server/                 # Rust server
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
├── shared/                 # Shared client-server crate
│   ├── Cargo.toml
│   └── src/
│       ├── types.rs
│       └── constants.rs
├── migrations/             # SQLx migrations
├── frontend/               # React + TypeScript
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       ├── components/
│       ├── hooks/
│       ├── lib/
│       └── pages/
└── docker-compose.yml      # PostgreSQL + LiveKit + Dockerfile
```

## Production Build

The project has a `build.sh` at the root with mode and target support:

```bash
./build.sh --mode external
./build.sh --mode embedded
./build.sh --mode embedded --target x86_64-unknown-linux-gnu
./build.sh --mode embedded --target aarch64-unknown-linux-gnu
```

Mode details:

- `external`: compiles the frontend and copies `frontend/dist` to `target/<target>/release/static`.
- `embedded`: compiles the frontend and embeds it inside the Rust binary via the `embedded-assets` feature.

### Frontend inside the binary

When compiled with `--features embedded-assets`, the server serves frontend assets from memory (SPA fallback to `index.html`).

When the feature is not enabled, the server looks for a static directory (`STATIC_DIR` or `./static`).

## Quick Start (Docker)

```bash
# Copy the example env file and fill in the values
cp .env.example .env

# Start all services
docker compose up -d

# The server is now available at http://localhost:3000
```

## Development Setup (without Docker)

### Requirements

- Rust 1.75+
- Node.js 20+ and pnpm
- PostgreSQL 16 (or use the Docker Compose service)

### Steps

```bash
# Install Rust dependencies and build
cargo build

# Install frontend dependencies
cd frontend && pnpm install

# Run database migrations
cargo sqlx migrate run

# Start backend in development mode
cargo run

# Start frontend dev server (in a separate terminal)
cd frontend && pnpm dev
```

## Environment Variables

Key variables for the backend (`.env` file):

| Variable | Description | Example |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://user:pass@localhost/chillgroup` |
| `JWT_SECRET` | Secret for signing JWT tokens | 64+ char random string |
| `OPEN_REGISTER` | Allow public registration | `true` / `false` |
| `ADMIN_USER` | Admin username (restricted mode) | `admin` |
| `ADMIN_PASSWORD` | Admin password (restricted mode) | strong password |
| `LIVEKIT_URL` | LiveKit server URL | `wss://livekit.example.com` |
| `LIVEKIT_API_KEY` | LiveKit API key | from LiveKit dashboard |
| `LIVEKIT_API_SECRET` | LiveKit API secret | from LiveKit dashboard |

## Contributing

See the [Contributing guide](/en/contributing) for PR workflow and conventions.
