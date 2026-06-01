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

**Nota:** Si el servidor té `OPEN_REGISTER=false`, aquest endpoint retorna **403 Forbidden** excepte si es proporciona `admin_invitation_code` vàlid (one-shot).

**Request:**
```json
{
  "username": "agusti",           // string, 3-50 chars, alfanumèric + _
  "password": "secretpassword",   // string, mínim 8 chars
  "admin_invitation_code": "CODI-UNIC-ADMIN" // opcional, promociona aquest registre a admin (1 sol ús)
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

**Response 403 Forbidden (registre tancat):**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "El registre està desactivat. Contacta amb un administrador."
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

### POST `/api/auth/register-with-invitation`

Registrar un nou usuari utilitzant una invitació. **Funciona fins i tot si `OPEN_REGISTER=false`**.

Si la invitació està vinculada a un servidor (`serverId`/`server_id`), l'usuari queda afegit automàticament com a membre (`role=member`) en completar el registre.

**Request:**
```json
{
  "code": "abc123-def456-ghi789",         // Codi d'invitació
  "username": "newuser",                  // string, 3-50 chars
  "password": "secretpassword",           // string, mínim 8 chars
  "admin_invitation_code": "CODI-UNIC-ADMIN" // opcional, promociona aquest registre a admin (1 sol ús)
}
```

**Response 201 Created:**
```json
{
  "success": true,
  "data": {
    "userId": "550e8400-e29b-41d4-a716-446655440005",
    "username": "newuser",
    "token": "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...",
    "deviceId": "550e8400-e29b-41d4-a716-446655440101",
    "deviceLabel": "Chrome on macOS"
  }
}
```

**Response 404 Not Found (codi invàlid):**
```json
{
  "success": false,
  "error": {
    "code": 404,
    "message": "Codi d'invitació no vàlid o desactivat"
  }
}
```

**Response 410 Gone (invitació exhausted):**
```json
{
  "success": false,
  "error": {
    "code": 410,
    "message": "Aquesta invitació ha assolit el límit d'usos"
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

## Administració

Els endpoints de administració són **només accessibles als usuaris amb rol `admin`**. Si intenteun usuari normal accedir-hi, reben **403 Forbidden**.

### GET `/api/admin/users`

Llistar tots els usuaris del sistema amb informació bàsica.

**Headers:** `Authorization: Bearer <JWT>` (usuari admin requerida)
**Query Params:**
- `limit` (number, opcional): màxim de resultats, per defecte 50
- `offset` (number, opcional): offset per paginació, per defecte 0

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "userId": "550e8400-e29b-41d4-a716-446655440000",
      "username": "agusti",
      "role": "admin",
      "createdAt": "2026-05-01T08:00:00Z",
      "updatedAt": "2026-05-13T10:30:00Z"
    },
    {
      "userId": "550e8400-e29b-41d4-a716-446655440002",
      "username": "marcus",
      "role": "user",
      "createdAt": "2026-05-02T09:00:00Z",
      "updatedAt": "2026-05-13T10:30:00Z"
    }
  ],
  "pagination": {
    "total": 42,
    "limit": 50,
    "offset": 0
  }
}
```

**Response 403 Forbidden (no és admin):**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "Accés denegat. Es requereix rol d'administrador."
  }
}
```

---

### POST `/api/admin/users`

Crear un nou usuari com a administrador (sempre funciona, independent de `OPEN_REGISTER`).

**Headers:** `Authorization: Bearer <JWT>` (usuari admin requerida)
**Request Body:**
```json
{
  "username": "newuser",
  "password": "temporalpassword",
  "role": "user"  // o "admin"
}
```

**Response 201 Created:**
```json
{
  "success": true,
  "data": {
    "userId": "550e8400-e29b-41d4-a716-446655440003",
    "username": "newuser",
    "role": "user",
    "createdAt": "2026-05-13T11:00:00Z"
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

**Response 403 Forbidden (no és admin):**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "Accés denegat. Es requereix rol d'administrador."
  }
}
```

