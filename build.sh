#!/usr/bin/env bash
# Script de build de producció: compila el frontend i l'integra dins el binari Rust.
# Resultat: target/release/chillgroup-server + target/release/static/

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRONTEND_DIR="$ROOT/frontend"
DIST_DIR="$ROOT/target/release/static"

echo "🔨 Build de producció ChillGroup"
echo ""

# --- Frontend ---
echo "📦 Compilant frontend..."
cd "$FRONTEND_DIR"
pnpm install --frozen-lockfile
pnpm run build
echo "✅ Frontend compilat"

# --- Copiar dist al costat del binari ---
echo "📂 Copiant fitxers estàtics a $DIST_DIR..."
rm -rf "$DIST_DIR"
cp -r "$FRONTEND_DIR/dist" "$DIST_DIR"
echo "✅ Fitxers estàtics copiats"

# --- Backend ---
cd "$ROOT"
echo "⚙️  Compilant servidor Rust (release)..."
cargo build --release -p chillgroup-server
echo "✅ Servidor compilat"

echo ""
echo "🚀 Build completat!"
echo "   Binari:   target/release/chillgroup-server"
echo "   Estàtics: target/release/static/"
echo ""
echo "Per executar:"
echo "   cd target/release && ./chillgroup-server"
echo ""
echo "   (El servidor servirà el frontend des de ./static automàticament)"
