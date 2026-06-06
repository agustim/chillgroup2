#!/usr/bin/env bash
# Wizard interactiu per generar docker-compose.yml + .env.compose per a ChillGroup.
# Ús directe:  ./setup-deploy.sh
# Ús remot:    curl -fsSL <url> | bash
#              curl -fsSL <url> -o setup-deploy.sh && bash setup-deploy.sh

set -euo pipefail

# ── Helpers ───────────────────────────────────────────────────────────────────

section() {
    printf '\n%s\n  %s\n%s\n' \
        '──────────────────────────────────────────────────' \
        "$1" \
        '──────────────────────────────────────────────────'
}

# Llegeix sempre de /dev/tty per funcionar tant interactivament com amb curl | bash
_read() {
    read "$@" </dev/tty
}

ask() {
    local question="$1" default="${2:-}" answer
    if [[ -n "$default" ]]; then
        _read -rp "  ${question} [${default}]: " answer || true
        printf '%s' "${answer:-$default}"
    else
        while true; do
            _read -rp "  ${question}: " answer || true
            [[ -n "$answer" ]] && { printf '%s' "$answer"; return; }
            printf '    Cal introduir un valor.\n' >&2
        done
    fi
}

ask_choice() {
    local question="$1" default="$2"; shift 2
    local choices=("$@") opts="" answer c
    for c in "${choices[@]}"; do
        [[ "$c" == "$default" ]] && opts+="[${c}]/" || opts+="${c}/"
    done
    while true; do
        _read -rp "  ${question} (${opts%/}): " answer || true
        answer="${answer:-$default}"
        for c in "${choices[@]}"; do [[ "$c" == "$answer" ]] && { printf '%s' "$answer"; return; }; done
        printf '    Opcions vàlides: %s\n' "${choices[*]}" >&2
    done
}

ask_optional() {
    local question="$1" answer
    _read -rp "  ${question} (Enter per ometre): " answer || true
    printf '%s' "$answer"
}

ask_secret() {
    local question="$1" value
    printf '\n  %s\n  (Enter = genera automàticament)\n' "$question" >&2
    _read -rsp "  Valor: " value || true
    printf '\n' >&2
    if [[ -z "$value" ]]; then
        openssl rand -hex 32
    else
        printf '%s' "$value"
    fi
}

# ── Inici ─────────────────────────────────────────────────────────────────────

printf '╔════════════════════════════════════════════════════╗\n'
printf '║    ChillGroup — Generador de desplegament          ║\n'
printf '╚════════════════════════════════════════════════════╝\n'

# Directori de sortida
section "Directori de sortida"
OUT_DIR="$(ask "Directori on generar els fitxers" "./deploy")"
mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"
printf '  → %s\n' "$OUT_DIR"

COMPOSE_FILE="$OUT_DIR/docker-compose.yml"
ENV_FILE="$OUT_DIR/.env.compose"

# ── Base de dades ─────────────────────────────────────────────────────────────
section "Base de dades"
printf '  1) PostgreSQL local (container Docker)\n'
printf '  2) PostgreSQL extern\n'
printf '  3) SQLite (fitxer local, sense container)\n'
DB_CHOICE="$(ask_choice "Tria" "1" "1" "2" "3")"

case "$DB_CHOICE" in
  1)
    DB_TYPE="postgres_local"
    DB_NAME="$(ask "Nom de la base de dades" "chillgroup")"
    DB_USER="$(ask "Usuari" "chillgroup")"
    DB_PASS="$(ask "Contrasenya" "chillgroup")"
    DATABASE_URL="postgres://${DB_USER}:${DB_PASS}@postgres:5432/${DB_NAME}?sslmode=disable"
    DATABASE_TYPE_VAR="postgres"
    ;;
  2)
    DB_TYPE="postgres_ext"
    DATABASE_URL="$(ask "DATABASE_URL" "postgres://user:pass@host:5432/chillgroup?sslmode=disable")"
    DATABASE_TYPE_VAR="postgres"
    DB_NAME="" DB_USER="" DB_PASS=""
    ;;
  3)
    DB_TYPE="sqlite"
    DATABASE_URL="sqlite:///app/data/chillgroup.db"
    DATABASE_TYPE_VAR="sqlite"
    DB_NAME="" DB_USER="" DB_PASS=""
    printf '  → Fitxer: /app/data/chillgroup.db (dins del container)\n'
    ;;
