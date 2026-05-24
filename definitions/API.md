# ChillGroup v2 — Contracte API REST

## Convenis Generals

- **Base URL**: `http://localhost:8080` (dev) / `https://chillgroup.example.com` (prod)
- **Autenticació**: Header `Authorization: Bearer <JWT>` per a tots els endpoints protegits
- **Contingut**: Tots els requests amb body són `Content-Type: application/json`
- **Respostes**: Totes les respostes són JSON amb format consenant
- **Paginació**: Cursor-based amb paràmetre `before` (UUID del missatge anterior)
- **Limit**: Màxim 100 items per pàgina, per defecte 50

### Format de Resposta Exitosa

```json
{
  "success": true,
  "data": { ... }
}
```

### Format de Resposta d'Error

```json
{
  "success": false,
  "error": {
    "code": 400,
    "message": "Descripció de l'error en català/castellà/anglès",
    "details": {}
  }
}
```

### Format de Resposta amb Paginació

```json
{
  "success": true,
  "data": [ ... ],
  "pagination": {
    "has_more": true,
    "next_cursor": "uuid-del-ultima"
  }
}
```

---

## Autenticació

### POST `/api/auth/register`

Registrar un nou usuari. Genera automàticament un device ID.

**Request:**
```json
{
  "username": "agusti",           // string, 3-50 chars, alfanumèric + _
  "password": "secretpassword"    // string, mínim 8 chars
}
```

**Response 201 Created:**
```json
{
  "success": true,
  "data": {
    "userId": "550e8400-e29b-41d4-a716-446655440000",
    "username": "agusti",
    "token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
    "deviceId": "550e8400-e29b-41d4-a716-446655440001",
    "deviceLabel": "Chrome on macOS"
  }
}
```

**Response 409 Conflict (username ja existeix):**
```json
{
  "success": false,
  "error": {
    "code": 409,
    "message": "El nom d'usuari ja existeix"
  }
}
```

**Response 400 Bad Request (validació fallida):**
```json
{
  "success": false,
  "error": {
    "code": 400,
    "message": "Validació fallida",
    "details": {
      "username": "El nom d'usuari ha de tenir entre 3 i 50 caràcters",
      "password": "La contrasenya ha de tenir almenys 8 caràcters"
    }
  }
}
```

---

### POST `/api/auth/login`

Login d'usuari existent. Retorna JWT + device info.

**Request:**
```json
{
  "username": "agusti",
  "password": "secretpassword"
}
```

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "userId": "550e8400-e29b-41d4-a716-446655440000",
    "username": "agusti",
    "token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
    "deviceId": "550e8400-e29b-41d4-a716-446655440001",
    "deviceLabel": "Chrome on macOS",
    "isAdmin": false
  }
}
```

**Response 401 Unauthorized:**
```json
{
  "success": false,
  "error": {
    "code": 401,
    "message": "Credencials incorrectes"
  }
}
```

---

### POST `/api/auth/refresh`

Renovar el token JWT.

**Request:** Cookie `chillgroup_refresh` (HttpOnly, Secure)

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9..."
  }
}
```

---

### GET `/api/user/me`

Obtenir informació de l'usuari actual (autenticat).

**Headers:** `Authorization: Bearer <JWT>`

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "userId": "550e8400-e29b-41d4-a716-446655440000",
    "username": "agusti",
    "isAdmin": false,
    "deviceId": "550e8400-e29b-41d4-a716-446655440001"
  }
}
```

---

## Amics i Cerca Global

### GET `/api/friends`

Llistar els amics desats de l'usuari autenticat, amb el seu estat de presència actual.

**Headers:** `Authorization: Bearer <JWT>`

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "userId": "550e8400-e29b-41d4-a716-446655440002",
      "username": "marcus",
      "status": "online"
    }
  ]
}
```

---

### POST `/api/friends`

Afegir un amic per `username`. L'amic queda guardat de manera persistent.

**Headers:** `Authorization: Bearer <JWT>`
**Request Body:**
```json
{
  "username": "marcus"
}
```

**Response 204 No Content:**
La relació s'ha creat o ja existia.

---