---

### PUT `/api/admin/users/:userId`

Modificar les dades d'un usuari (username, password, role).

**Headers:** `Authorization: Bearer <JWT>` (usuari admin requerida)
**Path Params:** `{ "userId": "string" }`
**Request Body:**
```json
{
  "username": "newusername",  // opcional
  "password": "newpassword",  // opcional
  "role": "admin"             // opcional, "user" o "admin"
}
```

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "userId": "550e8400-e29b-41d4-a716-446655440003",
    "username": "newusername",
    "role": "admin",
    "updatedAt": "2026-05-13T11:30:00Z"
  }
}
```

**Response 404 Not Found (usuari no existeix):**
```json
{
  "success": false,
  "error": {
    "code": 404,
    "message": "Usuari no trobat"
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

**Response 403 Forbidden (no és admin):**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "Accés denegat. Es requereix rol d'administrador."
  }
}
```

---

### DELETE `/api/admin/users/:userId`

Esborrar un usuari i tots els seus dispositius, servidors, canals, missatges i amics associats (cascada).

**Headers:** `Authorization: Bearer <JWT>` (usuari admin requerida)
**Path Params:** `{ "userId": "string" }`

**Response 204 No Content:**
L'usuari ha estat esborrat correctament.

**Response 404 Not Found (usuari no existeix):**
```json
{
  "success": false,
  "error": {
    "code": 404,
    "message": "Usuari no trobat"
  }
}
```

**Response 403 Forbidden (no és admin):**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "Accés denegat. Es requereix rol d'administrador."
  }
}
```

### PUT `/api/admin/users/:userId/plan/:planId`

Canviar el plan (tier) d'un usuari (admin only). S'usa per upgrade/downgrade o assignar tiers.

**Headers:** `Authorization: Bearer <JWT>` (usuari admin requerida)
**Path Params:** 
- `userId` (string): ID del usuari
- `planId` (string): UUID del plan

**Response 204 No Content:**
Plan assignat correctament.

**Response 404 Not Found (usuari no existeix):**
```json
{
  "success": false,
  "error": {
    "code": 404,
    "message": "Usuari no trobat"
  }
}
```

**Response 400 Bad Request (plan no existeix):**
```json
{
  "success": false,
  "error": {
    "code": 400,
    "message": "Petició incorrecta"
  }
}
```

**Response 403 Forbidden (no és admin):**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "Accés denegat. Es requereix rol d'administrador."
  }
}
```

---

## Plans i Límits

### Gestió de Plans (Admin)

Els endpoints de gestió de plans són exclusius d'administrador i permeten operar sobre plans personalitzats (crear, modificar, eliminar).

**Regles de negoci:**
- Els plans del sistema (`free`, `pro`, `enterprise`) són protegits i no es poden modificar ni eliminar.
- No es pot eliminar un plan que tingui usuaris assignats.
- Els camps de límit admeten `-1` com a "sense límit".

### GET `/api/admin/plans`

Llistar tots els plans amb marca de sistema.

**Headers:** `Authorization: Bearer <JWT>` (usuari admin requerida)

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655441001",
      "name": "free",
      "displayName": "Free",
      "description": "Plan gratuït",
      "maxServers": 1,
      "maxChannelsTextPerServer": 3,
      "maxChannelsVoicePerServer": 2,
      "maxMembersPerServer": 20,
      "apiCallsPerMinute": 60,
      "messagesPerDay": 10000,
      "isSystem": true
    }
  ]
}
```

### POST `/api/admin/plans`

Crear un plan personalitzat.

