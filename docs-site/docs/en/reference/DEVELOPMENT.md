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

## Desktop Client

ChillGroup ships a native desktop client for **Linux** (Electron) and for **macOS and Windows** (Tauri v2).

### Linux — Available formats

Each release produces four executables for Linux x64:

| Format | File | Distributions | Install |
|--------|------|---------------|---------|
| AppImage | `ChillGroup-*.AppImage` | All (no install needed) | `chmod +x *.AppImage && ./*.AppImage` |
| Debian/Ubuntu | `chillgroup_*.deb` | Debian, Ubuntu, Mint | `sudo dpkg -i *.deb` or `sudo apt install ./*.deb` |
| RPM | `chillgroup-*.rpm` | Fedora, RHEL, openSUSE | `sudo rpm -i *.rpm` or `sudo dnf install ./*.rpm` |
| Pacman | `chillgroup-*.pacman` | Arch Linux, Manjaro | `sudo pacman -U *.pacman` |

Executables are published automatically to GitHub Releases on every `v*` tag push.

### macOS and Windows — Tauri

The client for macOS (universal: Intel + Apple Silicon) and Windows is built with Tauri v2 and published to the same GitHub Release.

### Client source layout

```
chillgroup/
├── electron/               # Electron client source (Linux)
│   ├── main.ts             # Main process (tray, window, screen share)
│   └── preload.ts
├── electron-builder.yml    # Linux package config (AppImage, deb, rpm, pacman)
├── package.json            # Electron dependencies and scripts
└── src-tauri/              # Tauri client source (macOS/Windows)
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── icons/
    └── src/
        ├── main.rs
        └── lib.rs
```

### Electron dev setup (Linux)

Requirements: Node.js 20+, pnpm.

```bash
pnpm install
pnpm electron:dev    # starts Vite + Electron together
pnpm electron:build  # production build (AppImage, deb, rpm, pacman)
```

The dev client connects to Vite at `http://localhost:5173`. Production bundles the compiled frontend inside the package.

### Versioning and release

`release.sh` bumps the version in all five places (`server/Cargo.toml`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `package.json`, `chillgroup-client/Cargo.toml` + its isolated `Cargo.lock`), creates a git tag, and pushes. CI builds all desktop formats from that tag, including the native Slint client (`chillgroup-client` job) for Linux, Windows and macOS — uploaded as `chillgroup-client-<version>-<os>.zip`. The webcam backend (`nokhwa`) and `livekit` are platform-specific: Linux `input-v4l` + `glib-main-loop`, macOS `input-avfoundation`, Windows `input-msmf`.

```bash
./release.sh patch    # e.g. 0.1.36 → 0.1.37
./release.sh minor    # e.g. 0.1.37 → 0.2.0
./release.sh v1.0.0   # explicit version
```

## Contributing

See the [Contributing guide](/en/contributing) for PR workflow and conventions.