### DELETE `/api/friends/:friendUserId`

Eliminar un amic desat.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "friendUserId": "string" }`

**Response 204 No Content:**
La relació s'ha eliminat si existia.

---

### GET `/api/users/search`

Buscar usuaris de manera global a tota l'eina, no només dins del servidor actual.

**Headers:** `Authorization: Bearer <JWT>`
**Query Params:**
- `q` (string, obligatori): text de cerca
- `limit` (number, opcional): màxim de resultats, per defecte 20

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "userId": "550e8400-e29b-41d4-a716-446655440002",
      "username": "marcus",
      "isFriend": true,
      "status": "offline"
    }
  ]
}
```

---

## Dispositius

### GET `/api/user/me/devices`

Llistar tots els dispositius associats a l'usuari autenticat.

**Headers:** `Authorization: Bearer <JWT>`

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "deviceId": "550e8400-e29b-41d4-a716-446655440001",
      "label": "Chrome on Linux",
      "publicKey": "base64-encoded-kyber-public-key",
      "createdAt": "2026-05-01T08:00:00Z",
      "lastSeen": "2026-05-13T10:30:00Z",
      "revoked": false
    }
  ]
}
```

---

### PUT `/api/user/me/devices/:deviceId/publicKey`

Actualitzar la clau pública Kyber del dispositiu autenticat.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "deviceId": "string" }`
**Request Body:**
```json
{
  "publicKey": "base64-encoded-1568-bytes"
}
```

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "deviceId": "550e8400-e29b-41d4-a716-446655440001",
    "publicKey": "base64-encoded-1568-bytes",
    "updatedAt": "2026-05-13T10:30:00Z"
  }
}
```

---

### DELETE `/api/user/me/devices/:deviceId`

Revocar un dispositiu de l'usuari.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "deviceId": "string" }`

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "deviceId": "550e8400-e29b-41d4-a716-446655440001",
    "revoked": true
  }
}
```

---

### GET `/api/user/:username/devices`

Obtenir les claus públiques dels dispositius d'un altre usuari. Útil per a convidar a canals E2EE.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "username": "string" }`

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "deviceId": "550e8400-e29b-41d4-a716-446655440002",
      "username": "marcus",
      "publicKey": "base64-encoded-kyber-public-key",
      "revoked": false
    }
  ]
}
```

---

## Servidors

### GET `/api/servers`

Llistar tots els servidors als quals pertany l'usuari.

**Headers:** `Authorization: Bearer <JWT>`

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "serverId": "550e8400-e29b-41d4-a716-446655440010",
      "name": "ChillGroup Dev",
      "iconUrl": null,
      "ownerId": "550e8400-e29b-41d4-a716-446655440000",
      "memberCount": 3,
      "myRole": "owner",
      "createdAt": "2026-05-01T08:00:00Z"
    }
  ]
}
```

---

### POST `/api/servers`

Crear un nou servidor. L'usuari es converteix en owner.

**Headers:** `Authorization: Bearer <JWT>`
**Request Body:**
```json
{
  "name": "ChillGroup Dev",       // string, 1-100 chars
  "iconUrl": null                  // string | null, URL de la icona
}
```

**Response 201 Created:**
```json
{
  "success": true,
  "data": {
    "serverId": "550e8400-e29b-41d4-a716-446655440010",
    "name": "ChillGroup Dev",
    "iconUrl": null,
    "ownerId": "550e8400-e29b-41d4-a716-446655440000",
    "createdAt": "2026-05-13T10:30:00Z"
  }
}
```

**Response 409 Conflict (nom ja existeix):**
```json
{
  "success": false,
  "error": {
    "code": 409,
    "message": "Ja existeix un servidor amb aquest nom"
  }
}
```

---

### GET `/api/servers/:serverId`

