# ChillGroup — Client Natiu Rust

## Visió General

Client d'escriptori natiu escrit en Rust per a Linux, Windows i macOS. Substitueix el client Electron (Linux) i complementa Tauri (macOS/Windows) amb una alternativa lleugera sense webview embegut.

**Motivació**: Electron ocupa ~150 MB de RAM i ~200 MB de disc. El client natiu objectiu: <30 MB RAM en repòs, <20 MB executable.

---

## Tecnologia

| Component | Crate | Motiu |
|-----------|-------|-------|
| UI | `slint` | Declaratiu (`.slint`), natiu, cross-platform, ideal per panells/llistes |
| HTTP | `reqwest` | Ja al workspace, suport async |
| Socket.IO | `rust-socketio` (async) | Compatible amb `socketioxide` del servidor |
| Audio/Vídeo/Pantalla | `livekit` 0.7 | Provat al POC: E2EE ✅ Linux ✅ Windows ✅ macOS ✅ |
| E2EE missatges | `ml-kem` + `aes-gcm` | Ja al workspace shared crate |
| Vault local | `sled` | BD clau-valor embeguda, sense servidor |
| Configuració | `serde` + `toml` | Fitxer de configuració llegible |
| Notificacions | `notify-rust` | Natiu a Linux/Windows/macOS |
| Tray icon | `tray-icon` | Cross-platform, compatible amb `slint` |
| Async runtime | `tokio` | Ja al workspace |

---

## Estructura de Fitxers

```
chillgroup-client/
├── Cargo.toml
├── .cargo/
│   └── config.toml          # Flags per plataforma (-ObjC macOS, glib libs Linux, +crt-static Win — libwebrtc usa CRT estàtic /MT)
├── build.rs                  # Compilació de recursos slint
├── assets/
│   ├── icon.png              # Icona tray i finestra
│   └── sounds/
│       └── notification.wav
├── ui/
│   ├── main.slint            # Layout principal
│   ├── components/
│   │   ├── sidebar.slint     # Llista servidors + canals
│   │   ├── message-list.slint
│   │   ├── message-input.slint
│   │   ├── voice-room.slint  # Controls veu/vídeo/pantalla
│   │   └── settings.slint    # Pantalla configuració
│   └── theme.slint           # Colors, fonts, mides
└── src/
    ├── main.rs               # Entry point: tokio + slint event loop
    ├── app.rs                # AppState global, coordinació
    ├── api/
    │   ├── mod.rs            # Client HTTP base (reqwest)
    │   ├── auth.rs           # login, register, refresh token
    │   ├── servers.rs        # llista servidors, membres
    │   ├── channels.rs       # CRUD canals, posició
    │   └── messages.rs       # historial, enviar, adjunts
    ├── realtime/
    │   ├── mod.rs            # Connexió Socket.IO
    │   └── events.rs         # Handlers: new-message, channel-updated, etc.
    ├── voice/
    │   ├── mod.rs            # LiveKit Room lifecycle
    │   ├── e2ee.rs           # KeyProvider, shared key per canal
    │   ├── tracks.rs         # Publicar/subscriure audio, vídeo, pantalla
    │   └── devices.rs        # Llista micròfons/càmeres
    ├── crypto/
    │   ├── mod.rs            # Re-exporta del shared crate
    │   ├── vault.rs          # Xifrat/desxifrat del vault local
    │   └── channel_keys.rs   # Gestió claus E2EE per canal
    ├── storage/
    │   ├── mod.rs            # Inicialització sled DB
    │   ├── vault.rs          # Claus privades dispositiu (ML-KEM keypairs)
    │   └── settings.rs       # Configuració persistent
    ├── settings/
    │   ├── mod.rs            # Struct Settings + defaults
    │   └── paths.rs          # Resolució de paths per plataforma
    ├── notifications/
    │   └── mod.rs            # notify-rust, gestió preferències
    └── tray/
        └── mod.rs            # Tray icon, menú, events
```

---

## Configuració

### Fitxer de configuració

Ubicació per defecte: `~/.config/chillgroup/config.toml` (Linux/macOS) / `%APPDATA%\ChillGroup\config.toml` (Windows).

