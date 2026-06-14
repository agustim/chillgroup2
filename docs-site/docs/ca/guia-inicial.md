# Guia inicial

Aquest portal serveix per publicar la documentacio de ChillGroup com a web estatica a GitHub Pages.

## Que hi trobaras

- Una entrada rapida al projecte.
- Enllacos a tota l'especificacio tecnica existent.
- Una estructura bilingue per ampliar contingut en catala i angles.

## Mapa del projecte

- `definitions/`: font original de la documentacio tecnica.
- `docs-site/`: projecte VitePress per generar la web.
- `frontend/`: client React de l'aplicacio.
- `server/`: backend Rust amb Axum i SQLx.

## Flux recomanat

1. Actualitza la documentacio tecnica a `definitions/`.
2. Executa el build del portal de docs.
3. GitHub Pages publica la versio estatica generada.

## App d'escriptori (Linux, macOS, Windows)

A cada release de GitHub s'inclouen clients d'escriptori natius:

- **Linux**: AppImage (totes les distros), `.deb` (Debian/Ubuntu/Mint), `.rpm` (Fedora/RHEL/openSUSE), `.pacman` (Arch/Manjaro)
- **macOS**: `.dmg` universal (Intel + Apple Silicon)
- **Windows**: instal·lador `.msi`

Descarrega des de [GitHub Releases](https://github.com/agustim/chillgroup2/releases).

Exemples d'instal·lació ràpida per a Linux:

```bash
# AppImage — funciona a qualsevol distro, sense instal·lació
chmod +x ChillGroup-*.AppImage && ./ChillGroup-*.AppImage

# Debian / Ubuntu / Mint
sudo apt install ./chillgroup_*.deb

# Fedora / RHEL / openSUSE
sudo dnf install ./chillgroup-*.rpm

# Arch Linux / Manjaro
sudo pacman -U chillgroup-*.pacman
```

## Deploy del projecte

El projecte publica una imatge Docker precompilada a cada release. Per desplegar, executa el wizard interactiu que genera el `docker-compose.yml` i el `.env.compose` adaptats a la teva infraestructura:

```bash
curl -fsSL https://raw.githubusercontent.com/agustim/chillgroup2/main/setup-deploy.sh -o setup-deploy.sh
bash setup-deploy.sh
```

El wizard configura la base de dades, LiveKit, S3 i els secrets automàticament. També inclou opcions d'**HTTPS integrat** (Caddy amb Let's Encrypt o Cloudflare Tunnel) — necessari per accedir des de màquines remotes, ja que el navegador requereix HTTPS per usar l'API de criptografia.

Guia completa: [Deploy amb Docker](/ca/deploy-docker).

## Per on comencar

- Si vols entendre el producte, comenca per [Overview](/ca/reference/OVERVIEW).
- Si vols revisar l'arquitectura, ves a [Architecture](/ca/reference/ARCHITECTURE).
- Si vols desplegar o contribuir, mira [Development](/ca/reference/DEVELOPMENT).

## Notes sobre idiomes

La referencia completa neix ara mateix de la documentacio existent en catala. La seccio anglesa del portal ofereix una entrada equivalent i es pot anar ampliant per traduir les peces mes importants de forma incremental.