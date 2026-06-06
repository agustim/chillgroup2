# Docker Deployment

This guide explains how to run ChillGroup locally or on a server using Docker Compose.

## Pre-built image

Every time a new release is tagged (`vX.Y.Z`), GitHub Actions builds the Docker image and publishes it to the GitHub Container Registry:

```
ghcr.io/agustim/chillgroup2:latest
ghcr.io/agustim/chillgroup2:vX.Y.Z
```

No local compilation is required to deploy.

---

## Quick deployment with the wizard

The project ships an interactive setup script (`setup-deploy.sh`) that generates a `docker-compose.yml` and `.env.compose` tailored to your infrastructure.

### Option A — without cloning the repository

```bash
curl -fsSL https://raw.githubusercontent.com/agustim/chillgroup2/main/setup-deploy.sh -o setup-deploy.sh
bash setup-deploy.sh
```

### Option B — with the repository cloned

```bash
./setup-deploy.sh
```

### Option C — pipe directly (without reviewing)

```bash
curl -fsSL https://raw.githubusercontent.com/agustim/chillgroup2/main/setup-deploy.sh | bash
```

> It is recommended to review the script before running it via direct pipe.

### What the wizard asks

| Section | Options |
|---------|---------|
| **Database** | Local PostgreSQL (container) · External PostgreSQL · SQLite |
| **LiveKit** | Local container (dev mode) · Remote server |
| **S3 storage** | Local RustFS (container) · External S3 (AWS, Cloudflare R2, MinIO…) |
| **S3 proxy** | Whether the server proxies S3 uploads/downloads |
| **App** | Port, log level, open/closed registration |
| **Admin** | `ADMIN_USER` + `ADMIN_PASSWORD` (if registration closed) · `ONE_ADMIN_INVITATION` (optional) |
| **Secrets** | `JWT_SECRET` and `SERVER_MASTER_KEY` (auto-generated if left blank) |

The wizard writes the generated files to the directory you choose (default: `./deploy`).

### Starting after the wizard

```bash
cd deploy
docker compose pull
docker compose up -d
```

### View logs

```bash
docker compose logs -f app
```

### Stop

```bash
docker compose down
```

### Stop and remove data volumes

```bash
docker compose down -v
```

---

## Development mode (for contributors)

### Prerequisites

- Docker 24+
- Docker Compose (`docker compose` plugin)
- Linux (`backend` and `frontend` services use `network_mode: host`)

### Start

```bash
docker compose -f docker-compose.dev.yml up
```

This starts:

- `postgres` (PostgreSQL 16) — port `5432`
- `livekit` (voice service) — port `7880`
- `rustfs` (S3-compatible storage) — ports `9000` / `9001`
- `rustfs-init` and `rustfs-cors-init` (one-shot init services)
- `backend` — `cargo watch` with hot-reload — port `8080`
- `frontend` — Vite dev server — port `5173`

Access the app at: `http://localhost:5173`

The first run compiles `cargo-watch` (takes a few minutes). Subsequent starts reuse the `cargo-tools` volume cache.

### Stop

```bash
docker compose -f docker-compose.dev.yml down
```

To also remove the build and module caches:

```bash
docker compose -f docker-compose.dev.yml down -v
```

---

## Environment variables

| Variable | Description | Required |
|----------|-------------|----------|
| `DATABASE_URL` | Database connection URL | Yes |
| `DATABASE_TYPE` | `postgres` or `sqlite` | Yes |
| `JWT_SECRET` | Secret for signing JWT tokens | Yes |
| `SERVER_MASTER_KEY` | Encryption key (32-byte hex) | Yes |
| `LIVEKIT_HOST` | LiveKit server URL | Yes |
| `LIVEKIT_API_KEY` | LiveKit API key | Yes |
| `LIVEKIT_API_SECRET` | LiveKit API secret | Yes |
| `S3_ENDPOINT` | S3 endpoint (empty = AWS) | Yes |
| `S3_BUCKET` | Bucket name | Yes |
| `S3_ACCESS_KEY_ID` | S3 access key | Yes |
| `S3_SECRET_ACCESS_KEY` | S3 secret key | Yes |
| `OPEN_REGISTER` | `true` allows open registration | No (default `true`) |
| `ADMIN_USER` | Initial admin username | Yes if `OPEN_REGISTER=false` |
| `ADMIN_PASSWORD` | Initial admin password | Yes if `OPEN_REGISTER=false` |
| `ONE_ADMIN_INVITATION` | One-time code to promote a user to admin | No |
| `SERVER_PROXY_S3` | Server acts as S3 proxy | No (default `false`) |

---

## Verification

When startup is successful, the app is available at `http://localhost:8080` (or the configured port).

Logs should include messages similar to:

- `Migrations PostgreSQL aplicades correctament` (or SQLite equivalent)
- `Base de dades connectada correctament`
- `Servidor escoltant a 0.0.0.0:8080`

---

## Production notes

- Replace `JWT_SECRET` and `SERVER_MASTER_KEY` with strong random values before exposing the app publicly.
- Disable `OPEN_REGISTER` once the first admin account is created.
- If using `ONE_ADMIN_INVITATION`, remove or rotate the code after first use.
- Review exposed ports and firewall rules.
- If using a reverse proxy, route traffic to `app:8080`.

## Production checklist

### 1) Reverse proxy and TLS

- Expose the app over HTTPS only (Nginx, Caddy, or Traefik).
- Terminate TLS at the proxy and forward traffic to `app:8080`.
- Enable HTTP → HTTPS redirect and basic security headers.

### 2) Secrets and configuration

- Do not store secrets in git.
- The wizard auto-generates `JWT_SECRET` and `SERVER_MASTER_KEY` — store them in a secrets manager.
- Use long, random values for all keys and secrets.

### 3) Database

- PostgreSQL: keep data on a persistent volume and schedule regular backups (`pg_dump`).
- SQLite: mount the volume to a persistent path and include it in your backup strategy.
- Restrict PostgreSQL network exposure (avoid public port 5432 when possible).

### 4) Observability

- Centralize logs (files, Loki, ELK, etc.).
- Keep health checks enabled for `app`, `postgres`, and `livekit`.
- Add alerts for container crashes, high error rate, and resource saturation.

### 5) Deployment workflow

- Use versioned image tags (`v0.1.8`) instead of `latest` for critical environments.
- Promote changes through staging before production.
- Define rollback steps (previous image + DB restore path when needed).
