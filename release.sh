#!/usr/bin/env bash

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVER_TOML="$ROOT/server/Cargo.toml"

usage() {
	cat <<'EOF'
Ús: ./release.sh [patch|minor|major|vX.Y.Z]

Arguments:
  patch      Incrementa versió patch (per defecte): 0.1.8 → 0.1.9
  minor      Incrementa versió minor:               0.1.8 → 0.2.0
  major      Incrementa versió major:               0.1.8 → 1.0.0
  vX.Y.Z     Versió explícita, ex: v0.2.0

El script:
  1. Actualitza server/Cargo.toml
  2. Actualitza Cargo.lock (cargo check)
  3. Commit + tag git
  4. Push a origin (commit + tag)
EOF
}

BUMP="${1:-patch}"

if [[ "$BUMP" == "--help" || "$BUMP" == "-h" ]]; then
	usage
	exit 0
fi

# Llegeix versió actual
CURRENT="$(grep -E '^version\s*=' "$SERVER_TOML" | head -1 | sed 's/.*"\(.*\)".*/\1/')"
echo "Versió actual: $CURRENT"

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

case "$BUMP" in
	patch)
		NEW_VERSION="${MAJOR}.${MINOR}.$((PATCH + 1))"
		;;
	minor)
		NEW_VERSION="${MAJOR}.$((MINOR + 1)).0"
		;;
	major)
		NEW_VERSION="$((MAJOR + 1)).0.0"
		;;
	v*.*.*)
		NEW_VERSION="${BUMP#v}"
		;;
	*.*.*)
		NEW_VERSION="$BUMP"
		;;
	*)
		echo "Argument invàlid: $BUMP" >&2
		usage >&2
		exit 1
		;;
esac

TAG="v${NEW_VERSION}"
echo "Nova versió:   $NEW_VERSION  ($TAG)"

# Comprova branca main
CURRENT_BRANCH="$(git -C "$ROOT" rev-parse --abbrev-ref HEAD)"
if [[ "$CURRENT_BRANCH" != "main" ]]; then
	echo "Error: has d'estar a la branca 'main' (branca actual: $CURRENT_BRANCH)." >&2
	exit 1
fi

# Comprova estat git net
if [[ -n "$(git -C "$ROOT" status --porcelain)" ]]; then
	echo "" >&2
	echo "Error: directori de treball brut. Fes commit o stash dels canvis primer." >&2
	git -C "$ROOT" status --short >&2
	exit 1
fi

# Comprova que el tag no existeix ja
if git -C "$ROOT" rev-parse "$TAG" &>/dev/null; then
	echo "Error: el tag $TAG ja existeix." >&2
	exit 1
fi

# Actualitza server/Cargo.toml
sed -i "s/^version\s*=\s*\"${CURRENT}\"/version = \"${NEW_VERSION}\"/" "$SERVER_TOML"
echo "✅ server/Cargo.toml actualitzat"

# Actualitza Cargo.lock
cd "$ROOT"
cargo check -q -p chillgroup-server 2>/dev/null || cargo generate-lockfile
echo "✅ Cargo.lock actualitzat"

# Commit
git -C "$ROOT" add server/Cargo.toml Cargo.lock
git -C "$ROOT" commit -m "chore: bump version to ${TAG}"
echo "✅ Commit creat"

# Tag
git -C "$ROOT" tag "$TAG"
echo "✅ Tag $TAG creat"

# Push
echo "Pujant a origin..."
git -C "$ROOT" push origin HEAD
git -C "$ROOT" push origin "$TAG"
echo ""
echo "🚀 Versió $TAG publicada! El workflow de GitHub Actions s'ha iniciat."
