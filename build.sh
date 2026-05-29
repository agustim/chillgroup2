#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRONTEND_DIR="$ROOT/frontend"
MODE="external"
TARGET=""

usage() {
	cat <<'EOF'
Ús: ./build.sh [--mode external|embedded] [--target <triple>]

Opcions:
	--mode     external: genera binari + directori static/ al costat
						 embedded: incrusta el frontend dins del binari Rust
	--target   Triple de Rust, per exemple x86_64-unknown-linux-gnu o aarch64-unknown-linux-gnu
	--help     Mostra aquesta ajuda
EOF
}

while [[ $# -gt 0 ]]; do
	case "$1" in
		--mode)
			MODE="${2:-}"
			shift 2
			;;
		--target)
			TARGET="${2:-}"
			shift 2
			;;
		--help|-h)
			usage
			exit 0
			;;
		*)
			echo "Argument desconegut: $1" >&2
			usage >&2
			exit 1
			;;
	esac
done

if [[ "$MODE" != "external" && "$MODE" != "embedded" ]]; then
	echo "Mode invàlid: $MODE" >&2
	usage >&2
	exit 1
fi

TARGET_DIR="$ROOT/target"
CARGO_ARGS=(build --release -p chillgroup-server)
if [[ -n "$TARGET" ]]; then
	CARGO_ARGS+=(--target "$TARGET")
	TARGET_DIR="$TARGET_DIR/$TARGET"
fi

BIN_PATH="$TARGET_DIR/release/chillgroup-server"
DIST_DIR="$TARGET_DIR/release/static"

echo "🔨 Build de producció ChillGroup"
echo "   mode:   $MODE"
if [[ -n "$TARGET" ]]; then
	echo "   target: $TARGET"
else
	echo "   target: host"
fi
echo ""

echo "📦 Compilant frontend..."
cd "$FRONTEND_DIR"
pnpm install --frozen-lockfile
pnpm run build
echo "✅ Frontend compilat"

cd "$ROOT"

if [[ "$MODE" == "external" ]]; then
	echo "📂 Copiant fitxers estàtics a $DIST_DIR..."
	rm -rf "$DIST_DIR"
	cp -r "$FRONTEND_DIR/dist" "$DIST_DIR"
	echo "✅ Fitxers estàtics copiats"
else
	echo "📦 El frontend quedarà incrustat dins del binari"
	CARGO_ARGS+=(--features embedded-assets)
fi

if [[ -n "$TARGET" ]]; then
	echo "🧩 Assegurant target Rust: $TARGET"
	rustup target add "$TARGET" >/dev/null
fi

echo "⚙️  Compilant servidor Rust (release)..."
cargo "${CARGO_ARGS[@]}"
echo "✅ Servidor compilat"

echo ""
echo "🚀 Build completat!"
echo "   Binari: $BIN_PATH"
if [[ "$MODE" == "external" ]]; then
	echo "   Estàtics: $DIST_DIR/"
	echo ""
	echo "Per executar:"
	echo "   cd $(dirname "$BIN_PATH") && ./$(basename "$BIN_PATH")"
	echo ""
	echo "   (El servidor servirà el frontend des de ./static automàticament)"
else
	echo "   Frontend: incrustat dins del binari"
	echo ""
	echo "Per executar:"
	echo "   $(printf '%q' "$BIN_PATH")"
fi
