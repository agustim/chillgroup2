# ChillGroup v2 — Especificació del Frontend

## Stack del Frontend

| Tecnologia | Versió | Propòsit |
|------------|--------|----------|
| **Framework** | React 19 + TypeScript | UI |
| **Build tool** | Vite 5 | Compilació ràpida, HMR |
| **Comunicació temps real** | Socket.IO client 4.x | Missatges, presència, events |
| **Àudio/Vídeo** | livekit-client 2.x | WebRTC SFU amb E2EE |
| **Criptografia client** | Web Crypto API + @noble/post-quantum | AES-GCM, Kyber-1024 KEM |
| **Estils** | CSS pur amb CSS Variables | Sense dependències UI |
| **Ícones** | Emoji + Unicode symbols | Sense llibreries externes |
| **Build** | Node 20+ | Vite build |

## Variables d'Entorn del Frontend

El frontend utilitza el fitxer `.env` de l'arrel del projecte com a font única de configuració compartida amb el backend.

Variables rellevants:

- `LIVEKIT_HOST`: URL del servidor LiveKit utilitzada per `useLiveKit`
- `OPEN_REGISTER`: Controla si la pantalla de login mostra registre o només accés per invitació/admin
- `FRONTEND_DEBUG`: Nivell de logging del frontend

Detall tècnic:

- Vite no exposa automàticament variables sense prefix `VITE_` al codi client.
- Per aquest motiu, `frontend/vite.config.ts` carrega el `.env` de l'arrel i injecta constants de compilació (`__LIVEKIT_HOST__`, `__OPEN_REGISTER__`, `__FRONTEND_DEBUG__`).
- No s'ha de mantenir un `frontend/.env` duplicat per aquestes claus, per evitar desincronització.
- Les variables del frontend es resolen en build-time (durant `vite build`), no en runtime del binari.
- Canviar variables de frontend després de compilar requereix reconstruir el frontend (o el binari si es distribueix en mode embedded).

## Adjunts Xifrats (estat actual)

Flux implementat al client:

1. El composer encripta el fitxer localment amb `AES-GCM`.
2. Fa multipart upload del ciphertext (directe a RustFS o via proxy backend, segons `SERVER_PROXY_S3`).
3. Desa metadades criptografiques (`fileIv`, `wrappedFileKey`, `ciphertextSha256`, `keyVersionId`, `keyVersion`).
4. Quan es renderitza el missatge, el client recupera metadades de `/attachments/:id/download` per mostrar nom/mida.
5. En fer clic, descarrega el blob xifrat, el desxifra al navegador i nomes llavors desa el fitxer original.

Comportament UX esperat:

- No hi ha descàrrega automatica en render.
- Es mostra titol/mida de l'adjunt i la descàrrega s'activa nomes al clic.

### Thumbnails d'imatges

Per adjunts d'imatge (`image/*`), el client genera i puja un thumbnail **abans** de pujar el fitxer original.

**Generació (client-side, `generateThumbnail` a `lib/attachments.ts`):**
- Només per `file.type.startsWith('image/')`.
- Usa `createImageBitmap` + `OffscreenCanvas` per redimensionar.
- Màxim 200×200 px (manté aspect ratio).
- Exporta com `image/jpeg` amb qualitat 0.7.
- Si falla (format no suportat, etc.), continua sense thumbnail.

**Flux d'upload del thumbnail:**
1. `generateThumbnail(file)` → `Blob | null`
2. Si no és null, crea `File` amb nom `thumb_{originalName}` i `type: 'image/jpeg'`.
3. Puja via `uploadEncryptedAttachment` (mateix flux multipart xifrat), **sense** `thumbnailAttachmentId`.
4. Guarda el `thumbnailAttachmentId` resultant.
5. Puja el fitxer original amb `thumbnailAttachmentId` inclòs al `complete` request.

**Emmagatzematge:**
- El thumbnail és un adjunt independent a la taula `attachments`, xifrat amb la mateixa clau de canal.
- L'adjunt original té `thumbnail_attachment_id` → FK self-referencing a `attachments`.
- El thumbnail NO té `thumbnail_attachment_id` (evita recursió).

**Visualització (`MessageList.tsx`):**
- En render, detecta adjunts amb `thumbnailAttachmentId`.
- Per cada un, crida `attachmentGetDownload(channelId, thumbId)` → `decryptAttachmentToBlob` → `URL.createObjectURL`.
- Blob URLs es guarden a l'estat local `thumbnailBlobUrls` (clau: `attachmentId` principal).
- Es mostren inline al missatge com a previsualització clicable; en clicar es descarrega el fitxer original.
- En desmuntar el component, es criden `URL.revokeObjectURL` per alliberar memòria.

Detall important en mode proxy:

- Si `downloadUrl` es `/api/.../download-proxy`, la descàrrega s'ha de fer via `fetch` amb `Authorization: Bearer ...`.
- Obrir `downloadUrl` directament pot donar error de fitxer no disponible per manca de token.

Detall important en mode directe:

- Si `SERVER_PROXY_S3=false`, el bucket RustFS ha de tenir CORS configurat per l'origen del frontend.

## Estructura de Directoris

