# Deploy amb Docker

Aquesta guia explica com aixecar ChillGroup en local o servidor fent servir Docker Compose.

El projecte te dos fitxers Compose:

| Fitxer | Quan usar-lo |
|--------|-------------|
| `docker-compose.yml` | Producció o testeig de la build final |
| `docker-compose.dev.yml` | Desenvolupament diari amb hot-reload |

## Mode desenvolupament (recomanat per contribuir)

### Prerequisits

- Docker 24+
- Docker Compose (plugin `docker compose`)
- Linux (els serveis `backend` i `frontend` fan servir `network_mode: host`)

### Arrencada

```bash
docker compose -f docker-compose.dev.yml up
```

Aixo aixeca:

- `postgres` (PostgreSQL 16) — port `5432`
- `livekit` (servei de veu) — port `7880`
- `rustfs` (S3 compatible) — ports `9000` / `9001`
- `rustfs-init` i `rustfs-cors-init` (serveis one-shot d'inicialitzacio)
- `backend` — `cargo watch` amb hot-reload — port `8080`
- `frontend` — Vite dev server — port `5173`

Accedeix a l'app a: `http://localhost:5173`

La primera arrencada compila `cargo-watch` (uns minuts). Les seguents reutilitzen el volum `cargo-tools`.

### Aturar

```bash
docker compose -f docker-compose.dev.yml down
```

Per eliminar tambe els volums de cache (Rust i node_modules):

```bash
docker compose -f docker-compose.dev.yml down -v
```

---

## Mode produccio

### Prerequisits

- Docker 24+
- Docker Compose (plugin `docker compose`)
- Port 8080 lliure (app)
- Port 5432 lliure (PostgreSQL)
- Port 7880 lliure (LiveKit)

### Arrencada rapida

Des de l'arrel del projecte:

```bash
docker compose up --build
```

Aixo aixeca:

- `postgres` (PostgreSQL 16)
- `livekit` (servei de veu)
- `rustfs` (S3 compatible per adjunts)
- `app` (backend Rust amb frontend incrustat)

## Execucio en segon pla

```bash
docker compose up -d --build
```

Per veure logs:

```bash
docker compose logs -f app
```

Per aturar:

```bash
docker compose down
```

Per aturar i eliminar volum de base de dades:

```bash
docker compose down -v
```

## Variables d'entorn importants

Les variables principals ja venen definides a `docker-compose.yml` per a entorn local.

Les mes rellevants son:

- `DATABASE_URL`
- `LIVEKIT_HOST`
- `LIVEKIT_API_KEY`
- `LIVEKIT_API_SECRET`
- `JWT_SECRET`
- `SERVER_MASTER_KEY`
- `OPEN_REGISTER`
- `ONE_ADMIN_INVITATION` (opcional, un sol ús per promocionar un registre a admin)

## Verificacio

Quan arrenqui correctament, l'app respon a:

- `http://localhost:8080`

I als logs hauries de veure missatges similars a:

- `Migrations PostgreSQL aplicades correctament`
- `Base de dades connectada correctament`
- `Servidor escoltant a 0.0.0.0:8080`

## Notes de deploy real

Per entorns no locals:

- Canvia `JWT_SECRET` i `SERVER_MASTER_KEY`.
- No facis servir `OPEN_REGISTER=true` en produccio.
- Si uses `ONE_ADMIN_INVITATION`, elimina-la o rota-la després del primer ús.
- Revisa ports exposats i firewall.
- Si uses reverse proxy, apunta'l a `app:8080`.

## Checklist de produccio

### 1) Reverse proxy i TLS

- Publica nomes HTTPS (Nginx, Caddy o Traefik).
- Termina TLS al proxy i reenvia trafic a `app:8080`.
- Activa redireccio HTTP -> HTTPS i capcaleres de seguretat basiques.

### 2) Secrets i configuracio

- No guardis secrets al repositori ni al `docker-compose.yml` de produccio.
- Carrega `JWT_SECRET`, `SERVER_MASTER_KEY` i claus LiveKit des d'un gestor de secrets o variables del sistema.
- Usa valors llargs i aleatoris per claus i secrets.

### 3) Base de dades

- Mantingues PostgreSQL en volum persistent.
- Programa backups periodics (`pg_dump`) i prova restauracions.
- Limita acces de xarxa a PostgreSQL (no exposar 5432 publicament si no cal).

### 4) Observabilitat

- Recolleccio de logs centralitzada (fitxers, Loki, ELK, etc.).
- Health checks actius per `app`, `postgres` i `livekit`.
- Alertes basiques: caiguda de contenidors, error rate alt, saturacio CPU/RAM.

### 5) Cicle de deploy

- Fes deploy amb imatges versionades (tags), evita `latest` en produccio.
- Aplica canvis primer en staging.
- Defineix un pla de rollback (imatge anterior + restauracio DB si cal).
