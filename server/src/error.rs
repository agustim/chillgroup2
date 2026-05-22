//! Gestió unificada d'errors del servidor.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use tracing::error;
use serde::Serialize;

// ── Errors criptogràfics ────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum CryptoError {
    #[error("Format base64 invàlid")]
    Base64(#[from] base64::DecodeError),
    #[error("Error UTF-8")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("Mida de clau incorrecta: {expected} bytes, rebuts {actual}")]
    InvalidKeySize { expected: usize, actual: usize },
    #[error("Error criptogràfic")]
    CryptoError,
}

// ── Error principal de l'aplicació ──────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // Usuari (950-959)
    #[error("Usuari no trobat")]
    UserNotFound,
    // Autenticació (1000-1099)
    #[error("Credencials incorrectes")]
    UnauthorizedCredentials,
    #[error("Token expirat")]
    TokenExpired,
    #[error("Token invàlid")]
    TokenInvalid,
    #[error("No s'ha proporcionat token")]
    TokenMissing,
    #[error("Dispositiu revocat")]
    DeviceRevoked,
    #[error("Permisos insuficients")]
    Forbidden,
    #[error("El nom d'usuari ja existeix")]
    UsernameExists,
    #[error("Rate limit excedit")]
    RateLimitExceeded,
    #[error("Password massa feble (mínim {min} caràcters)")]
    WeakPassword { min: usize },
    // Servidors (2000-2099)
    #[error("Servidor no trobat")]
    ServerNotFound,
    #[error("No ets owner/admin d'aquest servidor")]
    ServerNotOwnerOrAdmin,
    #[error("Ja existeix un servidor amb aquest nom")]
    ServerNameExists,
    #[error("Has arribat al límit de servidors")]
    ServerLimitExceeded,
    #[error("Aquest usuari ja és membre del servidor")]
    MemberExists,
    #[error("Membre no trobat")]
    MemberNotFound,
    // Canals (3000-3099)
    #[error("Canal no trobat")]
    ChannelNotFound,
    #[error("No tens accés a aquest canal")]
    ChannelAccessDenied,
    #[error("Ja existeix un canal amb aquest nom al servidor")]
    ChannelNameExists,
    #[error("Has arribat al límit de canals")]
    ChannelLimitExceeded,
    #[error("No tens la clau de desxifratge per a aquest canal")]
    ChannelKeyNotFound,
    // Missatges (4000-4099)
    #[error("Missatge buit o massa llarg (màxim {max} caràcters)")]
    MessageTooLong { max: usize },
    #[error("Missatge no trobat")]
    MessageNotFound,
    #[error("Només el remitent pot editar aquest missatge")]
    NotMessageSender,
    #[error("No es pot editar un missatge més enllà de 5 minuts")]
    EditTimeExceeded,
    #[error("Massis missatges per minut (màxim {max})")]
    MessageRateLimited { max: u32 },
    // Encriptació (5000-5099)
    #[error("Clau pública no trobada per al dispositiu")]
    PublicKeyNotFound,
    #[error("Dispositiu sense clau pública")]
    DeviceNoPublicKey,
    #[error("No s'ha pogut desencriptar el missatge")]
    DecryptionFailed,
    #[error("Error en el procés d'encriptació")]
    EncapsulationFailed,
    #[error("La clau de canal ha expirat")]
    ChannelKeyExpired,
    #[error("Dispositiu revocat per E2EE")]
    DeviceRevokedE2EE,
    // LiveKit / Veu (6000-6099)
    #[error("Canal no és de veu")]
    ChannelNotVoice,
    #[error("No pertany al servidor")]
    NotServerMember,
    #[error("El servei de veu no està disponible ara mateix")]
    LiveKitUnavailable,
    #[error("Error generant token LiveKit")]
    LiveKitTokenError,
    #[error("Ja estàs en un canal de veu: {current_channel}")]
    AlreadyInVoiceChannel { current_channel: String },
    // Sistema (9000-9099)
    #[error("Petició incorrecta")]
    BadRequest,
    #[error("Error intern del servidor")]
    InternalError,
    #[error("Base de dades no disponible")]
    DatabaseUnavailable,
    #[error("Error de base de dades: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Error criptogràfic: {0}")]
    Crypto(#[from] CryptoError),
}