esac

# ── LiveKit ───────────────────────────────────────────────────────────────────
section "LiveKit"
printf '  1) Local (container Docker en mode dev)\n'
printf '  2) Remot (servidor extern)\n'
LK_CHOICE="$(ask_choice "Tria" "1" "1" "2")"

if [[ "$LK_CHOICE" == "1" ]]; then
    LK_LOCAL=1
    LIVEKIT_HOST="http://host.docker.internal:7880"
    LIVEKIT_API_KEY="devkey"
    LIVEKIT_API_SECRET="secret"
    printf '  → %s  key=%s\n' "$LIVEKIT_HOST" "$LIVEKIT_API_KEY"
else
    LK_LOCAL=0
    LIVEKIT_HOST="$(ask "LIVEKIT_HOST" "https://livekit.example.com")"
    LIVEKIT_API_KEY="$(ask "LIVEKIT_API_KEY")"
    LIVEKIT_API_SECRET="$(ask "LIVEKIT_API_SECRET")"
fi

# ── S3 ────────────────────────────────────────────────────────────────────────
section "Emmagatzematge S3"
printf '  1) RustFS local (container Docker)\n'
printf '  2) S3 extern (AWS, Cloudflare R2, MinIO...)\n'
S3_CHOICE="$(ask_choice "Tria" "1" "1" "2")"

