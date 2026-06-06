# ChillGroup v2 — Protocol Socket.IO

## Visió General

Socket.IO és el canal de temps real per a:
- **Missatges en temps real** (broadcast després d'insertar a DB)
- **Presença** (usuaris connectats, canals actius)
- **Events de veu** (speaking, joining, leaving)
- **Notificacions** (invitacions, canals creats)

**IMPORTANT**: El servidor actua com a única font de veritat. Cada missatge rebut per WebSocket és una còpia del que ja està a la DB. Si un client es reconecta, recupera l'historial per API REST.

## Connexió

### Inicialització (Client → Servidor)

```typescript
// Client
import { io } from 'socket.io-client'

const socket = io('http://localhost:8080', {
  auth: {
    token: 'eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...'  // JWT
  },
  transports: ['websocket', 'polling'],
  withCredentials: true,
})
```

### Connexió Confirmada (Servidor → Client)

```typescript
// Servidor
socket.on('connect', () => {
  console.log('Connectat, ID:', socket.id)
})

// Client rep:
socket.on('connected', (data) => {
  // { userId, username, deviceId }
})
```

**Payload `connected`:**
```json
{
  "userId": "550e8400-e29b-41d4-a716-446655440000",
  "username": "agusti",
  "deviceId": "550e8400-e29b-41d4-a716-446655440001"
}
```

### Reconexió

El client intenta reconexió automàticament (socket.io ho fa). Quan reconnecta:

```typescript
socket.on('reconnected', (data) => {
  // { reconnectCount, missedEvents: number }
})
```

**Payload `reconnected`:**
```json
{
  "reconnectCount": 2,
  "missedEvents": 5
}
```

## Events del Servidor → Client

### Missatges

#### `message` — Nou missatge enviat a un canal

**Emès quan:** Un usuari envia un missatge via API `POST /api/channels/:id/messages` i s'insereix a la DB.

**Target:** Tots els clients de la room `channel:{channelId}`

**Payload:**
```json
{
  "messageId": "550e8400-e29b-41d4-a716-446655440040",
  "channelId": "550e8400-e29b-41d4-a716-446655440020",
  "senderUserId": "550e8400-e29b-41d4-a716-446655440000",
  "senderUsername": "agusti",
  "encryptedPayload": "base64-encrypted-text",
  "iv": "base64-nonce",
  "timestamp": "2026-05-13T10:30:00Z",
  "editedAt": null,
  "deletedAt": null,
  "replyToMessageId": null,
  "reactions": []
}
```

**Client reacciona:**
```typescript
socket.on('message', (msg) => {
  // Comprovar si ja existeix (evitar duplicats)
  if (!messages.has(msg.messageId)) {
    messages.set(msg.messageId, msg)
    // Si el canal té E2EE, desencriptar si es pot
    if (channelNeedsEncryption(msg.channelId)) {
      const decrypted = decryptMessage(msg)
      displayMessage(decrypted)
    } else {
      displayMessage(msg.encryptedPayload)
    }
  }
})
```

---

#### `message-edited` — Missatge editat

> ⚠️ **Gap d'implementació:** El servidor actualment **no emet** aquest event quan s'edita un missatge via `PUT /api/messages/:messageId`. Els clients han de fer polling o refetch manual per detectar edicions. Aquest event queda documentat per implementació futura.

**Target previst:** Clients de la room `channel:{channelId}`

**Payload previst:**
```json
{
  "messageId": "550e8400-e29b-41d4-a716-446655440040",
  "channelId": "550e8400-e29b-41d4-a716-446655440020",
  "encryptedPayload": "base64-new-encrypted-text",
  "iv": "base64-new-nonce",
  "editedAt": "2026-05-13T10:35:00Z"
}
```

---

#### `message-reactions-updated` — Reaccions actualitzades

> ⚠️ **No implementat:** `POST/DELETE /api/messages/:id/reactions` no emeten cap event Socket.IO. Els clients han de refetchar el missatge per veure les reaccions actualitzades.

---

#### `message-deleted` — Missatge eliminat

**Emès quan:** Un usuari elimina el seu propi missatge o un admin de canal (permís `MANAGE`).

**Target:** Clients de la room `channel:{channelId}`

**Payload:**
```json
{
  "messageId": "550e8400-e29b-41d4-a716-446655440040",
  "channelId": "550e8400-e29b-41d4-a716-446655440020",
  "deletedAt": "2026-05-13T10:40:00Z"
}
```

---

### Presència

#### `friend-presence-updated` — Canvi d'estat d'un amic

**Emès quan:** Un usuari amb relació d'amistat activa es connecta o es desconnecta.

**Target:** Clients de la room `user:{ownerUserId}` que tenen aquest usuari com a amic

**Payload:**
```json
{
  "userId": "550e8400-e29b-41d4-a716-446655440000",
  "username": "agusti",
  "status": "online"
}
```

**Client reacciona:**
```typescript
socket.on('friend-presence-updated', (friend) => {
  updateFriendStatus(friend.userId, friend.status)
})
```

#### `user-joined-channel` — Un usuari s'uneix a un canal

**Emès quan:** Un usuari entra a un canal de text per primera vegada (via `join-channel`).

**Target:** Clients de la room `channel:{channelId}`

**Payload:**
```json
{
  "userId": "550e8400-e29b-41d4-a716-446655440000",
  "username": "agusti",
  "joinedAt": "2026-05-13T10:30:00Z"
}
```

---

#### `user-left-channel` — Un usuari surt d'un canal

**Emès quan:** Un usuari surt d'un canal o es desconnecta.

**Target:** Clients de la room `channel:{channelId}`

**Payload:**
```json
{
  "userId": "550e8400-e29b-41d4-a716-446655440000",
  "username": "agusti",
  "leftAt": "2026-05-13T10:35:00Z"
}
```

---

#### `channel-users` — Llista d'usuaris en un canal

**Emès com a resposta a `get-channel-users`** (quan un client entra al canal).

**Target:** Només el client que va demanar-ho

**Payload:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440020",
  "users": [
    {
      "userId": "550e8400-e29b-41d4-a716-446655440000",
      "username": "agusti",
      "connectedAt": "2026-05-13T10:30:00Z"
    },
    {
      "userId": "550e8400-e29b-41d4-a716-446655440002",
      "username": "marcus",
      "connectedAt": "2026-05-13T10:25:00Z"
    }
  ]
}
```

---

### Veu (LiveKit)

#### `voice-joined` — Un usuari s'uneix a un canal de veu

**Emès quan:** El client rep un token LiveKit exitós i es connecta.

**Target:** Clients de la room `voice:{channelId}`

**Payload:**
```json
{
  "userId": "550e8400-e29b-41d4-a716-446655440000",
  "username": "agusti",
  "channelId": "550e8400-e29b-41d4-a716-446655440021",
  "joinedAt": "2026-05-13T10:30:00Z"
}
```

---

#### `voice-left` — Un usuari surt d'un canal de veu

**Emès quan:** Un usuari deixa un canal de veu o es desconnecta.

**Target:** Clients de la room `voice:{channelId}`

**Payload:**
```json
{
  "userId": "550e8400-e29b-41d4-a716-446655440000",
  "username": "agusti",
  "channelId": "550e8400-e29b-41d4-a716-446655440021",
  "leftAt": "2026-05-13T10:35:00Z"
}
```

---

#### `voice-users` — Llista d'usuaris en un canal de veu

**Emès com a resposta a `get-voice-users`.**

**Payload:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440021",
  "users": [
    {
      "userId": "550e8400-e29b-41d4-a716-446655440000",
      "username": "agusti",
      "connectedAt": "2026-05-13T10:30:00Z"
    }
  ]
}
```