Obtenir informació d'un servidor.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "serverId": "string" }`

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "serverId": "550e8400-e29b-41d4-a716-446655440010",
    "name": "ChillGroup Dev",
    "iconUrl": null,
    "ownerId": "550e8400-e29b-41d4-a716-446655440000",
    "members": [
      {
        "userId": "550e8400-e29b-41d4-a716-446655440000",
        "username": "agusti",
        "role": "owner",
        "joinedAt": "2026-05-01T08:00:00Z"
      },
      {
        "userId": "550e8400-e29b-41d4-a716-446655440002",
        "username": "marcus",
        "role": "admin",
        "joinedAt": "2026-05-05T12:00:00Z"
      }
    ],
    "createdAt": "2026-05-01T08:00:00Z"
  }
}
```

---

### DELETE `/api/servers/:serverId`

Eliminar un servidor (només l'owner).

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "serverId": "string" }`

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "serverId": "550e8400-e29b-41d4-a716-446655440010",
    "deleted": true
  }
}
```

---

## Membres del Servidor

### GET `/api/servers/:serverId/members`

Llistar membres d'un servidor.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "serverId": "string" }`

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "userId": "550e8400-e29b-41d4-a716-446655440000",
      "username": "agusti",
      "role": "owner",
      "joinedAt": "2026-05-01T08:00:00Z"
    }
  ]
}
```

---

### POST `/api/servers/:serverId/members`

Afegir un membre al servidor per username.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "serverId": "string" }`
**Request Body:**
```json
{
  "username": "marcus"
}
```

**Response 201 Created:**
```json
{
  "success": true,
  "data": {
    "invitedUser": "marcus"
  }
}
```

---

### PUT `/api/servers/:serverId/members/:userId/role`

Canviar el rol d'un membre.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "serverId": "string", "userId": "string" }`
**Request Body:**
```json
{
  "role": "admin"
}
```

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "userId": "550e8400-e29b-41d4-a716-446655440002",
    "role": "admin"
  }
}
```

---

### DELETE `/api/servers/:serverId/members/:userId`

Eliminar un membre del servidor.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "serverId": "string", "userId": "string" }`

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "userId": "550e8400-e29b-41d4-a716-446655440002",
    "removed": true
  }
}
```

---

## Canals

### GET `/api/servers/:serverId/channels`

Llistar canals d'un servidor. Per a canals E2EE, només retorna canals on l'usuari té clau.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "serverId": "string" }`

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "channelId": "550e8400-e29b-41d4-a716-446655440020",
      "name": "general",
      "type": "text",
      "encryptionType": "none",
      "messageTTL": null,
      "isPrivate": false,
      "createdAt": "2026-05-01T08:00:00Z"
    },
    {
      "channelId": "550e8400-e29b-41d4-a716-446655440021",
      "name": "secret-room",
      "type": "text",
      "encryptionType": "asymmetric",
      "messageTTL": null,
      "isPrivate": true,
      "createdAt": "2026-05-01T08:00:00Z"
    }
  ]
}
```

---

### POST `/api/servers/:serverId/channels`

Crear un nou canal.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "serverId": "string" }`
**Request Body:**
```json
{
  "name": "general",            // string, 1-100 chars
  "type": "text",               // "text" | "voice"
  "encryptionType": "none",     // "none" | "symmetric" | "asymmetric"
  "messageTTL": null,           // integer | null (segons)
  "isPrivate": false            // boolean
}
```

**Response 201 Created:**
```json
{
  "success": true,
  "data": {
    "channelId": "550e8400-e29b-41d4-a716-446655440020",
    "name": "general",
    "type": "text",
    "encryptionType": "none",
    "messageTTL": null,
    "isPrivate": false,
    "createdAt": "2026-05-13T10:30:00Z"
  }
}
```

---

### GET `/api/channels/:channelId/keys`

Obtenir les claus de canal encriptades per al dispositiu actual.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "channelId": "string" }`

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "keyId": "550e8400-e29b-41d4-a716-446655440030",
      "deviceId": "550e8400-e29b-41d4-a716-446655440001",
      "encryptedKey": "base64-encrypted-channel-key",
      "kemCiphertext": "base64-kem-ciphertext",
      "encryptionType": "asymmetric",
      "createdAt": "2026-05-13T10:30:00Z"
    }
  ]
}
```