**Headers:** `Authorization: Bearer <JWT>` (usuari admin requerida)
**Request Body:**
```json
{
  "name": "team_plus",
  "displayName": "Team Plus",
  "description": "Plan per equips",
  "maxServers": 8,
  "maxChannelsTextPerServer": 30,
  "maxChannelsVoicePerServer": 15,
  "maxMembersPerServer": 800,
  "apiCallsPerMinute": 1200,
  "messagesPerDay": -1
}
```

**Response 201 Created:**
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655449999",
    "name": "team_plus",
    "displayName": "Team Plus",
    "description": "Plan per equips",
    "maxServers": 8,
    "maxChannelsTextPerServer": 30,
    "maxChannelsVoicePerServer": 15,
    "maxMembersPerServer": 800,
    "apiCallsPerMinute": 1200,
    "messagesPerDay": -1,
    "isSystem": false
  }
}
```

### PUT `/api/admin/plans/:planId`

Modificar un plan personalitzat (actualització parcial).

**Headers:** `Authorization: Bearer <JWT>` (usuari admin requerida)
**Path Params:** `{ "planId": "string" }`

**Request Body (exemple):**
```json
{
  "displayName": "Team Plus Updated",
  "maxServers": 10
}
```

**Response 204 No Content:**
Plan actualitzat correctament.

**Response 403 Forbidden (plan del sistema):**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "Aquest plan del sistema no es pot modificar"
  }
}
```

### DELETE `/api/admin/plans/:planId`

Eliminar un plan personalitzat.

**Headers:** `Authorization: Bearer <JWT>` (usuari admin requerida)
**Path Params:** `{ "planId": "string" }`

**Response 204 No Content:**
Plan eliminat correctament.

**Response 409 Conflict (plan en ús):**
```json
{
  "success": false,
  "error": {
    "code": 409,
    "message": "No es pot eliminar un plan que està assignat a usuaris"
  }
}
```

### GET `/api/plans`

Llistar tots els plans disponibles per al client autenticat.

**Headers:** `Authorization: Bearer <JWT>`

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655441001",
      "name": "free",
      "displayName": "Free",
      "description": "Tier gratuïto per a usuaris individuals",
      "maxServers": 1,
      "maxChannelsTextPerServer": 3,
      "maxChannelsVoicePerServer": 2,
      "maxMembersPerServer": 20,
      "apiCallsPerMinute": 60,
      "messagesPerDay": 10000
    },
    {
      "id": "550e8400-e29b-41d4-a716-446655441002",
      "name": "pro",
      "displayName": "Professional",
      "description": "Per a grups i petites organitzacions",
      "maxServers": 5,
      "maxChannelsTextPerServer": 20,
      "maxChannelsVoicePerServer": 10,
      "maxMembersPerServer": 500,
      "apiCallsPerMinute": 600,
      "messagesPerDay": -1
    },
    {
      "id": "550e8400-e29b-41d4-a716-446655441003",
      "name": "enterprise",
      "displayName": "Enterprise",
      "description": "Per a grans organitzacions amb suport personalitzat",
      "maxServers": -1,
      "maxChannelsTextPerServer": -1,
      "maxChannelsVoicePerServer": -1,
      "maxMembersPerServer": -1,
      "apiCallsPerMinute": -1,
      "messagesPerDay": -1
    }
  ]
}
```

**Nota:** `-1` significa "unlimited" (sense límit).

---

### GET `/api/user/me/plan`

Obtenir el plan actual del usuari autenticat amb límits i informació de uso.

**Headers:** `Authorization: Bearer <JWT>`

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "plan": {
      "id": "550e8400-e29b-41d4-a716-446655441001",
      "name": "free",
      "displayName": "Free",
      "maxServers": 1,
      "maxChannelsTextPerServer": 3,
      "maxChannelsVoicePerServer": 2,
      "maxMembersPerServer": 20,
      "apiCallsPerMinute": 60,
      "messagesPerDay": 10000
    },
    "usage": {
      "totalServers": 0,
      "totalTextChannels": 0,
      "totalVoiceChannels": 0,
      "totalMembersAcrossServers": 0,
      "messagesToday": 0,
      "apiCallsThisMinute": 0
    },
    "canCreateServer": true,
    "canCreateTextChannel": true,
    "canCreateVoiceChannel": true,
    "remainingServers": 1,
    "remainingTextChannels": 3,
    "remainingVoiceChannels": 2
  }
}
```