---

### Canals

#### `server-channels-updated` — Invalidation de llista de canals

**Emes quan:** Hi ha canvis estructurals de canals al servidor (actualment, creacio de canal).

**Target:** Tots els clients de la room `server:{serverId}`.

**Payload:**
```json
{
  "serverId": "550e8400-e29b-41d4-a716-446655440010",
  "reason": "channel-created",
  "channelId": "550e8400-e29b-41d4-a716-446655440021"
}
```

**Client reacciona:**
- Si `payload.serverId === selectedServer`, executa refetch de canals per REST (`fetchChannels`).
- Al frontend s'aplica debounce de 250ms per compactar bursts d'events.

---

#### `user-servers-updated` — Invalidation de llista de servidors d'un usuari

**Emes quan:** L'usuari passa a tenir visibilitat d'un nou servidor.

Casos implementats:
- Invitacio de membre via endpoint de servidor.
- Registre amb invitacio associada a servidor.

**Target:** Nomes la room `user:{userId}` de l'usuari afectat.

**Payload (exemples):**
```json
{
  "serverId": "550e8400-e29b-41d4-a716-446655440010",
  "reason": "server-invited"
}
```

```json
{
  "serverId": "550e8400-e29b-41d4-a716-446655440010",
  "reason": "server-joined-via-invitation"
}
```