**Response 403 Forbidden (no té accés al canal):**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "No tens accés a aquest canal"
  }
}
```

---

### POST `/api/channels/:channelId/invite`

Convidar un usuari a un canal. Encripta la clau de canal per als dispositius del destinatari.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "channelId": "string" }`
**Request Body:**
```json
{
  "username": "marcus",
  "encryptedKeys": [
    {
      "deviceId": "550e8400-e29b-41d4-a716-446655440001",
      "encryptedKey": "base64-encrypted-channel-key-for-device",
      "kemCiphertext": "base64-kem-ciphertext-for-device"
    }
  ]
}
```

**Response 201 Created:**
```json
{
  "success": true,
  "data": {
    "invitedUser": "marcus",
    "devicesInvited": 2
  }
}
```

---

### PUT `/api/channels/:channelId`

Actualitzar configuració del canal.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "channelId": "string" }`
**Request Body:**
```json
{
  "name": "novo-nom",
  "messageTTL": 3600
}
```

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "channelId": "550e8400-e29b-41d4-a716-446655440020",
    "name": "novo-nom",
    "messageTTL": 3600
  }
}
```

---

### DELETE `/api/channels/:channelId`

Eliminar un canal.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "channelId": "string" }`

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "channelId": "550e8400-e29b-41d4-a716-446655440020",
    "deleted": true
  }
}
```

---

## Missatges

### GET `/api/channels/:channelId/messages`

Llistar missatges d'un canal. Només retorna missatges que l'usuari pot desxifrar.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "channelId": "string" }`
**Query Params:**
- `limit=50` (màx 100)
- `before=uuid` (cursor)

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "messageId": "550e8400-e29b-41d4-a716-446655440040",
      "channelId": "550e8400-e29b-41d4-a716-446655440020",
      "senderUserId": "550e8400-e29b-41d4-a716-446655440000",
      "senderUsername": "agusti",
      "senderDeviceId": "550e8400-e29b-41d4-a716-446655440001",
      "encryptedPayload": "base64-encrypted-or-plain-text",
      "iv": "base64-initialization-vector",
      "timestamp": "2026-05-13T10:30:00Z",
      "expiresAt": null,
      "editedAt": null,
      "deletedAt": null
    }
  ],
  "pagination": {
    "has_more": true,
    "next_cursor": "550e8400-e29b-41d4-a716-446655440040"
  }
}
```

---

### POST `/api/channels/:channelId/messages`

Enviar un missatge a un canal.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "channelId": "string" }`
**Request Body:**
```json
{
  "encryptedPayload": "base64-encrypted-text",
  "iv": "base64-12-byte-nonce",
  "expiresAt": null                // "2026-05-13T11:00:00Z" | null
}
```

**Response 201 Created:**
```json
{
  "success": true,
  "data": {
    "messageId": "550e8400-e29b-41d4-a716-446655440040",
    "channelId": "550e8400-e29b-41d4-a716-446655440020",
    "timestamp": "2026-05-13T10:30:00Z"
  }
}
```

**Response 403 Forbidden:**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "No tens accés a aquest canal"
  }
}
```

---

### PUT `/api/messages/:messageId`

Editar un missatge (només el remitent, dins dels 5 minuts).

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "messageId": "string" }`
**Request Body:**
```json
{
  "encryptedPayload": "base64-new-encrypted-text",
  "iv": "base64-new-nonce"
}
```

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "messageId": "550e8400-e29b-41d4-a716-446655440040",
    "editedAt": "2026-05-13T10:35:00Z"
  }
}
```

---

### DELETE `/api/messages/:messageId`

Eliminar un missatge (soft delete).

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "messageId": "string" }`

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "messageId": "550e8400-e29b-41d4-a716-446655440040",
    "deletedAt": "2026-05-13T10:40:00Z"
  }
}
```

---

### GET `/api/messages/:messageId`

Recuperar un missatge concret pel seu ID.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "messageId": "string" }`

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "messageId": "550e8400-e29b-41d4-a716-446655440040",
    "channelId": "550e8400-e29b-41d4-a716-446655440020",
    "senderUserId": "550e8400-e29b-41d4-a716-446655440000",
    "senderUsername": "agusti",
    "senderDeviceId": "550e8400-e29b-41d4-a716-446655440001",
    "encryptedPayload": "base64-encrypted-text",
    "iv": "base64-iv",
    "timestamp": "2026-05-13T10:30:00Z",
    "expiresAt": null,
    "editedAt": null,
    "deletedAt": null
  }
}
```

