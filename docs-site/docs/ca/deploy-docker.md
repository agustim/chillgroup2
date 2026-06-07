# Deploy amb Docker

Aquesta guia explica com desplegar ChillGroup en local o en un servidor fent servir Docker Compose.

## Imatge precompilada i multi-arquitectura

Cada vegada que es publica una nova versió (tag `vX.Y.Z`) al repositori, GitHub Actions compila automàticament la imatge Docker per a **`linux/amd64`** i **`linux/arm64`** i les publica com a manifest multi-arch al GitHub Container Registry:

```
ghcr.io/agustim/chillgroup2:latest
ghcr.io/agustim/chillgroup2:vX.Y.Z
```

**Docker selecciona automàticament l'arquitectura correcta** en fer `docker compose pull` o `docker run`. No cal especificar cap sufixe d'arquitectura al `docker-compose.yml` — la mateixa imatge funciona tant en servidors x86_64 com en màquines ARM (Raspberry Pi, AWS Graviton, Apple Silicon via Docker Desktop, etc.).

No cal compilar res localment per desplegar en producció.

---

## Desplegament ràpid amb el wizard

El projecte inclou un script interactiu (`setup-deploy.sh`) que genera el `docker-compose.yml` i el `.env.compose` adaptats a la teva infraestructura.

### Opció A — sense clonar el repositori

```bash
curl -fsSL https://raw.githubusercontent.com/agustim/chillgroup2/main/setup-deploy.sh -o setup-deploy.sh
bash setup-deploy.sh
```

### Opció B — amb el repositori clonat

```bash
./setup-deploy.sh
```

### Opció C — pipe directe (sense revisar)

```bash
curl -fsSL https://raw.githubusercontent.com/agustim/chillgroup2/main/setup-deploy.sh | bash
```

> Recomanat revisar el script abans d'executar-lo amb pipe directe.

### Què pregunta el wizard

| Secció | Opcions |
|--------|---------|
| **HTTPS** | Cap (HTTP local/dev) · Caddy + Let's Encrypt · Cloudflare Tunnel |
| **Base de dades** | PostgreSQL local (container) · PostgreSQL extern · SQLite |
| **LiveKit** | Container local (mode dev) · Servidor remot |
| **S3** | RustFS local (container) · S3 extern (AWS, Cloudflare R2, MinIO…) |
| **S3 proxy** | Si el servidor actua de proxy per les pujades/baixades |
| **App** | Port, nivell de logs, registre obert/tancat |
| **Admin** | `ADMIN_USER` + `ADMIN_PASSWORD` (si registre tancat) · `ONE_ADMIN_INVITATION` (opcional) |
| **Secrets** | `JWT_SECRET` i `SERVER_MASTER_KEY` (genera automàticament si es deixa buit) |

### HTTPS i accés remot

::: warning Web Crypto API
L'API de criptografia del navegador (`crypto.subtle`) requereix un **context segur**. Funciona a `localhost` però **no funciona via HTTP des d'una màquina remota**. Si vols accedir a l'app des d'una altra màquina, necessites HTTPS.
:::

El wizard ofereix dues opcions integrades:

#### Opció A — Caddy (Let's Encrypt automàtic)

Caddy obté i renova el certificat TLS automàticament. Requisits:
- Un **domini** que apunti a la IP del servidor (registre DNS A)
- Ports **80 i 443 oberts** al firewall

El wizard genera un `Caddyfile` al directori de desplegament i afegeix el servei `caddy` al compose:

```
chillgroup.example.com {
    reverse_proxy app:8080
}
```

#### Opció B — Cloudflare Tunnel

Cloudflare Tunnel crea un túnel xifrat des del servidor cap a la xarxa de Cloudflare sense necessitat d'obrir ports ni tenir IP pública.