```
frontend/
├── package.json
├── tsconfig.json
├── vite.config.ts
├── index.html
├── public/
│   ├── favicon.svg
│   └── icons/
│       └── .gitkeep
├── src/
│   ├── main.tsx                    # Entry point
│   ├── App.tsx                     # Component arrel
│   ├── vite-env.d.ts               # Vite type declarations
│   │
│   ├── styles/
│   │   ├── variables.css           # CSS variables (tema dark/light)
│   │   ├── reset.css               # Normalize/reset
│   │   ├── layout.css              # Grid, flex, spacing
│   │   ├── sidebar.css             # Sidebar, canals, usuaris
│   │   ├── messages.css            # Missatges, input
│   │   ├── voice.css               # Voice area, video tiles
│   │   ├── modals.css              # Modals, forms
│   │   └── theme-light.css         # Override per tema clar
│   │
│   ├── contexts/
│   │   ├── AuthContext.tsx         # Autenticació + claus criptogràfiques
│   │   └── ChillGroupContext.tsx   # Estat global de l'app (serveis, canals, missatges)
│   │
│   ├── hooks/
│   │   ├── useAuth.ts              # Login, registre, claus dispositiu
│   │   ├── useChillGroup.ts        # Estat principal (canals, missatges, connexió)
│   │   ├── useMessages.ts          # Historial, enviar, editar, eliminar
│   │   ├── useChannels.ts          # CRUD canals, canviar entre canals
│   │   ├── useServers.ts           # CRUD servidors, membres
│   │   ├── useVoice.ts             # Connexió LiveKit, mic, càmera, pantalla
│   │   ├── useSocketIO.ts          # Socket.IO connect, events, reconnect
│   │   ├── usePresence.ts          # Usuaris connectats, veus actives
│   │   ├── useChannelKey.ts        # Obtenir/fer caché de claus de canal (E2EE)
│   │   └── useLocalStorage.ts      # Wrapper localStorage/IndexedDB
│   │
│   ├── lib/
│   │   ├── api.ts                  # Client HTTP (fetch wrapper)
│   │   ├── crypto.ts               # AES-GCM encrypt/decrypt
│   │   ├── crypto-e2ee.ts          # Kyber-1024 KEM encrypt/decapsulate
│   │   ├── socket.ts               # Socket.IO connect, events
│   │   ├── livekit.ts              # LiveKit room connect, token
│   │   └── storage.ts              # IndexedDB wrapper (claus, canals)
│   │
│   ├── components/
│   │   ├── LoginScreen.tsx         # Pantalla login/registre
│   │   ├── AppLayout.tsx           # Layout principal (sidebar + main)
│   │   │
│   │   ├── sidebar/
│   │   │   ├── ServerBar.tsx       # Barra vertical servidors (esquerra extrema)
│   │   │   ├── ServerPanel.tsx     # Dropdown servidors
│   │   │   ├── ChannelList.tsx     # Llista canals (text + veu)
│   │   │   ├── ChannelItem.tsx     # Un canal (text o veu)
│   │   │   ├── VoiceChannelUsers.tsx # Llista usuaris en un canal de veu
│   │   │   ├── UsersPanel.tsx      # Panell usuaris globals
│   │   │   └── ChannelSettings.tsx # Configuració canal
│   │   │
│   │   ├── main/
│   │   │   ├── MainContent.tsx     # Bloc principal (missatges O veu)
│   │   │   ├── ChannelHeader.tsx   # Capçalera canal (nom, tipus, encriptació)
│   │   │   ├── MessageList.tsx     # Llista missatges historics
│   │   │   ├── MessageBubble.tsx   # Un missatge individual
│   │   │   ├── MessageInput.tsx    # Input missatges de text
│   │   │   ├── VoiceArea.tsx       # Àrea veu (LiveKit + participants)
│   │   │   └── VoiceParticipants.tsx # Grid usuaris veu + controls
│   │   │
│   │   ├── modals/
│   │   │   ├── NewChannelModal.tsx # Form creació canal
│   │   │   ├── InviteModal.tsx     # Convidar usuari
│   │   │   ├── ServerModal.tsx     # Crear/Editar servidor
│   │   │   ├── EncryptionModal.tsx # Explicar encriptació canal
│   │   │   └── UserProfileModal.tsx # Perfil usuari
│   │   │
│   │   └── shared/
│   │       ├── Button.tsx          # Botons reutilitzables
│   │       ├── Spinner.tsx         # Loading indicator
│   │       ├── Badge.tsx           # Badges (rol, estat)
│   │       ├── Tooltip.tsx         # Tooltip
│   │       ├── EncryptionIcon.tsx  # Icona nivell encriptació
│   │       └── ConnectionStatus.tsx # Estat connexió
│   │
│   └── types/
│       ├── index.ts                # Types TypeScript globals
│       └── permissions.ts          # Rols i permisos
```

## Layout Principal

### Distribució

```
┌────────┬────────────┬─────────────────────────────────┐
│ Server │ Channel  │ Channel Header                    │
│  Bar   │  List    │  # general  [🔑E2EE] [⚙️] [👥]  │
│        ├────────────┼─────────────────────────────────┤
│ [🏠]   │ # general  │ ──────────────────────────────  │
│ [🎮]   │ # misc     │   Missatge 1                    │
│ [+]    │ 🔊 veu1    │   Missatge 2 (encriptat)        │
│        │ 🔊 veu2    │   Missatge 3                    │
│        ├────────────┤   ...                             │
│        │ ───────    │ ──────────────────────────────  │
│        │ Usuaris:   │ [🎤] [📷] [🖥️] [🚪 Deixa]      │
│        │ • agusti   │                                  │
│        │ • marcus   │                                  │
│        ├────────────┤                                  │
│        │ [Què       │ Message Input: [___________][📤]│
│        │  pensen?] │                                  │
│        └────────────┴─────────────────────────────────┘
└────────┴────────────┴─────────────────────────────────┘
```

| Bloc | Amplada | Funció |
|------|---------|--------|
| **Server Bar** | 72px | Barra vertical amb icones de servidors |
| **Channel List** | 240px | Canals de text i veu + panell d'usuaris |
| **Main Content** | flex:1 | Missatges (text) O sala de veu (àudio/vídeo) |

### Comportaments Clau

#### Canals de Text
- **Click = mostrar missatges**, no entra/surt
- El canal seleccionat es destaca a la llista
- Els missatges històrics es carreguen al bloc principal
- No hi ha "entrar" ni "sortir" — sempre pots veure els missatges
- La roda dentada de configuració del canal només es mostra si l'usuari té `permissionLevel >= 3` (`manage`)

#### Configuració integrada de permisos de canal
- La pantalla integrada de configuració mostra permisos efectius per usuari
- També mostra l'origen del permís amb etiqueta visual:
  - `heretat` (sense override explícit)
  - `explícit` (override a `channel_members.permission_level`)
- L'admin/manager pot canviar l'override per usuari (`read`/`write`/`manage`) o tornar-lo a `heretat`