```toml
[server]
url = "https://chillgroup.example.com"

[vault]
path = "~/.config/chillgroup/vault.db"  # configurable

[notifications]
enabled = true
sound = true
mention_only = false

[appearance]
theme = "system"  # "light" | "dark" | "system"
```

La ruta del vault és independent de la config — permet tenir el vault en un directori sincronitzat (Nextcloud, etc.) mentre la configuració és local.

### Pantalla de configuració

Accessible des de:
- Menú tray → "Configuració"
- Drecera de teclat dins l'app

Seccions:
1. **Servidor**: URL, botó "Connectar i verificar"
2. **Vault**: ruta actual, botó "Canviar ubicació..." (selector de directori natiu)
3. **Notificacions**: activar/desactivar, so, només mencions
4. **Àudio/Vídeo**: selecció de micròfon, càmera, dispositiu de sortida
5. **Aparença**: tema clar/fosc/sistema

---

## Tray Icon

El client minimitza a la safata del sistema (no tanca) quan es tanca la finestra.

Menú tray:
```
[Icona ChillGroup]
  Obrir ChillGroup          ← mostra/oculta finestra
  ─────────────────
  Estat: En línia  ▶        ← submenu: En línia / Absent / No molestar
  ─────────────────
  Configuració...
  ─────────────────
  Sortir
```

Indicadors visuals:
- Icona normal: connectat
- Icona amb punt vermell: missatges no llegits o mencions
- Icona grisada: desconnectat

---

## Flux de Dades

```
┌─────────────────────────────────────────────────┐
│  Thread UI (slint — no és async)                │
│  AppWindow { sidebar, messages, voice, settings }│
└────────────────┬────────────────────────────────┘
                 │ Callbacks + ModelRc<T>
                 ▼
┌─────────────────────────────────────────────────┐
│  Bridge (src/app.rs)                            │
│  Arc<RwLock<AppState>>                          │
│  mpsc::Sender → backend                         │
│  mpsc::Receiver ← backend (events)              │
└────────────────┬────────────────────────────────┘
                 │ tokio::spawn
                 ▼
┌──────────────────────────────────────────────────┐
│  Backend async (tokio runtime)                   │
│  ├── ApiClient (reqwest)                         │
│  ├── SocketIoClient (rust-socketio)              │
│  └── VoiceClient (livekit)                       │
└──────────────────────────────────────────────────┘
```

El thread de slint és el thread principal. El backend corre en un runtime tokio separat. La comunicació és via canals `mpsc`.

---

## Vault Local

### Ubicació

| Plataforma | Ruta per defecte |
|------------|-----------------|
| Linux | `~/.config/chillgroup/vault.db` |
| macOS | `~/Library/Application Support/ChillGroup/vault.db` |
| Windows | `%APPDATA%\ChillGroup\vault.db` |

Configurable via `config.toml` → `[vault] path`.

### Contingut

El vault (`sled` DB) emmagatzema:
- Keypairs ML-KEM del dispositiu (clau privada xifrada)
- Claus simètriques de canal (xifrades amb la clau del dispositiu)
- Token JWT actual
- Cache de claus públiques d'altres dispositius

La clau del vault es deriva d'una passphrase de l'usuari via Argon2id. En el primer inici, es demana crear la passphrase. En arrencades posteriors, es demana per desbloquejar.

### Seguretat

- La clau privada ML-KEM **mai** surt del dispositiu en clar
- Si es mou el vault a una altra ubicació, cal la passphrase per desxifrar
- El vault és portable: copiar el fitxer a un altre dispositiu desbloqueja amb la mateixa passphrase

---

## E2EE — Integració LiveKit

El client natiu usa exactament el mateix mecanisme E2EE que el client web:

```rust
// Canal de veu amb E2EE
let key_provider = KeyProvider::with_shared_key(
    KeyProviderOptions::default(),
    channel_key,  // clau simètrica del canal, del vault
);
let mut opts = RoomOptions::default();
opts.encryption = Some(E2eeOptions {
    encryption_type: EncryptionType::Gcm,
    key_provider,
});
let (room, rx) = Room::connect(&livekit_url, &token, opts).await?;
```

Les claus de canal (E2EE) es recuperen del vault local, igual que fa el client web des de localStorage.

---