Requisits:
- Compte Cloudflare (pla gratuït suficient)
- Token de túnel obtingut a [one.dash.cloudflare.com](https://one.dash.cloudflare.com/) → Zero Trust → Tunnels

El wizard afegeix el servei `cloudflared` i guarda `CF_TUNNEL_TOKEN` al `.env.compose`. Cal configurar la ruta al dashboard de Cloudflare per redirigir cap a `http://app:8080`.

| | Caddy | Cloudflare Tunnel |
|---|---|---|
| Domini propi necessari | Sí | Opcional (`*.trycloudflare.com` gratuït) |
| Ports 80/443 oberts | Sí | No |
| Funciona darrera NAT | No | Sí |
| Certificat TLS | Let's Encrypt automàtic | Cloudflare (gestionat per CF) |

El wizard genera els fitxers al directori que tries (per defecte `./deploy`).

### Arrencar després del wizard

```bash
cd deploy
docker compose pull
docker compose up -d
```

### Veure logs

```bash
docker compose logs -f app
```

### Aturar

```bash
docker compose down
```

### Aturar i eliminar volums de dades

```bash
docker compose down -v
```

---

## Mode desenvolupament (per contribuir)

### Prerequisits

- Docker 24+
- Docker Compose (plugin `docker compose`)
- Linux (`backend` i `frontend` fan servir `network_mode: host`)

### Arrencada

```bash
docker compose -f docker-compose.dev.yml up
```

Aixeca:

- `postgres` (PostgreSQL 16) — port `5432`
- `livekit` (servei de veu) — port `7880`
- `rustfs` (S3 compatible) — ports `9000` / `9001`
- `rustfs-init` i `rustfs-cors-init` (serveis one-shot d'inicialització)
- `backend` — `cargo watch` amb hot-reload — port `8080`
- `frontend` — Vite dev server — port `5173`

Accedeix a l'app a: `http://localhost:5173`

La primera arrencada compila `cargo-watch` (uns minuts). Les següents reutilitzen el volum `cargo-tools`.

### Aturar

```bash
docker compose -f docker-compose.dev.yml down
```

Per eliminar també els volums de cache (Rust i node_modules):

```bash
docker compose -f docker-compose.dev.yml down -v
```

---

## Variables d'entorn

| Variable | Descripció | Obligatòria |
|----------|------------|-------------|
| `DATABASE_URL` | URL de connexió a la BD | Sí |
| `DATABASE_TYPE` | `postgres` o `sqlite` | Sí |
| `JWT_SECRET` | Secret per signar tokens JWT | Sí |
| `SERVER_MASTER_KEY` | Clau de xifratge (32 bytes hex) | Sí |
| `LIVEKIT_HOST` | URL del servidor LiveKit | Sí |
| `LIVEKIT_API_KEY` | Clau API de LiveKit | Sí |
| `LIVEKIT_API_SECRET` | Secret API de LiveKit | Sí |
| `S3_ENDPOINT` | Endpoint S3 (buit = AWS) | Sí |
| `S3_BUCKET` | Nom del bucket | Sí |
| `S3_ACCESS_KEY_ID` | Clau d'accés S3 | Sí |
| `S3_SECRET_ACCESS_KEY` | Secret S3 | Sí |
| `OPEN_REGISTER` | `true` permet registre lliure | No (default `true`) |
| `ADMIN_USER` | Usuari admin inicial | Sí si `OPEN_REGISTER=false` |
| `ADMIN_PASSWORD` | Contrasenya admin inicial | Sí si `OPEN_REGISTER=false` |
| `ONE_ADMIN_INVITATION` | Codi únic per promoure un usuari a admin | No |
| `SERVER_PROXY_S3` | El servidor actua de proxy S3 | No (default `false`) |

---

## Verificació

Quan arrenqui correctament, l'app respon a `http://localhost:8080` (o el port configurat).

Als logs hauries de veure:

- `Migrations PostgreSQL aplicades correctament` (o SQLite equivalent)
- `Base de dades connectada correctament`
- `Servidor escoltant a 0.0.0.0:8080`

---

## Notes de deploy real

- Canvia `JWT_SECRET` i `SERVER_MASTER_KEY` — no facis servir els valors generats per defecte en entorns públics si no els has revisit.
- No facis servir `OPEN_REGISTER=true` en producció un cop creat el primer admin.
- Si uses `ONE_ADMIN_INVITATION`, elimina o rota el codi després del primer ús.
- Revisa ports exposats i firewall.
- Si uses reverse proxy, apunta'l a `app:8080`.

## Checklist de producció

### 1) Reverse proxy i TLS

- **Opció recomanada**: usa el wizard (`setup-deploy.sh`) i tria Caddy o Cloudflare Tunnel — configuren HTTPS automàticament.
- Si prefereixes un proxy propi (Nginx, Traefik…): termina TLS al proxy i reenvía tràfic a `app:8080`.
- Activa redirecció HTTP → HTTPS i capçaleres de seguretat bàsiques.
- L'API de criptografia del navegador (`crypto.subtle`) no funciona sense HTTPS fora de `localhost`.

### 2) Secrets i configuració

- No guardis secrets al repositori.
- El wizard genera `JWT_SECRET` i `SERVER_MASTER_KEY` aleatoris — guarda'ls en un gestor de secrets.
- Usa valors llargs i aleatoris per claus i secrets.

### 3) Base de dades

- PostgreSQL: mantingues les dades en volum persistent i programa backups (`pg_dump`).
- SQLite: munta el volum en una ruta persistent i inclou-lo als backups.
- Limita l'accés de xarxa a PostgreSQL (no exposar el port 5432 públicament si no cal).

### 4) Observabilitat

- Recollecció de logs centralitzada (fitxers, Loki, ELK, etc.).
- Health checks actius per `app`, `postgres` i `livekit`.
- Alertes bàsiques: caiguda de contenidors, error rate alt, saturació de recursos.

### 5) Cicle de deploy

- Fes servir tags de versió (`v0.1.8`) en lloc de `latest` per a entorns crítics.
- Aplica canvis primer en staging.
- Defineix un pla de rollback (imatge anterior + restauració DB si cal).
