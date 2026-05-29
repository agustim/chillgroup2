# ChillGroup v2 — Gestió d'Erros

## Filosofia

- **Errors consenvents**: Totes les respostes d'error tenen el mateix format JSON
- **Missatges en català**: Els errors es mostren al client en el idioma de l'usuari, però el codi és universal
- **Codis reutilitzables**: Un codi d'error sempre significa la mateixa cosa independentment de l'endpoint
- **Detalls opcionals**: `details` inclou informació extra només quan ajuda a depurar

## Format Global de Resposta d'Error

```json
{
  "success": false,
  "error": {
    "code": 1001,
    "message": "Cone il·legal",
    "details": {}
  }
}
```

### Estructura

| Camp | Tipus | Descripció |
|------|-------|-----------|
| `success` | `false` | Sempre `false` en errors |
| `error.code` | `integer` | Codí d'error universal (veure taula) |
| `error.message` | `string` | Descripció legible en català |
| `error.details` | `object` \| `null` | Dades extra (només quan aplica) |

---

## Taula de Codis d'Error

### Autenticació (1000-1099)

| Codi | HTTP | Significat | `details` possible |
|------|------|-----------|-------------------|
| 1001 | 400 | Validació fallida | `fields: { fieldName: "message" }` |
| 1002 | 401 | Credencials incorrectes | — |
| 1003 | 401 | Token JWT expirat | `expiresAt: "2026-05-01T00:00:00Z"` |
| 1004 | 401 | Token JWT invàlid | `reason: "malformed"` |
| 1005 | 401 | Token JWT no proporcionat | — |
| 1006 | 403 | Dispositiu revocat | `revokedAt: "2026-05-13T10:00:00Z"` |
| 1007 | 403 | Rols insuficients | `required: "admin"`, `current: "member"` |
| 1008 | 409 | Usuari ja existeix | — |
| 1009 | 429 | Rate limit excedit | `retryAfter: 900` (segons) |
| 1010 | 403 | Password feble | `minLength: 8`, `requiresUpper: true` |

**Exemple — 1001 (validació fallida):**
```json
{
  "success": false,
  "error": {
    "code": 1001,
    "message": "Dades de registre invàlides",
    "details": {
      "fields": {
        "username": "El nom d'usuari ha de tenir entre 3 i 50 caràcters, només lletres, números i guions baixos",
        "password": "La contrasenya ha de tenir almenys 8 caràcters"
      }
    }
  }
}
```

**Exemple — 1007 (permsos insuficients):**
```json
{
  "success": false,
  "error": {
    "code": 1007,
    "message": "No tens permís per realitzar aquesta acció",
    "details": {
      "required": "admin",
      "current": "member"
    }
  }
}
```

---

### Servidors (2000-2099)

| Codi | HTTP | Significat | `details` possible |
|------|------|-----------|-------------------|
| 2001 | 404 | Servidor no trobat | — |
| 2002 | 403 | No és owner/admin | — |
| 2003 | 409 | Nom de servidor ja existeix | — |
| 2004 | 400 | Nom de servidor invàlid | `maxChars: 100` |
| 2005 | 403 | Has arribat al límit de servidors | `maxServers: 10`, `current: 10` |
| 2006 | 409 | Membre ja existeix | `username: "marcus"` |
| 2007 | 404 | Membre no trobat | `userId: "uuid"` |

**Exemple — 2005 (límit servidors):**
```json
{
  "success": false,
  "error": {
    "code": 2005,
    "message": "Has arribat al límit de servidors",
    "details": {
      "maxServers": 10,
      "current": 10
    }
  }
}
```

---

### Canals (3000-3099)