**Client reacciona:**
- Executa refetch de servidors per REST (`fetchServers`).
- Al frontend s'aplica debounce de 250ms per evitar recarregues repetides.

---

#### `server-members-updated` — Invalidation de membres del servidor

**Emes quan:** Canvia la composicio de membres del servidor (alta de membre).

**Target:** Tots els clients de la room `server:{serverId}`.

**Payload:**
```json
{
  "serverId": "550e8400-e29b-41d4-a716-446655440010",
  "reason": "member-added",
  "userId": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Nota:** Aquest event ja s'emet al backend i queda preparat per connectar refresh de membres al frontend quan convingui.

---

#### `channel-created` — Nou canal creat

**Emès quan:** Un usuari crea un canal. Només es remet si el canal és E2EE (els clients han de verificar que tenen accés).

**Target:** Clients que pertanyen al servidor i tenen accés al canal

**Payload:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440021",
  "name": "secret-room",
  "type": "text",
  "encryptionType": "asymmetric",
  "messageTTL": null,
  "isPrivate": true,
  "createdAt": "2026-05-13T10:30:00Z"
}
```

**Client reacciona:**
```typescript
socket.on('channel-created', (channel) => {
  // Només afegir si tenim clau (canals E2EE) o no són privats
  if (channel.encryptionType === 'none' || !channel.isPrivate || hasChannelKey(channel.channelId)) {
    addChannelToList(channel)
  }
})
```

---

#### `channel-deleted` — Canal eliminat

**Payload:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440021",
  "serverId": "550e8400-e29b-41d4-a716-446655440010"
}
```

---

### Invitacions

#### `channel-invited` — Convidat a un canal

**Emès quan:** Un usuari rep una invitació a un canal E2EE.

**Target:** Només el dispositiu convidat (via room `user:{userId}`)

**Payload:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440021",
  "channelName": "secret-room",
  "serverId": "550e8400-e29b-41d4-a716-446655440010",
  "invitedBy": "agusti",
  "encryptedKey": {
    "deviceId": "550e8400-e29b-41d4-a716-446655440002",
    "encryptedKey": "base64",
    "kemCiphertext": "base64"
  }
}
```

**Client reacciona:**
```typescript
socket.on('channel-invited', async (data) => {
  // Desencriptar la clau amb el propi keypair
  const channelKey = await kemDecapsulate(
    localSecretKey,
    data.encryptedKey.kemCiphertext,
    data.encryptedKey.encryptedKey
  )

  // Guardar a IndexedDB
  await storeChannelKey(data.channelId, channelKey, 'asymmetric')

  // Afegir canal a la llista
  addChannelToList({
    channelId: data.channelId,
    name: data.channelName,
    encryptionType: 'asymmetric',
    // ...
  })
})
```

---

## Events del Client → Servidor

### `join-channel` — Entrar a un canal de text

El client s'uneix a la room Socket.IO del canal. **No entra/surt** del canal — sempre es pot veure. És només per a presència en temps real.

**Payload:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440020"
}
```

**Resposta del servidor (room unida):**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440020",
  "users": [ ... ],  // llista d'usuaris en aquest canal
  "lastMessageTimestamp": "2026-05-13T10:25:00Z"
}
```

---

### `leave-channel` — Sortir d'un canal de text

**Payload:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440020"
}
```

---

### `get-channel-users` — Demanar usuaris d'un canal

**Payload:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440020"
}
```

**Resposta:** Event `channel-users` (veure més amunt)

---

### `get-voice-users` — Demanar usuaris d'un canal de veu