---

### POST `/api/user/me/check-limits`

Verificar si l'usuari pot realitzar una acció específica (crear servidor, canal, etc) sense que es realitzi l'acció.

**Headers:** `Authorization: Bearer <JWT>`
**Request Body:**
```json
{
  "action": "create_server",  // o: "create_text_channel", "create_voice_channel", "add_member"
  "serverId": "550e8400-e29b-41d4-a716-446655440100"  // obligatori per a create_*_channel i add_member
}
```

**Response 200 OK (limit check OK):**
```json
{
  "success": true,
  "data": {
    "allowed": true,
    "message": "Pots crear aquest recurso"
  }
}
```

**Response 429 Too Many Requests (limit exceeded):**
```json
{
  "success": false,
  "error": {
    "code": 429,
    "message": "Has assolit el límit de servidors per a aquest plan. Límit: 1, Actuals: 1.",
    "details": {
      "limitType": "max_servers",
      "limit": 1,
      "current": 1,
      "suggestedAction": "upgrade_plan"
    }
  }
}
```

---

## Invitacions

### POST `/api/invitations`

Generar una nova invitació (admin only).

**Headers:** `Authorization: Bearer <JWT>` (usuari admin requerida)

**Request Body:**
```json
{
  "max_uses": 5,  // optional, default 1. -1 = unlimited
  "server_id": "550e8400-e29b-41d4-a716-446655440100" // optional, null/absent = registre global
}
```

**Response 201 Created:**
```json
{
  "success": true,
  "data": {
    "invitationId": "550e8400-e29b-41d4-a716-446655440200",
    "code": "abc123-def456-ghi789-xyz000",
    "serverId": "550e8400-e29b-41d4-a716-446655440100",
    "maxUses": 5,
    "usesCount": 0,
    "isActive": true,
    "createdBy": "admin"
  }
}
```

**Response 403 Forbidden (no és admin):**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "Accés denegat. Es requereix rol d'administrador."
  }
}
```

---

### GET `/api/invitations`

Llistar invitacions creades (admin only).

**Headers:** `Authorization: Bearer <JWT>` (usuari admin requerida)

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "invitationId": "550e8400-e29b-41d4-a716-446655440200",
      "code": "abc123-def456-ghi789-xyz000",
      "serverId": "550e8400-e29b-41d4-a716-446655440100",
      "maxUses": 5,
      "usesCount": 2,
      "isActive": true,
      "createdBy": "admin",
      "remainingUses": 3
    }
  ]
}
```

---

### DELETE `/api/invitations/:invitationId`

Invalidar una invitació (admin only).

**Headers:** `Authorization: Bearer <JWT>` (usuari admin requerida)

**Path Params:** `{ "invitationId": "string" }`

**Response 204 No Content:**
L'invitació ha estat invalidada.

**Response 404 Not Found:**
```json
{
  "success": false,
  "error": {
    "code": 404,
    "message": "Invitació no trobada"
  }
}
```

**Response 403 Forbidden (no és admin):**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "Accés denegat. Es requereix rol d'administrador."
  }
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