| Codi | HTTP | Significat | `details` possible |
|------|------|-----------|-------------------|
| 3001 | 404 | Canal no trobat | — |
| 3002 | 403 | No tens accés al canal | — |
| 3003 | 400 | Nom de canal invàlid | `maxChars: 100` |
| 3004 | 409 | Nom de canal ja existeix al servidor | — |
| 3005 | 403 | Has arribat al límit de canals | `maxChannels: 50`, `current: 50` |
| 3006 | 400 | Tipus de canal no vàlid | — |
| 3007 | 403 | Clau de canal no trobada | `channelId: "uuid"` |
| 3008 | 409 | Conflicte de bundle de clau de canal | `keyVersionId: "uuid"`, `deviceId: "uuid"` |
| 3009 | 400 | TTL invàlid | `min: 3600`, `max: 604800` |
| 3010 | 403 | No es pot convidar a aquest usuari | `username: "marcus"` |

**Exemple — 3002 (sense accés):**
```json
{
  "success": false,
  "error": {
    "code": 3002,
    "message": "No tens accés a aquest canal",
    "details": {}
  }
}
```

**Exemple — 3007 (clau no trobada):**
```json
{
  "success": false,
  "error": {
    "code": 3007,
    "message": "No tens la clau de desxifratge per a aquest canal",
    "details": {
      "channelId": "550e8400-e29b-41d4-a716-446655440021"
    }
  }
}
```

---

### Missatges (4000-4099)

| Codi | HTTP | Significat | `details` possible |
|------|------|-----------|-------------------|
| 4001 | 400 | Missatge buit o massa llarg | `maxLength: 4096` |
| 4002 | 404 | Missatge no trobat | — |
| 4003 | 403 | No és el remitent del missatge | — |
| 4004 | 403 | Temps d'edició excedit | `maxEditMinutes: 5` |
| 4005 | 403 | No pots eliminar missatges aliens | — |
| 4006 | 429 | Massis missatges per minut | `maxPerMinute: 30`, `retryAfter: 60` |
| 4007 | 403 | Canal expirat | — |

**Exemple — 4006 (rate limit):**
```json
{
  "success": false,
  "error": {
    "code": 4006,
    "message": "Massis missatges. Espera abans d'enviar-ne més",
    "details": {
      "maxPerMinute": 30,
      "retryAfter": 60
    }
  }
}
```