**Payload:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440021"
}
```

**Resposta:** Event `voice-users` (veure més amunt)

---

### `voice-join` — Notificar que s'entra a un canal de veu

**Emès DESPRÉS de connectar-se a LiveKit amb èxit.**

**Payload:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440021"
}
```

**Resposta del servidor:** Event `voice-joined` broadcast (veure més amunt)

---

### `voice-leave` — Notificar que surt d'un canal de veu

**Payload:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440021"
}
```

**Resposta del servidor:** Event `voice-left` broadcast

---

### `typing` — Indicador de "escrivint..."

**Emès quan l'usuari està escrivint un missatge.** Cooldown: màxim 1 event cada 2 segons per usuari.

**Payload:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440020"
}
```

**Resposta del servidor:** Broadcast `typing-indicator` a tots els clients del canal

```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440020",
  "userId": "550e8400-e29b-41d4-a716-446655440000",
  "username": "agusti",
  "timestamp": "2026-05-13T10:30:00Z"
}
```

El client amaga l'indicador després de 3 segons sense rebre'n més.

---

### `presence-ping` — Heartbeat de presència

**Emès periòdicament** (cada 30s) per mantenir la connexió activa.

**Payload:**
```json
{
  "timestamp": "2026-05-13T10:30:00Z"
}
```

**Resposta del servidor:**
```json
{
  "ok": true,
  "lastSeen": "2026-05-13T10:29:30Z"
}
```

Si no hi ha heartbeat durant 60s, el servidor marca l'usuari com a desconectat.

---

## Rooms del Servidor

El servidor organitza els clients en rooms:

| Room | Pattern | Ús |
|------|---------|-----|
| `channel:{channelId}` | Text channel | Missatges en temps real, presència |
| `voice:{channelId}` | Voice channel | Events de veu, llista de participants |
| `user:{userId}` | Usuari individual | Invitacions privades, notificacions |
| `server:{serverId}` | Servidor | Events a nivell de servidor |

---

## Fluix Complet: Missatge E2EE

```
CLIENT A (Remitent)                    SERVIDOR                      CLIENT B (Receptor)
     │                                      │                              │
     │── POST /api/channels/:id/messages ──▶│                              │
     │   { encryptedPayload, iv }           │                              │
     │                                      │── Inserta a DB               │
     │                                      │── Emet 'message'           │
     │◀── 201 { messageId, timestamp } ─────│                              │
     │                                      │── Room: channel:{id} ─────▶│
     │                                      │   { messageId, sender,      │
     │                                      │     encryptedPayload, iv }   │
     │                                      │                              │
     │                                      │  ◀── Rep 'message'          │
     │                                      │  ├── Verifica sender ≠ me   │
     │                                      │  ├── Busca channelKey       │
     │                                      │  ├── channelKey trobada?    │
     │                                      │  │   ├── Sí → Decrypt       │
     │                                      │  │   │   → Display text     │
     │                                      │  │   └── No → Display xifrat│
     │                                      │  │                           │
     │                                      │  └── Display missatge      │
```

---

## Fluix Complet: Convidar a Canal E2EE

```
CLIENT A (Creator)               SERVIDOR               CLIENT B (Convidat)
      │                               │                         │
      │── POST /channels/:id/invite ─▶│                         │
      │   { encryptedKeys: [         │                         │
      │       { deviceId,            │                         │
      │         encryptedKey,        │                         │
      │         kemCiphertext }     │                         │
      │     ]                       │                         │
      │                               │── Guarda a channel_keys │
      │                               │                         │
      │◀── 200 OK ───────────────────│                         │
      │                               │── Emet 'channel-invited'│
      │                               │── Room: user:{userIdB}──▶│
      │                               │   { channelId,           │
      │                               │     encryptedKey: {      │
      │                               │       kemCiphertext,     │
      │                               │       encryptedKey      │
      │                               │     }                   │
      │                               │   }                     │
      │                               │                         │
      │                               │  ◀── Rep 'channel-invited'│
      │                               │                         │  ├── Recuperar
      │                               │                         │  │   secretKey
      │                               │                         │  ├── kemDecapsulate
      │                               │                         │  │   → channelKey
      │                               │                         │  ├── Guardar a
      │                               │                         │  │   IndexedDB
      │                               │                         │  ├── Afegir canal
      │                               │                         │  │   a la llista
      │                               │                         │  └── Preparat!
```

