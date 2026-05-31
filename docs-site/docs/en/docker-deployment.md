# Docker Deployment

This guide explains how to run ChillGroup locally or on a server using Docker Compose.

## Prerequisites

- Docker 24+
- Docker Compose (`docker compose` plugin)
- Free port 8080 (app)
- Free port 5432 (PostgreSQL)
- Free port 7880 (LiveKit)

## Quick start

From the project root:

```bash
docker compose up --build
```

This starts:

- `postgres` (PostgreSQL 16)
- `livekit` (voice service)
- `app` (Rust backend with embedded frontend)

## Run in background

```bash
docker compose up -d --build
```

View logs:

```bash
docker compose logs -f app
```

Stop services:

```bash
docker compose down
```

Stop and remove database volume:

```bash
docker compose down -v
```

## Important environment variables

Core variables are already defined in `docker-compose.yml` for local usage.

Most relevant values:

- `DATABASE_URL`
- `LIVEKIT_HOST`
- `LIVEKIT_API_KEY`
- `LIVEKIT_API_SECRET`
- `JWT_SECRET`
- `SERVER_MASTER_KEY`
- `OPEN_REGISTER`

## Verification

When startup is successful, the app is available at:

- `http://localhost:8080`

Logs should include messages similar to:

- `Migrations PostgreSQL aplicades correctament`
- `Base de dades connectada correctament`
- `Servidor escoltant a 0.0.0.0:8080`

## Production notes

For non-local deployments:

- Replace `JWT_SECRET` and `SERVER_MASTER_KEY`.
- Do not keep `OPEN_REGISTER=true` in production.
- Review exposed ports and firewall rules.
- If using a reverse proxy, route traffic to `app:8080`.

## Production checklist

### 1) Reverse proxy and TLS

- Expose the app over HTTPS only (Nginx, Caddy, or Traefik).
- Terminate TLS at the proxy and forward traffic to `app:8080`.
- Enable HTTP -> HTTPS redirect and basic security headers.

### 2) Secrets and configuration

- Do not store secrets in git or directly in production compose files.
- Inject `JWT_SECRET`, `SERVER_MASTER_KEY`, and LiveKit credentials from a secrets manager or environment.
- Use long, random values for all keys and secrets.

### 3) Database operations

- Keep PostgreSQL data on a persistent volume.
- Schedule regular backups (`pg_dump`) and test restore procedures.
- Restrict PostgreSQL network exposure (avoid public 5432 when possible).

### 4) Observability

- Centralize logs (files, Loki, ELK, etc.).
- Keep health checks enabled for `app`, `postgres`, and `livekit`.
- Add alerts for container crashes, high error rate, and resource saturation.

### 5) Deployment workflow

- Use versioned image tags, avoid `latest` in production.
- Promote changes through staging before production.
- Define rollback steps (previous image + DB restore path when needed).