**Response 404 Not Found:**
```json
{
  "success": false,
  "error": {
    "code": 404,
    "message": "Missatge no trobat"
  }
}
```

---

### GET `/api/channels/:channelId/messages/check-new`

Consultar si hi ha missatges nous des de l'última visita de l'usuari al canal.
Aquest endpoint és útil per saber si cal descarregar nous missatges quan l'usuari
torna a entrar a un canal.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "channelId": "string" }`
**Query Params:**
- `last_seen=2026-05-13T10:00:00Z` — Timestamp de l'última visita (obligatori)

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "channelId": "550e8400-e29b-41d4-a716-446655440020",
    "hasNew": true,
    "newCount": 5,
    "firstNewMessageId": "550e8400-e29b-41d4-a716-446655440045",
    "lastSeen": "2026-05-13T10:00:00Z"
  }
}
```

**Response amb zero missatges nous:**
```json
{
  "success": true,
  "data": {
    "channelId": "550e8400-e29b-41d4-a716-446655440020",
    "hasNew": false,
    "newCount": 0,
    "firstNewMessageId": null,
    "lastSeen": "2026-05-13T10:00:00Z"
  }
}
```

---

## Missatges Directes (DM)

### DM v2 (canal 1:1 asimètric)

Implementació nova basada en canal privat 1:1.

#### POST `/api/dm/channels/open`

Obre (o crea) un canal DM 1:1 amb un altre usuari.

**Request Body:**
```json
{
  "targetUserId": "550e8400-e29b-41d4-a716-446655440002",
  "messageTTL": 86400
}
```

#### GET `/api/dm/channels`

Llistar canals DM de l'usuari autenticat.

#### GET `/api/dm/channels/:channelId/messages`

Llistar missatges d'un DM específic.

#### POST `/api/dm/channels/:channelId/messages`

Enviar un missatge al DM.

#### PUT `/api/dm/channels/:channelId/settings`

Actualitzar configuració del DM (actualment `messageTTL`).

> Nota: aquesta secció descriu els endpoints actuals (legacy).
> El model objectiu per a implementació nova és a `definitions/DM.md`:
> DM 1:1 com a canal privat asimètric amb `message_ttl`.

### POST `/api/direct-messages`

Enviar un missatge directe (privat) a un altre usuari.

**Headers:** `Authorization: Bearer <JWT>`
**Request Body:**
```json
{
  "encryptedPayload": "base64-encrypted-text",
  "iv": "base64-12-byte-nonce",
  "isDirect": true,
  "recipientUserId": "550e8400-e29b-41d4-a716-446655440002",
  "expiresAt": null
}
```

**Response 201 Created:**
```json
{
  "success": true,
  "data": {
    "messageId": "550e8400-e29b-41d4-a716-446655440040",
    "senderUserId": "550e8400-e29b-41d4-a716-446655440000",
    "recipientUserId": "550e8400-e29b-41d4-a716-446655440002",
    "timestamp": "2026-05-13T10:30:00Z"
  }
}
```

---

### GET `/api/direct-messages/list`

Llistar missatges directes entre l'usuari autenticat i un altre usuari.

**Headers:** `Authorization: Bearer <JWT>`
**Query Params:**
- `withUser=uuid` — ID de l'altre usuari (obligatori)
- `limit=50` — Nombre de missatges (màx 100)
- `after=uuid` — Cursor per missatges més nous
- `before=uuid` — Cursor per missatges més antics

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "messageId": "550e8400-e29b-41d4-a716-446655440040",
      "senderUserId": "550e8400-e29b-41d4-a716-446655440000",
      "recipientUserId": "550e8400-e29b-41d4-a716-446655440002",
      "encryptedPayload": "base64-encrypted-text",
      "iv": "base64-iv",
      "timestamp": "2026-05-13T10:30:00Z",
      "isDirect": true,
      "deletedAt": null
    }
  ],
  "pagination": {
    "hasMore": false,
    "nextCursor": null,
    "prevCursor": null
  }
}
```

---

### GET `/api/conversations`

Llistar les converses directes de l'usuari amb resum de cada conversa.

**Headers:** `Authorization: Bearer <JWT>`

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "otherUserId": "550e8400-e29b-41d4-a716-446655440002",
      "otherUserUsername": "marcus",
      "otherUserAvatar": "https://example.com/avatar.jpg",
      "lastMessageAt": "2026-05-13T10:30:00Z",
      "unreadCount": 3,
      "lastMessagePreview": "Hola!"
    }
  ]
}
```