---

## Gestió d'Errors WebSocket

### Error de connexió

```json
// Servidor emet quan la connexió falla
{
  "event": "connection-error",
  "data": {
    "code": 4001,
    "message": "Token JWT no vàlid o expirat"
  }
}
```

### Error de permisos

```json
{
  "event": "permission-denied",
  "data": {
    "code": 4003,
    "message": "No tens permís per unir-te a aquest canal",
    "channelId": "550e8400-e29b-41d4-a716-446655440020"
  }
}
```

### Codi d'error WebSocket

| Codi | Significat |
|------|-----------|
| 4000 | Connexió tancada正常ament |
| 4001 | JWT invàlid o expirat |
| 4002 | Dispositiu revocat |
| 4003 | Permisos insuficients |
| 4004 | Canal no trobat |
| 4005 | Rate limit excedit |
| 4006 | Servidor ple (massa connexions) |

---

## Configuració del Servidor (Rust)

```rust
// Servidor utilitza socketioxide per WebSocket natiu
use socketioxide::SocketIo;

let (io, _) = SocketIo::builder()
    .ping_interval(std::time::Duration::from_secs(30))  // Heartbeat
    .ping_timeout(std::time::Duration::from_secs(60))    // Timeout desconexió
    .build()
    .unwrap();

// Auth middleware
io.ns("/", move |sock: Socket<SocketRef>| {
    // Verificar JWT al connect
    let token = sock
        .auth()
        .get("token")
        .and_then(|t| t.as_str())
        .ok_or(Error::Unauthorized)?;

    let claims = verify_jwt(token)?;

    // Afegir claims al socket state
    sock.state().insert(claims);

    // Afegir a room d'usuari
    sock.join(format!("user:{}", claims.user_id))?;

    // Event 'connected'
    sock.emit("connected", ConnectedEvent {
        user_id: claims.user_id,
        username: claims.username,
        device_id: claims.device_id,
    })?;

    // Handlers
    sock.on("join-channel", |data: JoinChannelData, sock| { ... });
    sock.on("leave-channel", |data: LeaveChannelData, sock| { ... });
    sock.on("voice-join", |data: VoiceJoinData, sock| { ... });
    sock.on("voice-leave", |data: VoiceLeaveData, sock| { ... });
    sock.on("typing", |data: TypingData, sock| { ... });
    sock.on("presence-ping", |data: PingData, sock| { ... });

    // Disconnect
    sock.on_disconnect(|| { ... });
});
```

---

## Resum d'Events

### Servidor → Client (rebuts)

| Event | Descripció | Target |
|-------|-----------|--------|
| `connected` | Connexió establerta | Individual |
| `reconnected` | Reconexió completada | Individual |
| `message` | Nou missatge | Room: canal |
| `message-edited` | Missatge editat | Room: canal |
| `message-deleted` | Missatge eliminat | Room: canal |
| `friend-presence-updated` | Estat d'un amic | Room: user |
| `user-joined-channel` | Usuari entra al canal | Room: canal |
| `user-left-channel` | Usuari surt del canal | Room: canal |
| `channel-users` | Llista d'usuaris canal | Individual (resposta) |
| `voice-joined` | Usuari entra a veu | Room: veu |
| `voice-left` | Usuari surt de veu | Room: veu |
| `voice-users` | Llista d'usuaris veu | Individual (resposta) |
| `channel-created` | Nou canal creat | Room: servidor |
| `channel-deleted` | Canal eliminat | Room: servidor |
| `channel-invited` | Convidat a canal | Room: user |
| `typing-indicator` | Algu està escrivint | Room: canal |
| `connection-error` | Error de connexió | Individual |
| `permission-denied` | Permisos insuficients | Individual |

### Client → Servidor (enviats)

| Event | Descripció |
|-------|-----------|
| `join-channel` | Entrar a canal de text |
| `leave-channel` | Surt de canal de text |
| `get-channel-users` | Demanar usuaris canal |
| `get-voice-users` | Demanar usuaris veu |
| `voice-join` | Notificar connexió veu |
| `voice-leave` | Notificar desconexió veu |
| `typing` | Indicador d'escriptura |
| `presence-ping` | Heartbeat |