#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct ErrorResponse {
    pub code: i16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message, details) = match &self {
            AppError::UnauthorizedCredentials => (
                StatusCode::UNAUTHORIZED, 1002, "Credencials incorrectes".to_string(), None,
            ),
            AppError::TokenExpired => (
                StatusCode::UNAUTHORIZED, 1003, "Token JWT expirat".to_string(), None,
            ),
            AppError::TokenInvalid => (
                StatusCode::UNAUTHORIZED, 1004, "Token JWT invàlid".to_string(), None,
            ),
            AppError::TokenMissing => (
                StatusCode::UNAUTHORIZED, 1005, "No s'ha proporcionat token".to_string(), None,
            ),
            AppError::DeviceRevoked => (
                StatusCode::FORBIDDEN, 1006, "Dispositiu revocat".to_string(), None,
            ),
            AppError::Forbidden => (
                StatusCode::FORBIDDEN, 1007, "No tens permís per realitzar aquesta acció".to_string(),
                None,
            ),
            AppError::UsernameExists => (
                StatusCode::CONFLICT, 1008, "El nom d'usuari ja existeix".to_string(), None,
            ),
            AppError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS, 1009, "Massa intents. Espera uns segons".to_string(), None,
            ),
            AppError::ServerNotFound => (
                StatusCode::NOT_FOUND, 2001, "Servidor no trobat".to_string(), None,
            ),
            AppError::UserNotFound => (
                StatusCode::NOT_FOUND, 9501, "Usuari no trobat a la base de dades".to_string(), None,
            ),
            AppError::ServerNotOwnerOrAdmin => (
                StatusCode::FORBIDDEN, 2002, "No ets owner/admin d'aquest servidor".to_string(), None,
            ),
            AppError::ServerNameExists => (
                StatusCode::CONFLICT, 2003, "Ja existeix un servidor amb aquest nom".to_string(), None,
            ),
            AppError::MemberExists => (
                StatusCode::CONFLICT, 2006, "Aquest usuari ja és membre del servidor".to_string(), None,
            ),
            AppError::MemberNotFound => (
                StatusCode::NOT_FOUND, 2007, "Membre no trobat".to_string(), None,
            ),
            AppError::ChannelNotFound => (
                StatusCode::NOT_FOUND, 3001, "Canal no trobat".to_string(), None,
            ),
            AppError::ChannelAccessDenied => (
                StatusCode::FORBIDDEN, 3002, "No tens accés a aquest canal".to_string(), None,
            ),
            AppError::ChannelKeyNotFound => (
                StatusCode::NOT_FOUND, 3007, "No tens la clau de desxifratge per a aquest canal".to_string(), None,
            ),
            AppError::ChannelNameExists => (
                StatusCode::CONFLICT, 3004, "Ja existeix un canal amb aquest nom al servidor".to_string(), None,
            ),
            AppError::MessageTooLong { max } => (
                StatusCode::BAD_REQUEST, 4001, format!("El missatge és massa llarg (màxim {max} caràcters)"),
                None,
            ),
            AppError::MessageNotFound => (
                StatusCode::NOT_FOUND, 4002, "Missatge no trobat".to_string(), None,
            ),
            AppError::PublicKeyNotFound => (
                StatusCode::BAD_REQUEST, 5001, "Aquest dispositiu no té una clau pública registrada".to_string(), None,
            ),
            AppError::DeviceNoPublicKey => (
                StatusCode::FORBIDDEN, 5002, "El dispositiu actual encara no té clau pública registrada".to_string(), None,
            ),
            AppError::DecryptionFailed => (
                StatusCode::BAD_REQUEST, 5003, "No s'ha pogut desencriptar el missatge".to_string(), None,
            ),
            AppError::EncapsulationFailed => (
                StatusCode::BAD_REQUEST, 5004, "No s'ha pogut encapsular la clau del canal per aquest dispositiu".to_string(), None,
            ),
            AppError::LiveKitUnavailable => (
                StatusCode::BAD_GATEWAY, 6003, "El servei de veu no està disponible ara mateix".to_string(), None,
            ),
            AppError::AlreadyInVoiceChannel { current_channel } => (
                StatusCode::FORBIDDEN, 6004, "Ja estàs en un canal de veu".to_string(),
                Some(serde_json::json!({"currentChannel": current_channel})),
            ),
            AppError::BadRequest => (
                StatusCode::BAD_REQUEST, 4000, "Petició incorrecta".to_string(), None,
            ),
            AppError::InternalError | AppError::DatabaseError(_) | AppError::Crypto(_) => (
                StatusCode::INTERNAL_SERVER_ERROR, 9001, "S'ha produït un error intern".to_string(), None,
            ),
            AppError::DatabaseUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE, 9002, "Base de dades no disponible".to_string(), None,
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR, 9001, "S'ha produït un error intern".to_string(), None,
            ),
        };

        error!("Error {}: {}", code, message);

        let body = Json(serde_json::json!({
            "success": false,
            "error": { "code": code, "message": message, "details": details }
        }));

        (status, body).into_response()
    }
}