**Response 429 Too Many Requests (limit exceeded):**
```json
{
  "success": false,
  "error": {
    "code": 429,
    "message": "Has assolit el límit de servidors per al teu plan. Límit: 1, Actuals: 1.",
    "details": {
      "limitType": "max_servers",
      "limit": 1,
      "current": 1,
      "suggestedAction": "upgrade_plan"
    }
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

### Model de Permisos Explícits (Servidor)

El backend resol permisos de servidor amb nivells explícits:

- `1` — `view`: veure servidor i membres
- `2` — `manage_profile`: editar metadades del servidor (nom/icona)
- `3` — `manage_members`: convidar membres i gestionar rols

Mapeig actual de rols:

- `owner` -> nivell `3`
- `admin` -> nivell `3`
- `member` -> nivell `1`

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

### PUT `/api/servers/:serverId`

Actualitzar metadades del servidor (`name`, `iconUrl`).

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "serverId": "string" }`
**Request Body:**
```json
{
  "name": "ChillGroup Core",
  "iconUrl": "https://cdn.example.com/icons/core.png"
}
```

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "serverId": "550e8400-e29b-41d4-a716-446655440010",
    "name": "ChillGroup Core",
    "iconUrl": "https://cdn.example.com/icons/core.png"
  }
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

**Autorització:** mínim nivell `3` (`manage_members`).

**Restricció:** no es pot eliminar el membre amb rol `owner`.

---

## Canals

### Model de Permisos Explícits (Canal)

El backend resol permisos de canal amb nivells explícits:

- `1` — `read`: llegir canal i recuperar claus
- `2` — `write`: enviar missatges, convidar en canals asimètrics, pujar bundles asimètrics
- `3` — `manage`: editar/eliminar canal, convidar en canals no asimètrics, rotar clau simètrica

Regles especials:

- Si existeix override explícit a `channel_members.permission_level`, aquest valor **preval** tant en canals públics com privats.
- Si NO existeix override explícit: en canals públics, `member` rep nivell `2`; `owner/admin` rep nivell `3`.
- En canals privats, sense override explícit, l'usuari queda sense accés (`0`).
- En DM (`scope=dm`), els dos membres tenen nivell `3`.

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
      "permissionLevel": 3,
      "messageTTL": null,
      "isPrivate": false,
      "createdAt": "2026-05-01T08:00:00Z"
    },
    {
      "channelId": "550e8400-e29b-41d4-a716-446655440021",
      "name": "secret-room",
      "type": "text",
      "encryptionType": "asymmetric",
      "permissionLevel": 2,
      "messageTTL": null,
      "isPrivate": true,
      "createdAt": "2026-05-01T08:00:00Z"
    }
  ]
}
```

**Nota:** `permissionLevel` és el nivell efectiu resolt pel backend (`0..3`) i es pot usar al frontend per habilitar/deshabilitar accions (p. ex. configuració de canal).

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
    "permissionLevel": 3,
    "messageTTL": null,
    "isPrivate": false,
    "createdAt": "2026-05-13T10:30:00Z"
  }
}
```

**Response 429 Too Many Requests (limit exceeded):**
```json
{
  "success": false,
  "error": {
    "code": 429,
    "message": "Has assolit el límit de canals de text per a aquest servidor. Límit: 3, Actuals: 3.",
    "details": {
      "limitType": "max_channels_text_per_server",
      "limit": 3,
      "current": 3,
      "serverId": "550e8400-e29b-41d4-a716-446655440010",
      "suggestedAction": "upgrade_plan"
    }
  }
}
```

**Response 403 Forbidden (no és propietari o no té permisos):**
```json
{
  "success": false,
  "error": {
    "code": 403,
    "message": "No tens permisos per crear canals en aquest servidor"
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

**Autorització:**

- Canal `asymmetric`: mínim nivell `2` (`write`)
- Altres canals: mínim nivell `3` (`manage`)

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

**Autorització:** mínim nivell `3` (`manage`).

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

**Autorització:** mínim nivell `3` (`manage`).

---

### GET `/api/channels/:channelId/permissions`

Llistar permisos efectius per usuari d'un canal.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "channelId": "string" }`

**Autorització:**

