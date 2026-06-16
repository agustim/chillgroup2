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
    #[allow(dead_code)]
    DeviceRevoked,
    #[error("Permisos insuficients")]
    Forbidden,
    #[error("El nom d'usuari ja existeix")]
    UsernameExists,
    #[error("Nom d'usuari invàlid (longitud o caràcters no permesos)")]
    InvalidUsername,
    #[error("Rate limit excedit")]
    #[allow(dead_code)]
    RateLimitExceeded,
    #[error("Password massa feble (mínim {min} caràcters)")]
    WeakPassword { min: usize },
    #[error("El registre està tancat")]
    RegistrationClosed,
    #[error("Codi d'invitació no vàlid o desactivat")]
    InvitationInvalid,
    #[error("Aquesta invitació ha assolit el límit d'usos")]
    InvitationExhausted,
    // Servidors (2000-2099)
    #[error("Servidor no trobat")]
    ServerNotFound,
    #[error("No ets owner/admin d'aquest servidor")]
    #[allow(dead_code)]
    ServerNotOwnerOrAdmin,
    #[error("Ja existeix un servidor amb aquest nom")]
    ServerNameExists,
    #[error("Has arribat al límit de servidors")]
    ServerLimitExceeded,
    #[error("Aquest usuari ja és membre del servidor")]
    MemberExists,
    #[error("Membre no trobat")]
    MemberNotFound,
    #[error("L'owner no pot sortir del servidor. Elimina el servidor o transfereix l'ownership")]
    OwnerCannotLeave,
    #[error("Ets l'últim admin d'aquest servidor. Utilitza ?force=true per confirmar la sortida")]
    ServerLastAdmin,
    // Plans (2600-2699)
    #[error("Plan no trobat")]
    PlanNotFound,
    #[error("Ja existeix un plan amb aquest nom")]
    PlanNameExists,
    #[error("Aquest plan del sistema no es pot modificar")]
    PlanProtected,
    #[error("No es pot eliminar un plan que està assignat a usuaris")]
    PlanInUse,
    // Canals (3000-3099)
    #[error("Canal no trobat")]
    ChannelNotFound,
    #[error("No tens accés a aquest canal")]
    #[allow(dead_code)]
    ChannelAccessDenied,
    #[error("Ja existeix un canal amb aquest nom al servidor")]
    ChannelNameExists,
    #[error("Has arribat al límit de canals")]
    ChannelLimitExceeded,
    #[error("No tens la clau de desxifratge per a aquest canal")]
    ChannelKeyNotFound,
    #[error("Ja existeix un bundle diferent per aquesta versió i dispositiu")]
    ChannelKeyBundleConflict,
    // Missatges (4000-4099)
    #[error("Missatge buit o massa llarg (màxim {max} caràcters)")]
    MessageTooLong { max: usize },
    #[error("Missatge no trobat")]
    MessageNotFound,
    #[error("Adjunt no trobat")]
    AttachmentNotFound,
    #[error("El fitxer supera la mida màxima permesa ({max_mb} MB)")]
    FileTooLarge { max_mb: u64 },
    #[error("Has superat la quota d'emmagatzematge del teu pla")]
    StorageQuotaExceeded,
    #[error("Has superat la quota de transferència mensual del teu pla")]
    TransferQuotaExceeded,
    #[error("Només el remitent pot editar aquest missatge")]
    NotMessageSender,
    #[error("No es pot editar un missatge més enllà de 5 minuts")]
    #[allow(dead_code)]
    EditTimeExceeded,
    #[error("Massis missatges per minut (màxim {max})")]
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    ChannelKeyExpired,
    #[error("Dispositiu revocat per E2EE")]
    #[allow(dead_code)]
    DeviceRevokedE2EE,
    // LiveKit / Veu (6000-6099)
    #[error("Has superat la quota de streaming mensual del teu pla")]
    StreamingQuotaExceeded,
    #[error("Canal no és de veu")]
    #[allow(dead_code)]
    ChannelNotVoice,
    #[error("No pertany al servidor")]
    #[allow(dead_code)]
    NotServerMember,
    #[error("El servei de veu no està disponible ara mateix")]
    #[allow(dead_code)]
    LiveKitUnavailable,
    #[error("Error generant token LiveKit")]
    LiveKitTokenError,
    #[error("Ja estàs en un canal de veu: {current_channel}")]
    #[allow(dead_code)]
    AlreadyInVoiceChannel { current_channel: String },
    // Usuari (continuació)
    #[error("No es pot eliminar l'usuari mentre sigui propietari de servidors")]
    UserOwnsServers,
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
            AppError::InvalidUsername => (
                StatusCode::UNPROCESSABLE_ENTITY, 1013, "Nom d'usuari invàlid: longitud o caràcters no permesos".to_string(), None,
            ),
            AppError::RateLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS, 1009, "Massa intents. Espera uns segons".to_string(), None,
            ),
            AppError::RegistrationClosed => (
                StatusCode::FORBIDDEN, 1010, "El registre està tancat".to_string(), None,
            ),
            AppError::InvitationInvalid => (
                StatusCode::NOT_FOUND, 1011, "Codi d'invitació no vàlid o desactivat".to_string(), None,
            ),
            AppError::InvitationExhausted => (
                StatusCode::GONE, 1012, "Aquesta invitació ha assolit el límit d'usos".to_string(), None,
            ),
            AppError::ServerNotFound => (
                StatusCode::NOT_FOUND, 2001, "Servidor no trobat".to_string(), None,
            ),
            AppError::UserNotFound => (
                StatusCode::NOT_FOUND, 9501, "Usuari no trobat a la base de dades".to_string(), None,
            ),
            AppError::UserOwnsServers => (
                StatusCode::CONFLICT, 9503,
                "No es pot eliminar l'usuari mentre sigui propietari de servidors. Transferiu l'ownership o elimineu els servidors primer.".to_string(),
                None,
            ),
            AppError::ServerNotOwnerOrAdmin => (
                StatusCode::FORBIDDEN, 2002, "No ets owner/admin d'aquest servidor".to_string(), None,
            ),
            AppError::ServerNameExists => (
                StatusCode::CONFLICT, 2003, "Ja existeix un servidor amb aquest nom".to_string(), None,
            ),
            AppError::ServerLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS, 2004, "Has arribat al límit de servidors".to_string(), None,
            ),
            AppError::MemberExists => (
                StatusCode::CONFLICT, 2006, "Aquest usuari ja és membre del servidor".to_string(), None,
            ),
            AppError::MemberNotFound => (
                StatusCode::NOT_FOUND, 2007, "Membre no trobat".to_string(), None,
            ),
            AppError::OwnerCannotLeave => (
                StatusCode::FORBIDDEN, 2008, "L'owner no pot sortir del servidor. Elimina'l o transfereix l'ownership".to_string(), None,
            ),
            AppError::ServerLastAdmin => (
                StatusCode::CONFLICT, 2009, "Ets l'últim admin d'aquest servidor. Utilitza ?force=true per confirmar la sortida".to_string(), None,
            ),
            AppError::PlanNotFound => (
                StatusCode::NOT_FOUND, 2601, "Plan no trobat".to_string(), None,
            ),
            AppError::PlanNameExists => (
                StatusCode::CONFLICT, 2602, "Ja existeix un plan amb aquest nom".to_string(), None,
            ),
            AppError::PlanProtected => (
                StatusCode::FORBIDDEN, 2603, "Aquest plan del sistema no es pot modificar".to_string(), None,
            ),
            AppError::PlanInUse => (
                StatusCode::CONFLICT, 2604, "No es pot eliminar un plan que està assignat a usuaris".to_string(), None,
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
            AppError::ChannelKeyBundleConflict => (
                StatusCode::CONFLICT, 3008, "El bundle de clau ja existeix i no coincideix amb el payload actual".to_string(), None,
            ),
            AppError::ChannelNameExists => (
                StatusCode::CONFLICT, 3004, "Ja existeix un canal amb aquest nom al servidor".to_string(), None,
            ),
            AppError::ChannelLimitExceeded => (
                StatusCode::TOO_MANY_REQUESTS, 3005, "Has arribat al límit de canals".to_string(), None,
            ),
            AppError::MessageTooLong { max } => (
                StatusCode::BAD_REQUEST, 4001, format!("El missatge és massa llarg (màxim {max} caràcters)"),
                Some(serde_json::json!({ "max": max })),
            ),
            AppError::MessageNotFound => (
                StatusCode::NOT_FOUND, 4002, "Missatge no trobat".to_string(), None,
            ),
            AppError::AttachmentNotFound => (
                StatusCode::NOT_FOUND, 4003, "Adjunt no trobat".to_string(), None,
            ),
            AppError::FileTooLarge { max_mb } => (
                StatusCode::PAYLOAD_TOO_LARGE, 4004,
                format!("El fitxer supera la mida màxima de {max_mb} MB"),
                Some(serde_json::json!({ "max_mb": max_mb })),
            ),
            AppError::StorageQuotaExceeded => (
                StatusCode::TOO_MANY_REQUESTS, 4005,
                "Has superat la quota d'emmagatzematge del teu pla".to_string(),
                None,
            ),
            AppError::TransferQuotaExceeded => (
                StatusCode::TOO_MANY_REQUESTS, 4006,
                "Has superat la quota de transferència mensual del teu pla".to_string(),
                None,
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
            AppError::StreamingQuotaExceeded => (
                StatusCode::TOO_MANY_REQUESTS, 6001,
                "Has superat la quota de streaming mensual del teu pla".to_string(),
                None,
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