## Audio, Vídeo i Compartir Pantalla

### Linux
- Àudio: PipeWire / PulseAudio via LiveKit
- Vídeo: V4L2
- Pantalla: XDG Desktop Portal (PipeWire) — `ashpd` crate

### macOS
- Àudio: CoreAudio
- Vídeo: AVFoundation
- Pantalla: ScreenCaptureKit (macOS 12.3+) via LiveKit
- Flag obligatori: `-ObjC` al linker (categories Objective-C de WebRTC)

### Windows
- Àudio: WASAPI
- Vídeo: DirectShow / Media Foundation
- Pantalla: DXGI Desktop Duplication via LiveKit
- CRT: compilar amb `+crt-static` per compatibilitat amb `libwebrtc` precompilat

---

## Notificacions

`notify-rust` envia notificacions natives del SO per:
- Missatge nou en canal no visible
- Menció (`@username`)
- Algú entra/surt d'un canal de veu actiu

Les notificacions respecten:
- `[notifications] enabled = false` → cap notificació
- `[notifications] mention_only = true` → només mencions
- `[notifications] sound = false` → silenci

---

## Packaging i Distribució

Reutilitza la mateixa infraestructura que el client Electron:

| Format | Plataforma | Eina |
|--------|------------|------|
| AppImage | Linux | `cargo-bundle` o script manual |
| `.deb` | Debian/Ubuntu | `cargo-deb` |
| `.rpm` | Fedora/RHEL | `cargo-rpm` |
| `.pacman` | Arch Linux | `cargo-aur` o script |
| `.dmg` | macOS | `cargo-bundle` |
| `.msi` | Windows | `cargo-wix` |

El binari és estàtic (sense dependències dinàmiques extra) gràcies a `+crt-static` (Windows) i linkatge estàtic de `libwebrtc`.

---

## Flags de Compilació per Plataforma

`.cargo/config.toml` al directori del client:

```toml
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-args=-ObjC"]

[target.x86_64-apple-darwin]
rustflags = ["-C", "link-args=-ObjC"]

[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

---

## Lliçons de Compilació (del POC)

Problemes trobats i resolts durant el desenvolupament de `livekit-poc/`. Cada punt és un error real que apareixerà si no s'aplica la solució.

### macOS — Crash en inicialitzar WebRTC (`NSInvalidArgumentException`)

**Símptoma:**
```
+[NSString stringForAbslStringView:]: unrecognized selector sent to class
→ RTCVideoEncoderVP9 scalabilityModes → abort trap 6
```

**Causa:** `libwebrtc` precompilat usa categories Objective-C. El linker per defecte no les carrega des de biblioteques estàtiques.

**Solució:** flag `-ObjC` al linker — força carregar totes les categories ObjC de les `.a` estàtiques.

```toml
# .cargo/config.toml
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-args=-ObjC"]

[target.x86_64-apple-darwin]
rustflags = ["-C", "link-args=-ObjC"]
```

Documentat al README oficial de `rust-sdks` de LiveKit.

---

### Windows — Error de linkatge CRT (`LNK2038`)

**Símptoma:**
```
LNK2038: mismatch detected for 'RuntimeLibrary':
  value 'MT_StaticRelease' doesn't match 'MD_DynamicRelease'