- Nivell servidor `3` (`manage_members`) **o**
- Nivell canal `3` (`manage`)

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "userId": "550e8400-e29b-41d4-a716-446655440111",
      "username": "agusti",
      "permissionLevel": 3,
      "permission": "manage"
    },
    {
      "userId": "550e8400-e29b-41d4-a716-446655440112",
      "username": "pop",
      "permissionLevel": 1,
      "permission": "read"
    }
  ]
}
```

---

### GET `/api/channels/:channelId/permissions/explicit`

Llistar només overrides explícits (`channel_members.permission_level`) del canal.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "channelId": "string" }`

**Autorització:** mínim nivell canal `3` (`manage`).

**Response 200 OK:**
```json
{
  "success": true,
  "data": [
    {
      "userId": "550e8400-e29b-41d4-a716-446655440112",
      "username": "pop",
      "permissionLevel": 1,
      "permission": "read"
    }
  ]
}
```

---

### PUT `/api/channels/:channelId/permissions/explicit/:userId`

Crear/actualitzar o eliminar un override explícit de permís per a un usuari concret.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "channelId": "string", "userId": "string" }`

**Request Body (set override):**
```json
{
  "permissionLevel": 1
}
```

**Request Body (eliminar override i tornar a heretat):**
```json
{
  "permissionLevel": null
}
```

**Autorització:** mínim nivell canal `3` (`manage`).

**Response 204 No Content**

**Notes:**

- `permissionLevel` accepta `1`, `2`, `3` o `null`.
- `null` elimina el registre de `channel_members` i el permís torna a ser heretat.

---

### POST `/api/channels/:channelId/keys/rotate`

Rotar versió de clau de canal (`keyVersion = N+1`).

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "channelId": "string" }`

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "channelId": "550e8400-e29b-41d4-a716-446655440020",
    "keyVersionId": "550e8400-e29b-41d4-a716-446655440030",
    "keyVersion": 2
  }
}
```

**Autorització:**

- Canal `symmetric`: mínim nivell `3` (`manage`)
- Canal `asymmetric`: mínim nivell `2` (`write`)

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
  "attachmentIds": [
    "550e8400-e29b-41d4-a716-4466554400f1"
  ],
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
    "attachments": [
      {
        "attachmentId": "550e8400-e29b-41d4-a716-4466554400f1",
        "fileName": "contracte.pdf",
        "mimeType": "application/pdf",
        "sizeBytes": 184223,
        "createdAt": "2026-05-13T10:29:54Z"
      }
    ],
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

## Adjunts (S3-compatible, xifrats client-side)

### POST `/api/channels/:channelId/attachments/init`

Iniciar un upload multipart per un adjunt. El backend valida permisos i retorna `attachmentId` i `uploadId` S3.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:** `{ "channelId": "string" }`
**Request Body:**
```json
{
  "fileName": "contracte.pdf",
  "mimeType": "application/pdf",
  "sizeBytes": 184223,
  "createdAt": "2026-05-13T10:29:54Z",
  "chunkSizeBytes": 5242880,
  "chunkCount": 1
}
```

**Response 201 Created:**
```json
{
  "success": true,
  "data": {
    "attachmentId": "550e8400-e29b-41d4-a716-4466554400f1",
    "uploadId": "s3-multipart-upload-id",
    "objectKey": "channels/550e8400-e29b-41d4-a716-446655440020/attachments/550e8400-e29b-41d4-a716-4466554400f1.bin",
    "chunkSizeBytes": 5242880,
    "chunkCount": 1
  }
}
```

---

### POST `/api/channels/:channelId/attachments/:attachmentId/sign-part`

Obtenir URL signada per pujar un chunk concret al multipart upload.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:**
- `channelId`
- `attachmentId`

**Request Body:**
```json
{
  "uploadId": "s3-multipart-upload-id",
  "partNumber": 1
}
```

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "partNumber": 1,
    "uploadUrl": "https://s3-compatible.local/...signed...",
    "requiredHeaders": {
      "content-type": "application/octet-stream"
    }
  }
}
```

---

