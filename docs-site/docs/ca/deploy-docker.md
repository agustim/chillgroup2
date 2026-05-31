# Deploy amb Docker

Aquesta guia explica com aixecar ChillGroup en local o servidor fent servir Docker Compose.

## Prerequisits

- Docker 24+
- Docker Compose (plugin `docker compose`)
- Port 8080 lliure (app)
- Port 5432 lliure (PostgreSQL)
- Port 7880 lliure (LiveKit)

## Arrencada rapida

Des de l'arrel del projecte:

```bash
docker compose up --build
```

Aixo aixeca:

- `postgres` (PostgreSQL 16)
- `livekit` (servei de veu)
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