**Exemple — 4004 (temps d'edició):**
```json
{
  "success": false,
  "error": {
    "code": 4004,
    "message": "No es pot editar un missatge més enllà de 5 minuts",
    "details": {
      "maxEditMinutes": 5
    }
  }
}
```

---

### Encriptació (5000-5099)

| Codi | HTTP | Significat | `details` possible |
|------|------|-----------|-------------------|
| 5001 | 400 | Clau pública Kyber no trobada | `deviceId: "uuid"` |
| 5002 | 400 | Dispositiu sense publicKey | `deviceId: "uuid"` |
| 5003 | 400 | Desencriptació fallida | — |
| 5004 | 400 | Encapsulació KEM fallida | — |
| 5005 | 400 | Clau de canal ja existeix | `channelId: "uuid"` |
| 5006 | 400 | Clau de canal expirada | — |
| 5007 | 400 | Format de clau invàlid | `expectedFormat: "base64"` |
| 5008 | 403 | Dispositiu revocat per E2EE | — |

**Exemple — 5001 (publicKey no trobada):**
```json
{
  "success": false,
  "error": {
    "code": 5001,
    "message": "Aquest dispositiu no té una clau pública registrada",
    "details": {
      "deviceId": "550e8400-e29b-41d4-a716-446655440002"
    }
  }
}
```

**Exemple — 5003 (desencriptació fallida):**
```json
{
  "success": false,
  "error": {
    "code": 5003,
    "message": "No s'ha pogut desencriptar el missatge. Intenta obtenir la clau del canal de nou",
    "details": {}
  }
}
```

---

### LiveKit / Veu (6000-6099)

| Codi | HTTP | Significat | `details` possible |
|------|------|-----------|-------------------|
| 6001 | 400 | Canal no és de veu | — |
| 6002 | 403 | No pertany al servidor | — |
| 6003 | 502 | LiveKit no disponible | — |
| 6004 | 403 | Ja estàs en un canal de veu | `currentChannel: "uuid"` |
| 6005 | 502 | Error generant token LiveKit | — |

**Exemple — 6004 (ja en veu):**
```json
{
  "success": false,
  "error": {
    "code": 6004,
    "message": "Ja estàs en un canal de veu",
    "details": {
      "currentChannel": "550e8400-e29b-41d4-a716-446655440021",
      "currentChannelName": "Sala de música"
    }
  }
}
```

**Exemple — 6003 (LiveKit down):**
```json
{
  "success": false,
  "error": {
    "code": 6003,
    "message": "El servei de veu no està disponible ara mateix. Torna-ho a provar més tard",
    "details": {}
  }
}
```

---

### Sistema (9000-9099)

| Codi | HTTP | Significat | `details` possible |
|------|------|-----------|-------------------|
| 9001 | 500 | Error intern del servidor | `traceId: "abc123"` |
| 9002 | 503 | Base de dades no disponible | — |
| 9003 | 503 | Servei no disponible | — |
| 9004 | 413 | Fitxer massa gran | `maxSize: "10MB"` |
| 9005 | 415 | Tipus de contingut no suportat | — |
| 9006 | 400 | Camp desconegut al request | `unknownFields: ["foo"]` |

**Exemple — 9001 (error intern):**
```json
{
  "success": false,
  "error": {
    "code": 9001,
    "message": "S'ha produït un error intern. Ho sentim",
    "details": {
      "traceId": "abc123-def456"
    }
  }
}
```

---

## Gestió d'Errors a Rust (Server)

### Error Type Unificat

```rust
// server/src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Credencials incorrectes")]
    UnauthorizedCredentials,

    #[error("Token expirat")]
    TokenExpired,

    #[error("Token invàlid: {reason}")]
    TokenInvalid { reason: String },

    #[error("No s'ha proporcionat token")]
    TokenMissing,

    #[error("Dispositiu revocat")]
    DeviceRevoked,

    #[error("Permisos insuficients: es requereix {required}, tens {current}")]
    Forbidden { required: String, current: String },

    #[error("Nom d'usuari ja existeix")]
    UsernameExists,

    #[error("Rate limit excedit")]
    RateLimitExceeded,

    #[error("Servidor no trobat")]
    ServerNotFound,

    #[error("No ets owner/admin d'aquest servidor")]
    ServerNotOwner,

    #[error("Nom de servidor ja existeix")]
    ServerNameExists,

    #[error("Has arribat al límit de servidors: {max}/{current}")]
    ServerLimitExceeded { max: u32, current: u32 },

    #[error("Membre ja existeix al servidor")]
    MemberExists,

    #[error("Membre no trobat")]
    MemberNotFound,

    #[error("Canal no trobat")]
    ChannelNotFound,

    #[error("No tens accés a aquest canal")]
    ChannelAccessDenied,

    #[error("Nom de canal ja existeix al servidor")]
    ChannelNameExists,

    #[error("Has arribat al límit de canals: {max}/{current}")]
    ChannelLimitExceeded { max: u32, current: u32 },

    #[error("Clau de canal no trobada")]
    ChannelKeyNotFound,

    #[error("Error encriptant/desencriptant")]
    CryptoError(#[from] CryptoError),

    #[error("Clau pública Kyber no trobada per al dispositiu")]
    PublicKeyNotFound,

    #[error("Dispositiu sense publicKey")]
    DeviceNoPublicKey,

    #[error("Desencriptació fallida")]
    DecryptionFailed,

    #[error("Encapsulació KEM fallida")]
    EncapsulationFailed,

    #[error("Clau de canal expirada")]
    ChannelKeyExpired,

    #[error("Ja estàs en un canal de veu: {current_channel}")]
    AlreadyInVoiceChannel { current_channel: String },

    #[error("LiveKit no disponible")]
    LiveKitUnavailable,

    #[error("Error token LiveKit")]
    LiveKitTokenError,

    #[error("Error intern del servidor")]
    InternalError,

    #[error("Base de dades no disponible")]
    DatabaseUnavailable,
}

impl AppError {
    /// Convertir a HTTP response (Axum)
    pub fn into_response(self) -> (StatusCode, Json<ErrorResponse>) {
        let (status, code, message, details) = match &self {
            AppError::UnauthorizedCredentials => (
                StatusCode::UNAUTHORIZED,
                1002,
                "Credencials incorrectes".to_string(),
                None,
            ),
            AppError::TokenExpired => (
                StatusCode::UNAUTHORIZED,
                1003,
                "Token JWT expirat".to_string(),
                None,
            ),
            AppError::TokenInvalid { reason } => (
                StatusCode::UNAUTHORIZED,
                1004,
                "Token JWT invàlid".to_string(),
                Some(serde_json::json!({ "reason": reason })),
            ),
            AppError::TokenMissing => (
                StatusCode::UNAUTHORIZED,
                1005,
                "No s'ha proporcionat token".to_string(),
                None,
            ),
            AppError::DeviceRevoked => (
                StatusCode::FORBIDDEN,
                1006,
                "Dispositiu revocat".to_string(),
                None,
            ),
            AppError::Forbidden { required, current } => (
                StatusCode::FORBIDDEN,
                1007,
                "No tens permís per realitzar aquesta acció".to_string(),
                Some(serde_json::json!({
                    "required": required,
                    "current": current,
                })),
            ),
            AppError::UsernameExists => (
                StatusCode::CONFLICT,
                1008,
                "El nom d'usuari ja existeix".to_string(),
                None,
            ),
            AppError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS,
                1009,
                "Massis intents. Espera uns segons".to_string(),
                None,
            ),
            AppError::ServerNotFound => (
                StatusCode::NOT_FOUND,
                2001,
                "Servidor no trobat".to_string(),
                None,
            ),
            AppError::ServerNotOwner => (
                StatusCode::FORBIDDEN,
                2002,
                "No ets owner/admin d'aquest servidor".to_string(),
                None,
            ),
            AppError::ServerNameExists => (
                StatusCode::CONFLICT,
                2003,
                "Ja existeix un servidor amb aquest nom".to_string(),
                None,
            ),
            AppError::ServerLimitExceeded { max, current } => (
                StatusCode::FORBIDDEN,
                2005,
                "Has arribat al límit de servidors".to_string(),
                Some(serde_json::json!({ "maxServers": max, "current": current })),
            ),
            AppError::MemberExists => (
                StatusCode::CONFLICT,
                2006,
                "Aquest usuari ja és membre del servidor".to_string(),
                None,
            ),
            AppError::MemberNotFound => (
                StatusCode::NOT_FOUND,
                2007,
                "Membre no trobat".to_string(),
                None,
            ),
            AppError::ChannelNotFound => (
                StatusCode::NOT_FOUND,
                3001,
                "Canal no trobat".to_string(),
                None,
            ),
            AppError::ChannelAccessDenied => (
                StatusCode::FORBIDDEN,
                3002,
                "No tens accés a aquest canal".to_string(),
                None,
            ),
            AppError::ChannelNameExists => (
                StatusCode::CONFLICT,
                3004,
                "Ja existeix un canal amb aquest nom al servidor".to_string(),
                None,
            ),
            AppError::ChannelLimitExceeded { max, current } => (
                StatusCode::FORBIDDEN,
                3005,
                "Has arribat al límit de canals".to_string(),
                Some(serde_json::json!({ "maxChannelsPerServer": max, "current": current })),
            ),
            AppError::ChannelKeyNotFound => (
                StatusCode::FORBIDDEN,
                3007,
                "No tens la clau de desxifratge per a aquest canal".to_string(),
                None,
            ),
            AppError::MessageTooLong { max_length } => (
                StatusCode::BAD_REQUEST,
                4001,
                "El missatge és massa llarg".to_string(),
                Some(serde_json::json!({ "maxLength": max_length })),
            ),
            AppError::MessageNotFound => (
                StatusCode::NOT_FOUND,
                4002,
                "Missatge no trobat".to_string(),
                None,
            ),
            AppError::NotMessageSender => (
                StatusCode::FORBIDDEN,
                4003,
                "Només el remitent pot editar aquest missatge".to_string(),
                None,
            ),
            AppError::EditTimeExceeded => (
                StatusCode::FORBIDDEN,
                4004,
                "No es pot editar un missatge més enllà de 5 minuts".to_string(),
                Some(serde_json::json!({ "maxEditMinutes": 5 })),
            ),
            AppError::MessageRateLimited { max_per_minute, retry_after } => (
                StatusCode::TOO_MANY_REQUESTS,
                4006,
                "Massis missatges per minut".to_string(),
                Some(serde_json::json!({
                    "maxPerMinute": max_per_minute,
                    "retryAfter": retry_after,
                })),
            ),
            AppError::PublicKeyNotFound => (
                StatusCode::BAD_REQUEST,
                5001,
                "Aquest dispositiu no té una clau pública registrada".to_string(),
                None,
            ),
            AppError::DeviceNoPublicKey => (
                StatusCode::BAD_REQUEST,
                5002,
                "Aquest dispositiu no té clau pública".to_string(),
                None,
            ),
            AppError::DecryptionFailed => (
                StatusCode::BAD_REQUEST,
                5003,
                "No s'ha pogut desencriptar el missatge".to_string(),
                None,
            ),
            AppError::EncapsulationFailed => (
                StatusCode::BAD_REQUEST,
                5004,
                "Error en el procés d'encriptació".to_string(),
                None,
            ),
            AppError::ChannelKeyExpired => (
                StatusCode::BAD_REQUEST,
                5006,
                "La clau de canal ha expirat".to_string(),
                None,
            ),
            AppError::AlreadyInVoiceChannel { current_channel } => (
                StatusCode::FORBIDDEN,
                6004,
                "Ja estàs en un canal de veu".to_string(),
                Some(serde_json::json!({ "currentChannel": current_channel })),
            ),
            AppError::LiveKitUnavailable => (
                StatusCode::BAD_GATEWAY,
                6003,
                "El servei de veu no està disponible ara mateix".to_string(),
                None,
            ),
            AppError::InternalError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                9001,
                "S'ha produït un error intern".to_string(),
                None,
            ),
            AppError::DatabaseUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                9002,
                "Base de dades no disponible".to_string(),
                None,
            ),
        };

        let response = ErrorResponse {
            code,
            message,
            details,
        };

        (status, Json(ErrorResponse::into_json(response)))
    }
}
```

### Axum Handler Error

```rust
// server/src/middleware/error_handler.rs

use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

pub async fn handle_error(
    req: Request,
    err: AppError,
) -> Response {
    // Loggejar l'error amb tracing
    error!(
        endpoint = req.uri().to_string(),
        error_code = err.error_code(),
        message = %err,
        "Request error"
    );

    // Convertir a resposta HTTP
    err.into_response()
}
```

---

## Gestió d'Errors a Frontend (TypeScript)

### Tipus d'Error

```typescript
// frontend/src/lib/errors.ts

export interface ApiError {
  success: false
  error: {
    code: number
    message: string
    details?: Record<string, unknown>
  }
}

export class ChillGroupError extends Error {
  constructor(
    public readonly code: number,
    message: string,
    public readonly details?: Record<string, unknown>
  ) {
    super(message)
    this.name = 'ChillGroupError'
  }
}

// Mapping de codis d'error a missatges de UI en català
const ERROR_MESSAGES: Record<number, string> = {
  1001: 'Dades invàlides',
  1002: 'Credencials incorrectes',
  1003: 'Sessió expirada. Inicia sessió de nou',
  1004: 'Error d\'autenticació',
  1006: 'Dispositiu revocat',
  1007: 'No tens permís per fer això',
  1008: 'Aquest usuari ja existeix',
  1009: 'Massis intents. Espera uns segons',
  2001: 'Servidor no trobat',
  2002: 'No ets owner/admin d\'aquest servidor',
  2003: 'Ja existeix un servidor amb aquest nom',
  2005: 'Has arribat al límit de servidors',
  3001: 'Canal no trobat',
  3002: 'No tens accés a aquest canal',
  3004: 'Ja existeix un canal amb aquest nom',
  3005: 'Has arribat al límit de canals',
  3007: 'No tens la clau del canal',
  4001: 'Missatge buit o massa llarg',
  4003: 'Només el remitent pot editar el missatge',
  4004: 'Massa temps des de l\'enviament per editar',
  4006: 'Massis missatges. Espera una mica',
  5001: 'Dispositiu sense clau pública',
  5003: 'No es pot desencriptar aquest missatge',
  6003: 'Servei de veu no disponible',
  6004: 'Ja estàs en un canal de veu',
  9001: 'Error intern. Intenta-ho de nou',
  9002: 'Base de dades no disponible',
}

export function parseApiError(response: Response): ChillGroupError {
  return response.json().then((data) => {
    return new ChillGroupError(
      data.error.code,
      data.error.message || ERROR_MESSAGES[data.error.code] || 'Error desconegut',
      data.error.details
    )
  })
}
```

### Mostrar Error al Client

```typescript
// frontend/src/lib/api.ts

async function request<T>(url: string, options?: RequestInit): Promise<T> {
  const response = await fetch(url, options)

  if (!response.ok) {
    // Parsejar error de l'API
    const error = await parseApiError(response)

    // Mostrar notificación al usuari
    showNotification(error.message, getNotificationLevel(error.code))
    throw error
  }

  const data = await response.json()
  return data.data  // Desempacar el camp "data"
}

function getNotificationLevel(code: number): 'error' | 'warning' | 'info' {
  if (code >= 1000 && code < 2000) return 'error'
  if (code >= 2000 && code < 3000) return 'error'
  if (code >= 3000 && code < 4000) return 'error'
  if (code >= 4000 && code < 5000) return 'warning'
  if (code >= 5000 && code < 6000) return 'warning'
  if (code >= 6000 && code < 7000) return 'error'
  return 'error'
}

// Notificació UI
function showNotification(message: string, level: 'error' | 'warning' | 'info') {
  // Envia event al React
  window.dispatchEvent(new CustomEvent('chillgroup-notification', {
    detail: { message, level, id: crypto.randomUUID() }
  }))
}
```

---

## Gestió d'Errors WebSocket

### Errors de Connexió

```typescript
// frontend/src/lib/socket.ts

socket.on('connection-error', (data) => {
  // { code, message }
  const error = new ChillGroupError(data.code, data.message)
  showNotification(error.message, 'error')

  if (data.code === 1003 || data.code === 1004) {
    // Token expirat o invàlid → redirigir a login
    window.location.href = '/login'
  }
})

socket.on('permission-denied', (data) => {
  // { code, message, channelId }
  const error = new ChillGroupError(data.code, data.message)
  showNotification(error.message, 'warning')

  // Si era un canal de veu, desconectar
  if (data.channelId) {
    leaveVoiceChannel()
  }
})
```

### Codi d'Error WebSocket

| Codi | HTTP equivalent | Significat | Acció frontend |
|------|----------------|-----------|---------------|
| 4000 | 200 | Tancament normal | — |
| 4001 | 401 | Token invàlid/expirat | Redirigir a login |
| 4002 | 403 | Dispositiu revocat | Redirigir a login |
| 4003 | 403 | Permisos insuficients | Mostrar warning, sortir del canal |
| 4004 | 404 | Canal no trobat | Treure de la llista |
| 4005 | 429 | Rate limit | Mostrar timer |
| 4006 | 503 | Servidor ple | Reintentar en 5s |

---

## Resum de Codis per Mòdul

| Mòdul | Rang | Total codis |
|-------|------|------------|
| Autenticació | 1001-1010 | 10 |
| Servidors | 2001-2007 | 7 |
| Canals | 3001-3010 | 10 |
| Missatges | 4001-4007 | 7 |
| Encriptació | 5001-5008 | 8 |
| LiveKit/Veu | 6001-6005 | 5 |
| Sistema | 9001-9006 | 6 |
| **Total** | | **53** |