LNK1169: one or more multiply defined symbols found
```

**Causa:** `libwebrtc` precompilat per Windows és compilat amb `/MT` (CRT estàtic). Rust per defecte usa `/MD` (CRT dinàmic). El linker MSVC no permet barrejar-los.

**Solució:** forçar Rust a usar `/MT` també:

```toml
# .cargo/config.toml
[target.x86_64-pc-windows-msvc]
rustflags = ["-C", "target-feature=+crt-static"]
```

**No funciona:** `/NODEFAULTLIB:libcmt.lib` — elimina la biblioteca però no resol el mismatch de CRT.

---

### Windows — Error de certificat TLS (`UnknownIssuer`)

**Símptoma:**
```
client error (Connect): invalid peer certificate: UnknownIssuer
```

**Causa:** la feature `rustls-tls-native-roots` usa `rustls-native-certs` per llegir el cert store del SO. A Windows, la integració amb el cert store del sistema via rustls és incompleta en alguns entorns.

**Solució:** usar `native-tls` que delega al TLS natiu de cada SO (SChannel a Windows, SecureTransport a macOS, OpenSSL a Linux):

```toml
# Cargo.toml
livekit = { version = "0.7", features = ["native-tls"] }
livekit-api = { version = "0.5", features = ["native-tls", "access-token"] }
```

`native-tls` funciona correctament als 3 sistemes sense configuració extra.

---

### Linux (Arch) — Warnings de binutils 2.46.0

**Símptoma:**
```
as: BFD (GNU Binutils) 2.46.0 assertion fail
/usr/src/debug/binutils/binutils-gdb/bfd/elf.c:3571
```

**Causa:** bug del linker `as` de binutils 2.46.0 d'Arch Linux amb alguns objectes C++ de WebRTC.

**Impacte:** cap — són `warning`, no `error`. El binari final és correcte i funcional. Es poden ignorar.

---

### Linux — Espai en disc durant la compilació

`webrtc-sys` i `libwebrtc` requereixen ~3-4 GB d'espai temporal durant la compilació (objectes C++ intermedis). Si el disc és quasi ple, el linker falla amb:

```
LLVM ERROR: IO failure on output stream: No space left on device
```

**Solució:** especificar un directori `target` en una partició amb espai suficient:

```bash
CARGO_TARGET_DIR=/tmp/client-target cargo build --release
```

O netejar la caché de compilació: `cargo clean`.

---

### API de `livekit` 0.7 — Notes d'ús

**`RoomOptions` és `#[non_exhaustive]`** — no es pot construir amb struct literal des de crates externes:

```rust
// ❌ No compila
let opts = RoomOptions { encryption: Some(...), ..Default::default() };

// ✅ Correcte
let mut opts = RoomOptions::default();
opts.encryption = Some(E2eeOptions { ... });
```

**Camp `e2ee` deprecated** — usar `encryption` en comptes:

```rust
// ❌ Deprecated (warning)
opts.e2ee = Some(...);

// ✅ Correcte
opts.encryption = Some(...);
```

**`DataPacket` — camp `reliable` en comptes de `kind`:**

```rust
room.local_participant().publish_data(DataPacket {
    payload: b"data".to_vec(),
    reliable: true,          // ✅
    ..Default::default()
}).await?;
```

**`KeyProvider` per E2EE amb shared key:**

```rust
use livekit::e2ee::key_provider::{KeyProvider, KeyProviderOptions};
use livekit::e2ee::{E2eeOptions, EncryptionType};

let key_provider = KeyProvider::with_shared_key(
    KeyProviderOptions::default(),
    channel_key_bytes.to_vec(),  // 32 bytes recomanat
);
let mut opts = RoomOptions::default();
opts.encryption = Some(E2eeOptions {
    encryption_type: EncryptionType::Gcm,
    key_provider,
});
```

---

## Compatibilitat Verificada (POC)

| Plataforma | Connexió LiveKit | E2EE GCM | Data packet |
|------------|-----------------|----------|-------------|
| Linux x64 (Ubuntu 22.04) | ✅ | ✅ | ✅ |
| Windows x64 | ✅ | ✅ | ✅ |
| macOS ARM (M-series, macOS 15) | ✅ | ✅ | ✅ |

POC disponible a `livekit-poc/` — codi de referència per la integració inicial.

---

## Fases d'Implementació

### Fase 1 — Connexió i text
- Autenticació (login/register)
- Llista servidors i canals
- Missatgeria de text (enviar/rebre via Socket.IO)
- E2EE missatges (desxifrat des del vault)
- Tray icon bàsic

### Fase 2 — Veu
- Entrar/sortir canal de veu
- Àudio bidireccional amb LiveKit
- E2EE veu (shared key per canal)
- Controls: mute micro, mute sortida
- Llista participants en canal de veu

### Fase 3 — Vídeo i pantalla
- Publicar càmera
- Compartir pantalla (PipeWire / ScreenCaptureKit / DXGI)
- Finestra flotant participants vídeo

### Fase 4 — Poliment
- Notificacions natives
- Pantalla de configuració completa (inclou selector vault)
- Selecció dispositius àudio/vídeo
- Tema clar/fosc/sistema
- Auto-update (opcional)