#### Canals de Veu
- **Click = connectar/desconnectar** a la sala de veu
- Només pots estar en **UN sol canal de veu** alhora
- Si ja estàs en un canal de veu i en selecciones un altre, **surt automàticament** del primer
- El canal actiu mostra una indicació visual (fons verd, icona animada)
- Els usuaris dins del canal es mostren a la llista de canals

#### Controls de Veu (barra inferior del sidebar)

| Botó | Icona | Acció |
|------|-------|-------|
| Micròfon | 🎤 | Toggle mute/unmute local |
| Altaveu | 🔊 | Toggle deafen (silencia tots els remots) |
| Càmera | 🎥 | Toggle càmera local |
| Pantalla | 🖥️ | Toggle screen share |
| Fitxer media | 🎬 | Compartir fitxer d'àudio o vídeo al canal |

#### Compartir Fitxer de Media
- Clic a 🎬 obre el selector de fitxers del sistema operatiu (`audio/*`, `video/*`)
- El fitxer es reprodueix localment i l'stream s'envia als altres membres via LiveKit
- El propietari veu un **reproductor flotant** (`MediaFilePlayer`) a la VoiceArea amb:
  - Preview de vídeo (si el fitxer és vídeo)
  - Barra de seek + temps actual/total
  - Botó play/pause
  - Botó mute local (silencia la reproducció local sense aturar l'stream)
  - Botó ✕ per aturar i tancar
- Si el fitxer és àudio sense vídeo, apareix igualment un **tile** a la graella de participants amb badge `FILE`
- Quan el fitxer acaba, l'share s'atura automàticament
- Mecanisme tècnic: `HTMLVideoElement.captureStream()` → `LocalVideoTrack` + `LocalAudioTrack` → LiveKit `publishTrack`

#### Silenciar Streams Localment
Qualsevol tile de participant (remot o fitxer media local) té un botó 🔊/🔇 al costat de 📌 i ⛶:
- **Participant remot**: muta el seu `HTMLAudioElement` local (no afecta l'stream)
- **Tile FILE local**: muta la reproducció local del fitxer (equivalent al botó del `MediaFilePlayer`)

## Sistema de Temes (Dark/Light)

### CSS Variables — Tema Dark (Per defecte)

```css
/* styles/variables.css */
:root {
  /* Colors de fons */
  --bg-app: #1a1b1e;          /* Aplicació principal */
  --bg-sidebar: #2b2d31;      /* Sidebar canals */
  --bg-serverbar: #1e1f22;    /* Barra servidors */
  --bg-header: #2b2d31;       /* Capçaleres */
  --bg-input: #383a40;        /* Inputs */
  --bg-hover: #35373c;        /* Hover */
  --bg-active: #404249;       /* Actiu/seleccionat */
  --bg-message-hover: #2f3136; /* Hover missatge */
  --bg-modal: rgba(0,0,0,0.7); /* Overlay modal */
  --bg-tile: #1e1f22;         /* Video tiles */
  --bg-badge: #404249;        /* Badges */

  /* Colors de text */
  --text-primary: #dbdee1;     /* Text principal */
  --text-secondary: #949ba4;   /* Text secundari */
  --text-muted: #6d7177;      /* Text atenuat */
  --text-link: #00a8fc;       /* Links */
  --text-on-accent: #ffffff;  /* Text sobre accent */

  /* Colors d'accent */
  --accent: #00a8fc;          /* Accent primari (blau) */
  --accent-hover: #0097e6;    /* Accent hover */
  --accent-active: #0084cc;   /* Accent actiu */

  /* Colors d'estat */
  --success: #23a559;         /* Verd (connectat, OK) */
  --warning: #f0b232;         /* Taronja (warning) */
  --error: #f23f43;           /* Vermell (error) */
  --offline: #80848c;         /* Gris (offline) */
  --dnd: #f23f43;             /* No molestar */
  --idle: #f0b232;            /* Idle */
  --online: #23a559;          /* Online */

  /* Colors de criptografia */
  --crypto-none: #949ba4;     /* Sense encriptació (gris) */
  --crypto-symmetric: #00a8fc; /* Simètrica (blau) */
  --crypto-asymmetric: #23a559; /* Asimètrica (verd) */

  /* Colors de veu */
  --voice-speaking: #f0b232;  /* Parlant */
  --voice-muted: #f23f43;     /* Mic mut */
  --voice-border: #404249;    /* Bordes */

  /* Bordes */
  --border: #3f3f44;
  --border-light: #505055;

  /* Tipografia */
  --font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
  --font-size-xs: 12px;
  --font-size-sm: 13px;
  --font-size-base: 14px;
  --font-size-md: 15px;
  --font-size-lg: 16px;
  --font-size-xl: 20px;
  --font-size-xxl: 24px;
  --font-weight-regular: 400;
  --font-weight-medium: 500;
  --font-weight-semibold: 600;
  --font-weight-bold: 700;

  /* Espaiat */
  --spacing-xs: 4px;
  --spacing-sm: 8px;
  --spacing-md: 12px;
  --spacing-lg: 16px;
  --spacing-xl: 24px;
  --spacing-xxl: 32px;

  /* Radis */
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-full: 9999px;

  /* Transicions */
  --transition-fast: 150ms ease;
  --transition-normal: 200ms ease;
  --transition-slow: 300ms ease;

  /* Ombres */
  --shadow-sm: 0 1px 3px rgba(0,0,0,0.3);
  --shadow-md: 0 4px 12px rgba(0,0,0,0.4);
  --shadow-lg: 0 8px 30px rgba(0,0,0,0.5);
}
```

### CSS Variables — Tema Light (Override)

```css
/* styles/theme-light.css */
[data-theme="light"] {
  --bg-app: #f0f2f5;
  --bg-sidebar: #ffffff;
  --bg-serverbar: #f8f9fa;
  --bg-header: #ffffff;
  --bg-input: #e9ecef;
  --bg-hover: #e8eaed;
  --bg-active: #dde0e6;
  --bg-message-hover: #e8eaed;
  --bg-modal: rgba(0,0,0,0.4);
  --bg-tile: #ffffff;
  --bg-badge: #e9ecef;

  --text-primary: #1a1a1a;
  --text-secondary: #65676b;
  --text-muted: #8a8d91;
  --text-link: #0066cc;
  --text-on-accent: #ffffff;

  --accent: #0066cc;
  --accent-hover: #0055bb;
  --accent-active: #004499;

  --success: #1a8d48;
  --warning: #d97706;
  --error: #dc2626;
  --offline: #9ca3af;
  --dnd: #dc2626;
  --idle: #d97706;
  --online: #1a8d48;

  --crypto-none: #65676b;
  --crypto-symmetric: #0066cc;
  --crypto-asymmetric: #1a8d48;

  --voice-speaking: #d97706;
  --voice-muted: #dc2626;
  --voice-border: #d1d5db;

  --border: #e0e0e0;
  --border-light: #d1d5db;

  --shadow-sm: 0 1px 3px rgba(0,0,0,0.08);
  --shadow-md: 0 4px 12px rgba(0,0,0,0.1);
  --shadow-lg: 0 8px 30px rgba(0,0,0,0.15);
}
```

### Selector de Tema

```tsx
// components/shared/ThemeToggle.tsx
const themes = ['dark', 'light'] as const
type Theme = typeof themes[number]

export const ThemeToggle: React.FC = () => {
  const [theme, setTheme] = useState<Theme>(() => {
    return (localStorage.getItem('chillgroup-theme') as Theme) || 'dark'
  })

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme)
    localStorage.setItem('chillgroup-theme', theme)
  }, [theme])

  const next = () => {
    const i = themes.indexOf(theme)
    setTheme(themes[(i + 1) % themes.length])
  }

  return (
    <button onClick={next} title="Canvia el tema" aria-label="Canvia el tema">
      {theme === 'dark' ? '☀️' : '🌙'}
    </button>
  )
}
```

## Fonts i Tipografia

### Font Principal — Inter

Inter és la font per defecte, carregada via Google Fonts o font-display swap:

```html
<!-- index.html -->
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
```

Fallback stack:
```css
--font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
```

### Fonts de Codi (per missatges encriptats)

Els missatges encriptats es mostren amb monospace per facilitar la còpia:
```css
font-family: 'JetBrains Mono', 'SF Mono', 'Fira Code', 'Courier New', monospace;
```

Carregada opcional:
```html
<link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
```

### Jerarquia Tipogràfica

| Rols | Font-size | Weight | Ús |
|------|-----------|--------|-----|
| Header canal | 16px | 600 | Nom del canal |
| Text missatge | 14px | 400 | Contingut del missatge |
| Autor missatge | 14px | 600 | Nom de l'usuari |
| Timestamp | 11px | 400 | Hora del missatge |
| Nom canal llista | 14px | 400 | Item de canal |
| Canal actiu | 14px | 500 | Canal seleccionat |
| Usuari llista | 13px | 400 | Usuari individual |
| Text secundari | 12px | 400 | Labels, badges |
| Secció header | 11px | 500 | "CANALS", "USUARIS" |

## Ícones i Grafics

### Estratègia: Emojis + Unicode (Sense Dependències)

El projecte utilitza emojis i caràcters Unicode com a ícones. Això elimina la necessitat de llibreries d'ícones externes i redueix la mida del bundle.

| Símbol | Ús | Component |
|--------|-----|-----------|
| `#` | Canal de text | `ChannelItem.tsx` |
| `🔊` | Canal de veu (inactiu) | `ChannelItem.tsx` |
| `🔴` | Canal de veu (actiu, parlant) | `ChannelItem.tsx` |
| `🎤` | Micròfon control | `VoiceControls.tsx` |
| `📷` | Càmera control | `VoiceControls.tsx` |
| `🖥️` | Screen share control | `VoiceControls.tsx` |
| `🚪` | Sortir de veu | `VoiceControls.tsx` |
| `⚙️` | Configuració | `ChannelHeader.tsx` |
| `👥` | Usuaris / Convidar | `ChannelHeader.tsx` |
| `🔑` | Encriptació activada | `ChannelHeader.tsx` |
| `🔒` | Encriptació E2EE | `ChannelHeader.tsx` |
| `🔓` | Sense encriptació | `ChannelHeader.tsx` |
| `✏️` | Editar missatge | `MessageBubble.tsx` |
| `🗑️` | Eliminar missatge | `MessageBubble.tsx` |
| `↩️` | Respondre | `MessageBubble.tsx` |
| `📎` | Adjunt | `MessageInput.tsx` |
| `📤` | Enviar | `MessageInput.tsx` |
| `⚠️` | Warning | Banner E2EE |
| `🏠` | Servidor principal | `ServerBar.tsx` |
| `+` | Afegir servidor | `ServerBar.tsx` |
| `●` | Estat usuari (online) | `UsersPanel.tsx` |
| `◐` | Estat usuari (idle) | `UsersPanel.tsx` |
| `○` | Estat usuari (offline) | `UsersPanel.tsx` |
| `▶️` | Parlant (speaking) | `VoiceParticipants.tsx` |
| `🔇` | Mut | `VoiceParticipants.tsx` |
| `↔️` | Canviar tema | `ThemeToggle.tsx` |
| `👤` | Perfil | `UserProfileModal.tsx` |
| `⚡` | Admin | `Badge.tsx` |

### Icona de Nivell de Criptografia

```tsx
// components/shared/EncryptionIcon.tsx

interface EncryptionIconProps {
  type: 'none' | 'symmetric' | 'asymmetric'
  size?: 'sm' | 'md' | 'lg'
}

export const EncryptionIcon: React.FC<EncryptionIconProps> = ({ type, size = 'md' }) => {
  const icons = {
    none: { emoji: '🔓', label: 'Sense encriptació', color: '--crypto-none' },
    symmetric: { emoji: '🔑', label: 'Clau compartida', color: '--crypto-symmetric' },
    asymmetric: { emoji: '🔒', label: 'E2EE — Zero Knowledge', color: '--crypto-asymmetric' },
  }

  const icon = icons[type]

  return (
    <span
      title={icon.label}
      style={{
        fontSize: size === 'sm' ? '14px' : size === 'md' ? '16px' : '20px',
        filter: 'grayscale(0.3)'
      }}
    >
      {icon.emoji}
    </span>
  )
}
```

## Storage Local (localStorage + IndexedDB)

### localStorage (Dades No Sensibles)

```
┌──────────────────────────────────────────────────────────┐
│                    localStorage                          │
├──────────────────────┬───────────────────────────────────┤
│ Clau                 │ Valor                             │
├──────────────────────┼───────────────────────────────────┤
│ chillgroup-theme     │ 'dark' | 'light'                  │
│ chillgroup-username  │ 'agusti' (només per UI display)   │
│ chillgroup-server    │ 'uuid-ultim-servidor'             │
│ chillgroup-deviceId  │ UUID generat localment            │
│ chillgroup-local-vault-meta │ Metadata del vault local    │
└──────────────────────┴───────────────────────────────────┘
```

**Regles:**
- `chillgroup-username` és **només per UI** — mai per autenticació
- `chillgroup-deviceId` és un UUID generat localment (un per navegador)
- `chillgroup-theme` es canvia amb el selector de tema
- **JWT NO es guarda a localStorage** — es guarda a `Cookie: HttpOnly` o `sessionStorage`
- `chillgroup-local-vault-meta` guarda només metadata criptogràfica (salt/verifier), no claus de canal

### IndexedDB (Dades Sensibles — Claus Criptogràfiques)

```
┌──────────────────────────────────────────────────────────┐
│                     IndexedDB                            │
│              Store: chillgroup-store                     │
├──────────────────────┬───────────────────────────────────┤
│ Object Store         │ Key Path                          │
├──────────────────────┼───────────────────────────────────┤
│ keypairs             │ deviceId (UUID)                   │
│                      │ - kyberSecretKey: Uint8Array      │
│                      │ - createdAt: number (timestamp)   │
├──────────────────────┼───────────────────────────────────┤
│ channelKeys          │ channelId (UUID)                  │
│                      │ - aesKey: CryptoKey               │
│                      │ - acquiredAt: number              │
├──────────────────────┼───────────────────────────────────┤
│ channelKeysBytes     │ channelId (UUID)                  │
│                      │ - keyBytes: Uint8Array | null      │
│                      │ - keyCiphertext: string | null     │
│                      │ - type: 'symmetric' | 'asymmetric'│
│                      │ - expiresAt: number | null        │
├──────────────────────┼───────────────────────────────────┤
│ channelKeysByVersion │ compoundId channelId::version      │
│                      │ - keyBytes: Uint8Array | null      │
│                      │ - keyCiphertext: string | null     │
│                      │ - keyVersion, keyVersionId         │
│                      │ - type, acquiredAt, expiresAt      │
├──────────────────────┼───────────────────────────────────┤
│ devicePublicKeys     │ deviceId (UUID)                   │
│                      │ - publicKey: Uint8Array (1568b)   │
│                      │ - algorithm: 'Kyber-1024'         │
├──────────────────────┼───────────────────────────────────┤
│ livekitSessionKeys   │ sessionId (UUID)                  │
│                      │ - sessionKey: CryptoKey           │
│                      │ - channelChannelId                │
│                      │ - createdAt: number               │
└──────────────────────┴───────────────────────────────────┘
```

### Detall dels Object Stores

#### `keypairs` — Claus Criptogràfiques del Dispositiu

```typescript
interface KeyPairRecord {
  deviceId: string          // UUID del dispositiu
  kyberSecretKey: Uint8Array // Kyber-1024 secret key (3168 bytes)
  createdAt: number          // Timestamp de creació
}
```

**Regles:**
- Generat la primera vegada que l'usuari fa login
- **MAI s'envia al servidor**
- Es guarda a IndexedDB del navegador
- Si esborra el navegador, es perd → cal regenerar
- Si fa login en un altre dispositiu, té el seu propi keypair

#### `channelKeysBytes` — Claus de Canal (Caché)

```typescript
interface CachedChannelKey {
  channelId: string           // UUID del canal
  keyBytes: Uint8Array | null // Legacy en clar (compatibilitat)
  keyCiphertext: string | null // Valor xifrat en repòs (vault local)
  type: 'symmetric' | 'asymmetric'
  acquiredAt: number          // Quan es va obtenir
  expiresAt: number | null    // NULL = no expira
}
```

**Regles:**
- Cache de rendiment — la font de veritat és el servidor
- Per a canals **simètrics**: es demana al servidor i es desa aquí
- Per a canals **asimètrics**: es desencripta amb Kyber i es desa aquí
- Amb vault local actiu, el valor persistent és `keyCiphertext` (xifrat AES-GCM)
- S'expira si el canal té TTL (coincideix amb TTL del canal)
- Si no es troba a IndexedDB → es demana al servidor

#### `channelKeysByVersion` — Claus per Versió

```typescript
interface VersionedChannelKeyRecord {
  compoundId: string          // "channelId::keyVersion"
  channelId: string
  keyVersion: number
  keyVersionId: string | null
  keyBytes: Uint8Array | null
  keyCiphertext: string | null
  type: 'symmetric' | 'asymmetric'
  acquiredAt: number
  expiresAt: number | null
}
```

**Regles:**
- Store principal per lectura/escriptura de claus de canal
- Manté historial per `keyVersion`
- En migració, converteix entrades legacy en clar cap a `keyCiphertext`

#### `channelKeys` — Claus de Canal Web Crypto

```typescript
interface ChannelKeyRecord {
  channelId: string
  aesKey: CryptoKey          // CryptoKey importat des de bytes
  acquiredAt: number
}
```

**Regles:**
- Versió Web Crypto API de `channelKeysBytes`
- Permet fer `crypto.subtle.encrypt/decrypt` directament
- Es genera a partir de `channelKeysBytes` quan cal encriptar/desxifrar

#### `devicePublicKeys` — Claus Públiques del Dispositiu

```typescript
interface DevicePublicKeyRecord {
  deviceId: string
  publicKey: Uint8Array      // Kyber-1024 public key (1568 bytes)
  algorithm: 'Kyber-1024'
}
```

**Regles:**
- Copia local de la keypair (la pública)
- Sincronitzada amb el servidor via `PUT /api/user/me/devices/:deviceId/publicKey`

#### `livekitSessionKeys` — Claus de Sessió LiveKit (E2EE de Veu)

```typescript
interface LiveKitSessionKeyRecord {
  sessionId: string           // UUID
  sessionKey: CryptoKey       // AES-256 CryptoKey per LiveKit E2EE
  channelChannelId: string    // Canal de veu associat
  createdAt: number
}
```

**Regles:**
- Generada per cada session de veu
- Si el canal és E2EE, la session key es distribueix via canal de text
- Es guarda a IndexedDB per reutilitzar en reconexions

### Flux Login + Desbloqueig de Dispositiu

1. L'usuari inicia sessió amb `username + contrasenya`.
2. Si el client detecta vault local:
  1. mostra pantalla "Desbloqueja el dispositiu",
  2. sense desbloqueig no es poden usar claus locals.
3. Si no hi ha vault local:
  1. mostra pantalla de configuració de clau local,
  2. crea metadata del vault i activa xifrat en repòs.

### Logout: backup i neteja desacoblats

El modal de logout permet combinacions independents:

1. fer backup (xifrat o no),
2. esborrar dades locals o conservar-les xifrades.

Conservar dades locals implica desbloqueig local obligatori en el proper inici.

### Canvi de clau local

El panell de canvi de contrasenya inclou una segona secció per canviar la clau local:

1. valida clau local actual,
2. defineix clau local nova,
3. re-xifra claus de canal locals amb la nova clau.

## Hook: `useChillGroup` — Estat Global

```typescript
// hooks/useChillGroup.ts

export interface ChannelState {
  id: string
  name: string
  type: 'text' | 'voice'
  encryptionType: 'none' | 'symmetric' | 'asymmetric'
  messageTTL?: number
  members: number
}

export interface ChannelKeyCache {
  channelId: string
  key: string  // Base64 AES-256
}

export interface UseChillGroupReturn {
  // Estat de canals
  channels: ChannelState[]
  currentTextChannel: string | null   // ID del canal de text seleccionat
  currentVoiceChannel: string | null  // ID del canal de veu actiu (només 1)

  // Missatges
  messages: Message[]
  isLoadingMessages: boolean
  hasMoreMessages: boolean

  // Connexió
  connectionStatus: 'connected' | 'disconnected' | 'connecting' | 'error'
  connectionMessage: string

  // Veu
  micEnabled: boolean
  cameraEnabled: boolean
  screenShareEnabled: boolean
  currentRoom: LiveKitRoom | null

  // Servidors
  servers: ServerState[]
  currentServer: ServerState | null

  // Accions
  joinChannel: (channelId: string, type: 'text' | 'voice') => Promise<void>
  sendMessage: (channelId: string, text: string) => Promise<void>
  toggleMic: () => Promise<void>
  toggleCamera: () => Promise<void>
  toggleScreenShare: () => Promise<void>
  leaveVoice: () => Promise<void>
  switchServer: (serverId: string) => Promise<void>
}
```

### Lògica de `joinChannel`

```typescript
async function joinChannel(channelId: string, type: 'text' | 'voice') {
  // Si és canal de veu, sortir del canal de veu actual si existeix
  if (type === 'voice' && currentVoiceChannel && currentVoiceChannel !== channelId) {
    await leaveVoice() // Surt automàticament del canal anterior
  }

  if (type === 'text') {
    // Només canviar el canal de text seleccionat (mostra missatges)
    setCurrentTextChannel(channelId)
    // Carregar missatges històrics
    await loadMessages(channelId)
    // Si el canal és E2EE, obtenir la clau
    if (channelNeedsEncryption(channelId)) {
      await ensureChannelKey(channelId)
    }
  } else {
    // Canal de veu: connectar a LiveKit
    setCurrentVoiceChannel(channelId)
    await connectToLiveKit(channelId)
  }
}
```

## Components Clau

### `ChannelItem` — Un Element de la Llista de Canals

```tsx
// components/sidebar/ChannelItem.tsx

interface ChannelItemProps {
  channel: ChannelState
  isActive: boolean        // És el canal actual de text?
  isVoiceActive: boolean   // És el canal de veu actiu?
  voiceParticipants?: string[]  // Usuaris en aquest canal de veu
  onJoin: (channelId: string, type: 'text' | 'voice') => void
}

export const ChannelItem: React.FC<ChannelItemProps> = ({
  channel,
  isActive,
  isVoiceActive,
  voiceParticipants = [],
  onJoin,
}) => {
  const isTextActive = isActive && channel.type === 'text'
  const isVoiceActiveChannel = isVoiceActive && channel.type === 'voice'
  const isSelected = isTextActive || isVoiceActiveChannel

  if (channel.type === 'text') {
    return (
      <div
        className={`channel-item text-channel ${isSelected ? 'active' : ''}`}
        onClick={() => onJoin(channel.id, 'text')}
      >
        <span className="channel-icon">#</span>
        <span className="channel-name">{channel.name}</span>
        <EncryptionIcon type={channel.encryptionType} size="sm" />
      </div>
    )
  }

  // Canal de veu
  return (
    <div
      className={`channel-item voice-channel ${isSelected ? 'active' : ''}`}
      onClick={() => onJoin(channel.id, 'voice')}
    >
      <span className="channel-icon">{isVoiceActiveChannel ? '🔴' : '🔊'}</span>
      <span className="channel-name">{channel.name}</span>
      <EncryptionIcon type={channel.encryptionType} size="sm" />
      {/* Usuaris dins d'aquest canal de veu */}
      {isVoiceActiveChannel && voiceParticipants.length > 0 && (
        <div className="channel-users">
          {voiceParticipants.map(user => (
            <div key={user.id} className="channel-user">
              <span className="user-status">●</span>
              <span className="user-identity">{user.username}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
```

### `ChannelHeader` — Capçalera del Canal

```tsx
// components/main/ChannelHeader.tsx

interface ChannelHeaderProps {
  channelName: string
  channelType: 'text' | 'voice'
  encryptionType: 'none' | 'symmetric' | 'asymmetric'
  messageTTL?: number
  onInvite: () => void
  onSettings: () => void
}

export const ChannelHeader: React.FC<ChannelHeaderProps> = ({
  channelName,
  channelType,
  encryptionType,
  messageTTL,
  onInvite,
  onSettings,
}) => {
  return (
    <div className="channel-header">
      <div className="channel-header-left">
        <span className="channel-hash">
          {channelType === 'text' ? '#' : '🔊'}
        </span>
        <span className="channel-title">{channelName}</span>
        <EncryptionIcon type={encryptionType} size="md" />
        {messageTTL && (
          <span className="channel-ttl" title={`Missatges expiren en ${messageTTL}s`}>
            ⏱️
          </span>
        )}
      </div>
      <div className="channel-header-actions">
        <button className="invite-btn" onClick={onInvite} title="Convidar usuari">
          👥 Convidar
        </button>
        <button className="settings-btn" onClick={onSettings} title="Configuració">
          ⚙️
        </button>
      </div>
    </div>
  )
}
```

### `VoiceArea` — Àrea de Veu

`components/main/VoiceArea.tsx`

Props principals:

| Prop | Tipus | Descripció |
|------|-------|------------|
| `connection` | `VoiceConnection \| null` | Estat de la connexió activa |
| `localVideoTrack` | `any` | Track de càmera local |
| `localScreenTrack` | `any` | Track de screen share local |
| `localMediaFileTrack` | `any` | Track de vídeo del fitxer media (null si àudio-only) |
| `isMediaFileSharing` | `boolean` | Indica si s'està compartint un fitxer (inclou àudio-only) |
| `mediaFileName` | `string \| null` | Nom del fitxer que s'està compartint |
| `mediaFileElementRef` | `MutableRefObject<HTMLVideoElement \| null>` | Ref a l'element DOM per al reproductor |
| `onStopMediaFileShare` | `() => void` | Callback per aturar el file share |
| `onSetParticipantLocalMuted` | `(identity, muted) => void` | Mute local d'un participant remot |
| `remoteVideoTracks` | `Record<string, any[]>` | Tracks de vídeo remots per userId |

**Tiles de participants** — La graella construeix `ParticipantRenderItem[]` amb:
- Participant local (càmera si activa)
- Tile `SCREEN` local (si screen share actiu)
- Tile `FILE` local (si `isMediaFileSharing`, sense vídeo si és àudio-only)
- Un tile per cada track remot (badge `CAM` o `SCREEN` segons source)

**Modes de visualització:** `mosaic` (graella automàtica) | `focus` (un participant gran + strip lateral)

**Mute local per stream:** cada tile té botó 🔊/🔇. L'estat es gestiona amb `localMutedStreamIds: Set<string>` intern a `VoiceArea`.

### `MediaFilePlayer` — Reproductor Flotant

`components/main/MediaFilePlayer.tsx`

Overlay posicionat `absolute` a la cantonada inferior dreta de `VoiceArea`. Apareix quan `mediaFileName` és no-nul.

| Prop | Tipus | Descripció |
|------|-------|------------|
| `mediaFileElementRef` | `MutableRefObject<HTMLVideoElement \| null>` | Ref a l'element DOM font |
| `fileName` | `string` | Nom del fitxer (truncat a 28 chars) |
| `onStop` | `() => void` | Atura i tanca el reproductor |

Comportament intern:
- Mou l'element `<video>` DOM al contenidor del reproductor per mostrar el preview
- En desmuntar, retorna l'element al `document.body` (ocult) perquè el hook pugui netejar-lo
- Detecta si és vídeo via `el.videoWidth > 0` (event `loadedmetadata`)
- Controls: play/pause, seek (input range), mute local, aturar

### `MessageList` — Llista de Missatges

```tsx
// components/main/MessageList.tsx

interface MessageListProps {
  messages: Message[]
  onEdit: (id: string, text: string) => void
  onDelete: (id: string) => void
  onReply: (message: Message) => void
  channelKey?: string | null  // Base64, null si no és E2EE
  encryptionType: 'none' | 'symmetric' | 'asymmetric'
}

export const MessageList: React.FC<MessageListProps> = ({
  messages,
  onEdit,
  onDelete,
  onReply,
  channelKey,
  encryptionType,
}) => {
  // Si no tenim clau i el canal és E2EE, mostra missatges encriptats
  const canDecrypt = channelKey !== null && channelKey !== undefined

  return (
    <div className="messages-area">
      {messages.map((msg) => {
        let displayText = msg.encryptedPayload
        let isEncrypted = false

        if (encryptionType === 'none' || canDecrypt) {
          if (canDecrypt) {
            displayText = decryptMessageSync(channelKey!, msg.encryptedPayload, msg.iv)
          }
        } else {
          isEncrypted = true
        }

        return (
          <MessageBubble
            key={msg.id}
            author={msg.senderUsername}
            timestamp={msg.timestamp}
            text={displayText}
            isEncrypted={isEncrypted}
            isEdited={msg.editedAt !== null}
            onEdit={() => onEdit(msg.id, msg.text)}
            onDelete={() => onDelete(msg.id)}
            onReply={() => onReply(msg)}
          />
        )
      })}
    </div>
  )
}
```

## Flux de Missatges Complet

```
1. USUARI ESCRIBEIX
   └─> MessageInput.tsx captura text
   └─> onSend(channelId, text)

2. OBTENCIO DE CLAU (si E2EE)
   └─> useChannelKey.ts: getChannelKey(channelId)
  └─> IndexedDB (xifrada en repòs) + vault desbloquejat? → retorna directament
  └─> No → API GET /channels/:id/keys → desencripta → guarda a IndexedDB (xifrada en repòs)

3. ENCRYPTACIO (si canal necessita)
   └─> crypto.ts: encryptMessage(channelKey, text)
   └─> Genera IV aleatori (12 bytes)
   └─> AES-GCM-256 → encryptedPayload (Base64) + iv (Base64)

4. ENVIO AL SERVIDOR
   └─> POST /channels/:id/messages
   └─> { encryptedPayload, iv, timestamp }
   └─> Servidor guarda a DB

5. BROADCAST
   └─> Server → Socket.IO → "message" event
   └─> Tots els clients del canal reben el missatge

6. RECEPCIO
   └─> useSocketIO hook rep "message"
   └─> useMessages hook afegeix a messages[]
   └─> MessageList es renderitza amb missatge nou
  └─> Si E2EE → desencripta amb channelKey recuperada des de IndexedDB (requereix vault local desbloquejat)
```

## Flux de Compartir Fitxer de Media

```
1. USUARI CLICA 🎬 (sidebar bottom controls)
   └─> Si isMediaFileSharing → stopMediaFileShare() i end
   └─> Si no → obre <input type="file" accept="audio/*,video/*">

2. USUARI SELECCIONA FITXER
   └─> startMediaFileShare(file) a useLiveKit

3. CREACIÓ DE L'ELEMENT DOM
   └─> document.createElement('video')
   └─> el.src = URL.createObjectURL(file)
   └─> el.style = hidden (position:absolute; left:-9999px)
   └─> document.body.appendChild(el)
   └─> await el.play()

4. CAPTURA DE L'STREAM
   └─> stream = el.captureStream()
   └─> videoTrack = stream.getVideoTracks()[0]  (null si àudio-only)
   └─> audioTrack = stream.getAudioTracks()[0]  (null si vídeo sense so)

5. PUBLICACIÓ A LIVEKIT
   └─> Si videoTrack → new LocalVideoTrack(videoMediaTrack, undefined, true)
       └─> room.localParticipant.publishTrack(lvTrack)
   └─> Si audioTrack → new LocalAudioTrack(audioMediaTrack, undefined, true)
       └─> room.localParticipant.publishTrack(laTrack)

6. ESTAT ACTUALITZAT
   └─> isMediaFileSharing = true
   └─> mediaFileName = file.name
   └─> localMediaFileTrack = lvTrack (o null si àudio-only)
   └─> VoiceArea mostra tile FILE + MediaFilePlayer overlay

7. FI DE L'SHARE
   └─> Automàtic: event 'ended' de l'element → stopMediaFileShare()
   └─> Manual: clic ✕ al MediaFilePlayer o 🎬 al sidebar

8. NETEJA
   └─> unpublishTrack per cada track publicat
   └─> el.pause() + el.src = '' + el.remove()
   └─> URL.revokeObjectURL(objectUrl)
   └─> Estat resetejat
```

**Nota:** `captureStream()` no és suportat a Safari. A Firefox i Chrome funciona correctament.

**Limitació actual:** els participants remots NO veuen un tile FILE per streams d'àudio-only (necessitaria un event Socket.IO out-of-band). Pendent per iteració futura.

## Flux de Connexió a Veu

```
1. USUARI CLICA CANAL DE VEU
   └─> joinChannel(channelId, 'voice')

2. SI JA ESTA EN UN CANAL DE VEU
   └─> leaveVoice() → Desconnecta del canal anterior
   └─> Netega livekit room

3. DEMANA TOKEN A SERVIDOR
   └─> POST /livekit/token
   └─> { channel_id, user_id }
   └─> Servidor verifica permisos
   └─> Retorn: { token, room, e2ee_enabled }

4. CONNECTA A LIVEKIT
   └─> livekit-client: new Room()
   └─> room.connect(token)
   └─> room.on(RoomEvent.TrackSubscribed) → mostra video/audio

5. E2EE DE VEU (si canal ho requereix)
   └─> room.setE2EE(true, { key: sessionKey, keyStore })
   └─> Session key obtinguda via canal de text (E2EE)
   └─> Àudio/vídeo encriptat automàticament

6. USUARIS MOSTRATS
   └─> VoiceParticipants mostra grid de participants
   └─> Llista també apareix a la sidebar sota el canal
   └─> Indicadors: parlant, mut, video
```

## Responsivitat

| Breakpoint | Comportament |
|------------|-------------|
| **> 1200px** | Layout complet: ServerBar + ChannelList + MainContent |
| **900–1200px** | ChannelList es compacta (noms curts), MainContent reduït |
| **< 900px** | ServerBar esdevé icones + tooltip, ChannelList pot amagar-se darrere hamburger |
| **< 600px** | Mobile: només MainContent, sidebar com a drawer deslizable |

### Mobile: Drawer per Sidebar

```tsx
// components/AppLayout.tsx
const [sidebarOpen, setSidebarOpen] = useState(false)

return (
  <div className="app">
    {/* Server bar (sempre visible en desktop, drawer en mobile) */}
    <ServerBar mobile={!isDesktop} />

    {/* Main Content */}
    <main className="main-content">
      <ChannelHeader onToggleSidebar={() => setSidebarOpen(true)} />
      {/* ... */}
    </main>

    {/* Mobile sidebar overlay */}
    {sidebarOpen && (
      <div className="sidebar-overlay" onClick={() => setSidebarOpen(false)}>
        <div className="sidebar mobile-drawer" onClick={e => e.stopPropagation()}>
          <ChannelList />
        </div>
      </div>
    )}
  </div>
)
```

## Rendiment

### Paginació de Missatges

```typescript
// Càrrega incremental — 50 missatges per pàgina
const loadMessages = async (channelId: string, before?: string) => {
  const params = new URLSearchParams({ limit: '50' })
  if (before) params.set('before', before)

  const response = await api.getMessages(channelId, params)
  const newMessages = decryptMessagesIfNeeded(response.messages)

  setMessages(prev => before
    ? [...newMessages, ...prev]  // load more at top
    : [...prev, ...newMessages]  // new at bottom
  )

  setHasMoreMessages(newMessages.length === 50)
}
```

### Virtualització de Missatges (Futur)

Si un canal té milers de missatges, es pot implementar virtualització amb `react-virtuoso` o `@tanstack/virtual`:

```tsx
import { Virtuoso } from 'react-virtuoso'

<Virtuoso
  data={messages}
  itemContent={(index) => <MessageBubble message={messages[index]} />}
  estimateSize={() => 80}
  overscan={200}
/>
```

### Caché de Claus

- **Memòria (React state)**: la clau es manté en state mentre el canal és visible
- **IndexedDB**: caché de persistència entre recarregues
- **No caché**: canals asimètrics on la clau es desencripta sota demanda
- ** TTL**: si el canal té TTL de missatges, la clau s'expira simultàniament