---

## LiveKit

### POST `/api/livekit/token`

Generar un token d'accés a LiveKit per a un canal de veu.

**Headers:** `Authorization: Bearer <JWT>`
**Request Body:**
```json
{
  "channelId": "550e8400-e29b-41d4-a716-446655440021"
}
```

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "room": "chill-550e8400-e29b-41d4-a716-446655440010-550e8400-e29b-41d4-a716-446655440021",
    "livekitHost": "wss://livekit.example.com",
    "e2eeEnabled": true
  }
}
```

**Response 403 Forbidden:**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "No tens accés a aquest canal de veu"
  }
}
```

---

## Health Check

### GET `/health`

**Response 200 OK:**
```json
{
  "status": "healthy",
  "database": "connected",
  "uptimeSeconds": 3600
}
```

**Response 503 Service Unavailable:**
```json
{
  "status": "degraded",
  "database": "error",
  "uptimeSeconds": 3600
}
```

---

## Resum d'Endpoints

| Mètode | Path | Autenticat? | Descripció |
|--------|------|-------------|------------|
| POST | `/api/auth/register` | No | Registrar usuari |
| POST | `/api/auth/login` | No | Login |
| POST | `/api/auth/refresh` | No (cookie) | Renovar token |
| GET | `/api/user/me` | Sí | Info usuari actual |
| GET | `/api/user/me/devices` | Sí | Llistar dispositius |
| PUT | `/api/user/me/devices/:id/publicKey` | Sí | Actualitzar public key |
| DELETE | `/api/user/me/devices/:id` | Sí | Revocar dispositiu |
| GET | `/api/user/:username/devices` | Sí | Claus públiques dispositius |
| GET | `/api/servers` | Sí | Llistar servidors |
| POST | `/api/servers` | Sí | Crear servidor |
| GET | `/api/servers/:id` | Sí | Info servidor |
| DELETE | `/api/servers/:id` | Sí | Eliminar servidor |
| GET | `/api/servers/:id/members` | Sí | Llistar membres |
| POST | `/api/servers/:id/members` | Sí | Afegir membre |
| PUT | `/api/servers/:id/members/:uid/role` | Sí | Canviar rol |
| DELETE | `/api/servers/:id/members/:uid` | Sí | Eliminar membre |
| GET | `/api/servers/:id/channels` | Sí | Llistar canals |
| POST | `/api/servers/:id/channels` | Sí | Crear canal |
| GET | `/api/channels/:id/keys` | Sí | Claus de canal |
| POST | `/api/channels/:id/invite` | Sí | Convidar a canal |
| PUT | `/api/channels/:id` | Sí | Actualitzar canal |
| DELETE | `/api/channels/:id` | Sí | Eliminar canal |
| GET | `/api/channels/:id/messages` | Sí | Llistar missatges |
| POST | `/api/channels/:id/messages` | Sí | Enviar missatge |
| PUT | `/api/messages/:id` | Sí | Editar missatge |
| GET | `/api/messages/:id` | Sí | Recuperar missatge concret |
| DELETE | `/api/messages/:id` | Sí | Eliminar missatge |
| GET | `/api/channels/:id/messages/check-new` | Sí | Check missatges nous |
| POST | `/api/direct-messages` | Sí | Enviar DM |
| GET | `/api/direct-messages/list` | Sí | Llistar DMs |
| GET | `/api/conversations` | Sí | Llistar converses |
| POST | `/api/livekit/token` | Sí | Generar token LiveKit |
| GET | `/health` | No | Health check |