### POST `/api/channels/:channelId/attachments/:attachmentId/complete`

Tancar l'upload multipart i persistir metadades criptogràfiques de l'adjunt.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:**
- `channelId`
- `attachmentId`

**Request Body:**
```json
{
  "uploadId": "s3-multipart-upload-id",
  "parts": [
    {
      "partNumber": 1,
      "etag": "\"6805f2cfc46c0f04559748bb039d69ae\""
    }
  ],
  "crypto": {
    "algorithm": "aes-256-gcm",
    "fileIv": "base64-12-byte-nonce",
    "wrappedFileKey": "base64-wrapped-file-key",
    "keyVersionId": "550e8400-e29b-41d4-a716-446655440030",
    "keyVersion": 2,
    "ciphertextSha256": "hex-sha256-ciphertext"
  }
}
```

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "attachmentId": "550e8400-e29b-41d4-a716-4466554400f1",
    "status": "ready"
  }
}
```

---

### GET `/api/channels/:channelId/attachments/:attachmentId/download`

Obtenir URL signada de descàrrega i metadades necessàries per desxifrar al client.

**Headers:** `Authorization: Bearer <JWT>`
**Path Params:**
- `channelId`
- `attachmentId`

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "attachmentId": "550e8400-e29b-41d4-a716-4466554400f1",
    "fileName": "contracte.pdf",
    "mimeType": "application/pdf",
    "sizeBytes": 184223,
    "createdAt": "2026-05-13T10:29:54Z",
    "downloadUrl": "https://s3-compatible.local/...signed...",
    "crypto": {
      "algorithm": "aes-256-gcm",
      "fileIv": "base64-12-byte-nonce",
      "wrappedFileKey": "base64-wrapped-file-key",
      "keyVersionId": "550e8400-e29b-41d4-a716-446655440030",
      "keyVersion": 2,
      "chunkSizeBytes": 5242880,
      "chunkCount": 1,
      "ciphertextSha256": "hex-sha256-ciphertext"
    }
  }
}
```

**Nota de versioning:**

Igual que amb els missatges, si el client no disposa de la clau de `keyVersionId`, l'ha de recuperar explícitament abans de desxifrar l'adjunt.

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

#### POST `/api/dm/channels/:channelId/keys/rotate`

Forçar rotació de clau del DM (crea `keyVersion = N+1`).

**Response 200 OK:**
```json
{
  "success": true,
  "data": {
    "dmChannelId": "550e8400-e29b-41d4-a716-4466554400aa",
    "keyVersionId": "550e8400-e29b-41d4-a716-4466554400bb",
    "keyVersion": 2
  }
}
```

### Nota de consistència de bundles asimètrics

En `POST /api/channels/:id/keys`, la combinació `(keyVersionId, deviceId)` és immutable.

- Si no existeix: s'insereix.
- Si existeix i el payload és idèntic: idempotent (`204`).
- Si existeix i el payload és diferent: `409` amb `ChannelKeyBundleConflict`.

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
| POST | `/api/channels/:channelId/attachments/init` | Sí | Iniciar upload multipart d'adjunt |
| POST | `/api/channels/:channelId/attachments/:attachmentId/sign-part` | Sí | Signar pujada d'un chunk |
| POST | `/api/channels/:channelId/attachments/:attachmentId/complete` | Sí | Completar upload i guardar metadades crypto |
| GET | `/api/channels/:channelId/attachments/:attachmentId/download` | Sí | Obtenir URL signada de descàrrega |
| POST | `/api/direct-messages` | Sí | Enviar DM |
| GET | `/api/direct-messages/list` | Sí | Llistar DMs |
| POST | `/api/dm/channels/:id/keys/rotate` | Sí | Rotar clau DM |
| GET | `/api/conversations` | Sí | Llistar converses |
| POST | `/api/livekit/token` | Sí | Generar token LiveKit |
| GET | `/health` | No | Health check |