if [[ "$S3_CHOICE" == "1" ]]; then
    S3_LOCAL=1
    S3_BUCKET="$(ask "Nom del bucket" "chillgroup-attachments")"
    S3_ADMIN_KEY="$(ask "Clau d'admin RustFS" "rustfsadmin")"
    S3_ADMIN_SECRET="$(ask "Secret d'admin RustFS" "rustfsadmin")"
    S3_ENDPOINT_INTERNAL="http://rustfs:9000"
    S3_PUBLIC_ENDPOINT="$(ask "URL pública del S3 (des del navegador)" "http://localhost:9000")"
    S3_REGION="us-east-1"
    S3_ACCESS_KEY="$S3_ADMIN_KEY"
    S3_SECRET_KEY="$S3_ADMIN_SECRET"
    S3_FORCE_PATH="true"
    S3_CORS_1="$(ask "CORS origen 1 (URL de l'app)" "http://localhost:8080")"
    S3_CORS_2="$(ask "CORS origen 2 (opcional)" "$S3_CORS_1")"
    SERVER_PROXY_S3="$(ask_choice "Proxy S3 a través del servidor (SERVER_PROXY_S3)" "false" "true" "false")"
else
    S3_LOCAL=0
    S3_ADMIN_KEY="" S3_ADMIN_SECRET=""
    S3_ENDPOINT_INTERNAL="$(ask "S3_ENDPOINT (buit = AWS)" "")"
    S3_PUBLIC_ENDPOINT="$(ask "S3_PUBLIC_ENDPOINT" "")"
    S3_BUCKET="$(ask "S3_BUCKET" "chillgroup-attachments")"
    S3_ACCESS_KEY="$(ask "S3_ACCESS_KEY_ID")"
    S3_SECRET_KEY="$(ask "S3_SECRET_ACCESS_KEY")"
    S3_REGION="$(ask "S3_REGION" "us-east-1")"
    S3_FORCE_PATH="$(ask_choice "S3_FORCE_PATH_STYLE" "false" "true" "false")"
    S3_CORS_1="$(ask "S3_CORS_ALLOWED_ORIGIN_1" "https://app.example.com")"
    S3_CORS_2="$(ask "S3_CORS_ALLOWED_ORIGIN_2" "$S3_CORS_1")"
    SERVER_PROXY_S3="$(ask_choice "Proxy S3 a través del servidor (SERVER_PROXY_S3)" "false" "true" "false")"
fi

# ── App ───────────────────────────────────────────────────────────────────────
section "Configuració de l'app"
SERVER_PORT="$(ask "Port del servidor" "8080")"
BACKEND_DEBUG="$(ask_choice "Nivell de logs" "info" "error" "warn" "info" "debug")"
OPEN_REGISTER="$(ask_choice "Registre obert" "true" "true" "false")"
ADMIN_USER=""
ADMIN_PASSWORD=""
if [[ "$OPEN_REGISTER" == "false" ]]; then
    ADMIN_USER="$(ask "ADMIN_USER (obligatori si registre tancat)" "admin")"
    ADMIN_PASSWORD="$(ask_secret "ADMIN_PASSWORD")"
fi
printf '\n  ONE_ADMIN_INVITATION — codi únic per crear el primer admin (opcional)\n'
ONE_ADMIN_INVITATION="$(ask_optional "Codi invitació admin")"
TTL_CLEANUP="$(ask "Interval de temps en minuts per netejar missatges expirats" "5")"
JWT_SECRET="$(ask_secret "JWT_SECRET")"
SERVER_MASTER_KEY="$(ask_secret "SERVER_MASTER_KEY (32 bytes hex)")"

# ── Genera docker-compose.yml ─────────────────────────────────────────────────
section "Generant fitxers..."

printf 'services:\n' > "$COMPOSE_FILE"

# Postgres
if [[ "$DB_TYPE" == "postgres_local" ]]; then
    cat >> "$COMPOSE_FILE" << EOF

  postgres:
    image: postgres:16-alpine
    env_file:
      - .env.compose
    volumes:
      - pgdata:/var/lib/postgresql/data
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U ${DB_USER} -d ${DB_NAME}"]
      interval: 5s
      timeout: 5s
      retries: 10
EOF
fi

# LiveKit
if [[ "$LK_LOCAL" == "1" ]]; then
    cat >> "$COMPOSE_FILE" << 'EOF'

  livekit:
    image: livekit/livekit-server:latest
    command:
      - --bind
      - 0.0.0.0
      - --port
      - "7880"
      - --dev
    network_mode: host
    healthcheck:
      test: ["CMD-SHELL", "wget -q --spider http://localhost:7880/"]
      interval: 10s
      timeout: 5s
      retries: 10
EOF
fi

# RustFS
if [[ "$S3_LOCAL" == "1" ]]; then
    cat >> "$COMPOSE_FILE" << 'EOF'

  rustfs:
    image: rustfs/rustfs:latest
    command: /data
    env_file:
      - .env.compose
    volumes:
      - rustfsdata:/data
    ports:
      - "9000:9000"
      - "9001:9001"
    healthcheck:
      test: ["CMD-SHELL", "curl -sS http://localhost:9000/ >/dev/null || exit 1"]
      interval: 10s
      timeout: 5s
      retries: 10

  rustfs-init:
    image: minio/mc:latest
    env_file:
      - .env.compose
    depends_on:
      rustfs:
        condition: service_healthy
    entrypoint: >
      /bin/sh -c "
      mc alias set rustfs $$S3_ENDPOINT $$S3_ACCESS_KEY_ID $$S3_SECRET_ACCESS_KEY &&
      mc mb --ignore-existing rustfs/$$S3_BUCKET
      "
    restart: "no"

  rustfs-cors-init:
    image: amazon/aws-cli:2.22.35
    env_file:
      - .env.compose
    depends_on:
      rustfs:
        condition: service_healthy
      rustfs-init:
        condition: service_completed_successfully
    entrypoint: >
      /bin/sh -c "
      export AWS_ACCESS_KEY_ID=$$S3_ACCESS_KEY_ID &&
      export AWS_SECRET_ACCESS_KEY=$$S3_SECRET_ACCESS_KEY &&
      export AWS_DEFAULT_REGION=$$S3_REGION &&
      export AWS_EC2_METADATA_DISABLED=true &&
      printf '%s' '{\"CORSRules\":[{\"AllowedOrigins\":[\"'\"$$S3_CORS_ALLOWED_ORIGIN_1\"'\",\"'\"$$S3_CORS_ALLOWED_ORIGIN_2\"'\"],\"AllowedMethods\":[\"GET\",\"HEAD\",\"PUT\",\"POST\",\"DELETE\"],\"AllowedHeaders\":[\"*\"],\"ExposeHeaders\":[\"ETag\",\"x-amz-request-id\"],\"MaxAgeSeconds\":3600}]}' > /tmp/cors.json &&
      aws --endpoint-url $$S3_ENDPOINT s3api put-bucket-cors --bucket $$S3_BUCKET --cors-configuration file:///tmp/cors.json
      "
    restart: "no"
EOF
fi

# App — depends_on dinàmic
printf '\n  app:\n' >> "$COMPOSE_FILE"
printf '    image: ghcr.io/agustim/chillgroup2:latest\n' >> "$COMPOSE_FILE"
printf '    env_file:\n      - .env.compose\n' >> "$COMPOSE_FILE"
printf '    extra_hosts:\n      - "host.docker.internal:host-gateway"\n' >> "$COMPOSE_FILE"

# Volum SQLite
if [[ "$DB_TYPE" == "sqlite" ]]; then
    printf '    volumes:\n      - sqlitedata:/app/data\n' >> "$COMPOSE_FILE"
fi

# depends_on
HAS_DEPENDS=0
DEPENDS_BLOCK=""
[[ "$DB_TYPE" == "postgres_local" ]] && DEPENDS_BLOCK+="      postgres:\n        condition: service_healthy\n" && HAS_DEPENDS=1
[[ "$LK_LOCAL" == "1" ]]             && DEPENDS_BLOCK+="      livekit:\n        condition: service_healthy\n" && HAS_DEPENDS=1
if [[ "$S3_LOCAL" == "1" ]]; then
    DEPENDS_BLOCK+="      rustfs:\n        condition: service_healthy\n"
    DEPENDS_BLOCK+="      rustfs-init:\n        condition: service_completed_successfully\n"
    DEPENDS_BLOCK+="      rustfs-cors-init:\n        condition: service_completed_successfully\n"
    HAS_DEPENDS=1
fi
if [[ "$HAS_DEPENDS" == "1" ]]; then
    printf '    depends_on:\n' >> "$COMPOSE_FILE"
    printf '%b' "$DEPENDS_BLOCK" >> "$COMPOSE_FILE"
fi

printf '    ports:\n      - "%s:8080"\n' "$SERVER_PORT" >> "$COMPOSE_FILE"

# Volumes
VOLS=()
[[ "$DB_TYPE" == "postgres_local" ]] && VOLS+=("pgdata")
[[ "$DB_TYPE" == "sqlite" ]]         && VOLS+=("sqlitedata")
[[ "$S3_LOCAL" == "1" ]]             && VOLS+=("rustfsdata")

if [[ ${#VOLS[@]} -gt 0 ]]; then
    printf '\nvolumes:\n' >> "$COMPOSE_FILE"
    for v in "${VOLS[@]}"; do
        printf '  %s:\n' "$v" >> "$COMPOSE_FILE"
    done
fi

# ── Genera .env.compose ───────────────────────────────────────────────────────
{
    printf '# Base de dades\n'
    if [[ "$DB_TYPE" == "postgres_local" ]]; then
        printf 'POSTGRES_DB=%s\n' "$DB_NAME"
        printf 'POSTGRES_USER=%s\n' "$DB_USER"
        printf 'POSTGRES_PASSWORD=%s\n' "$DB_PASS"
    fi
    printf 'DATABASE_TYPE=%s\n' "$DATABASE_TYPE_VAR"
    printf 'DATABASE_URL=%s\n' "$DATABASE_URL"
    printf '\n'

    printf '# Backend\n'
    printf 'SERVER_HOST=0.0.0.0\n'
    printf 'SERVER_PORT=%s\n' "$SERVER_PORT"
    printf 'BACKEND_DEBUG=%s\n' "$BACKEND_DEBUG"
    printf 'JWT_SECRET=%s\n' "$JWT_SECRET"
    printf 'SERVER_MASTER_KEY=%s\n' "$SERVER_MASTER_KEY"
    printf 'OPEN_REGISTER=%s\n' "$OPEN_REGISTER"
    if [[ "$OPEN_REGISTER" == "false" ]]; then
        printf 'ADMIN_USER=%s\n' "$ADMIN_USER"
        printf 'ADMIN_PASSWORD=%s\n' "$ADMIN_PASSWORD"
    fi
    [[ -n "$ONE_ADMIN_INVITATION" ]] && printf 'ONE_ADMIN_INVITATION=%s\n' "$ONE_ADMIN_INVITATION"
    printf 'TTL_CLEANUP_INTERVAL_MINUTES=%s\n' "$TTL_CLEANUP"
    printf '\n'

    printf '# LiveKit\n'
    printf 'LIVEKIT_HOST=%s\n' "$LIVEKIT_HOST"
    printf 'LIVEKIT_API_KEY=%s\n' "$LIVEKIT_API_KEY"
    printf 'LIVEKIT_API_SECRET=%s\n' "$LIVEKIT_API_SECRET"
    printf '\n'

    printf '# S3\n'
    if [[ "$S3_LOCAL" == "1" ]]; then
        printf 'RUSTFS_ACCESS_KEY=%s\n' "$S3_ADMIN_KEY"
        printf 'RUSTFS_SECRET_KEY=%s\n' "$S3_ADMIN_SECRET"
        printf 'RUSTFS_CONSOLE_ENABLE=true\n'
    fi
    [[ -n "$S3_ENDPOINT_INTERNAL" ]] && printf 'S3_ENDPOINT=%s\n' "$S3_ENDPOINT_INTERNAL"
    [[ -n "$S3_PUBLIC_ENDPOINT" ]]   && printf 'S3_PUBLIC_ENDPOINT=%s\n' "$S3_PUBLIC_ENDPOINT"
    printf 'S3_REGION=%s\n' "$S3_REGION"
    printf 'S3_BUCKET=%s\n' "$S3_BUCKET"
    printf 'S3_ACCESS_KEY_ID=%s\n' "$S3_ACCESS_KEY"
    printf 'S3_SECRET_ACCESS_KEY=%s\n' "$S3_SECRET_KEY"
    printf 'S3_FORCE_PATH_STYLE=%s\n' "$S3_FORCE_PATH"
    printf 'S3_CORS_ALLOWED_ORIGIN_1=%s\n' "$S3_CORS_1"
    printf 'S3_CORS_ALLOWED_ORIGIN_2=%s\n' "$S3_CORS_2"
    printf 'SERVER_PROXY_S3=%s\n' "$SERVER_PROXY_S3"
} > "$ENV_FILE"

# ── Resum ─────────────────────────────────────────────────────────────────────
section "Fitxers generats"
printf '  ✅ %s\n' "$COMPOSE_FILE"
printf '  ✅ %s\n' "$ENV_FILE"
printf '\n'
printf '  ⚠️  .env.compose conté secrets — no pujar al repositori!\n'
printf '\n'
printf '  Per arrencar:\n'
printf '    cd %s\n' "$OUT_DIR"
printf '    docker compose pull\n'
printf '    docker compose up -d\n'
printf '\n'
