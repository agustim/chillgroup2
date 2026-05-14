//! Constants globals del domini de ChillGroup.

/// Màxim de caràcters per al nom d'usuari.
pub const MAX_USERNAME_LENGTH: usize = 50;

/// Mínim de caràcters per al nom d'usuari.
pub const MIN_USERNAME_LENGTH: usize = 3;

/// Màxim de caràcters per al password.
pub const MAX_PASSWORD_LENGTH: usize = 128;

/// Mínim de caràcters per al password.
pub const MIN_PASSWORD_LENGTH: usize = 8;

/// Màxim de caràcters per al nom del servidor.
pub const MAX_SERVER_NAME_LENGTH: usize = 100;

/// Màxim de caràcters per al nom del canal.
pub const MAX_CHANNEL_NAME_LENGTH: usize = 100;

/// Màxim de missatges per minut per usuari.
pub const MAX_MESSAGES_PER_MINUTE: u32 = 30;

/// Màxim de servidors per usuari.
pub const MAX_SERVERS_PER_USER: u32 = 10;

/// Màxim de canals per servidor.
pub const MAX_CHANNELS_PER_SERVER: u32 = 50;

/// Temps màxim per editar un missatge (en minuts).
pub const MAX_EDIT_MINUTES: u32 = 5;

/// Màxim de caràcters per a un missatge.
pub const MAX_MESSAGE_LENGTH: usize = 4096;

/// Màxim de dispositius per usuari.
pub const MAX_DEVICES_PER_USER: u32 = 10;

/// Màxim de connexions WebSocket per usuari.
pub const MAX_WEBSOCKET_CONNECTIONS: u32 = 5;

/// Interval de heartbeat WebSocket (segons).
pub const WEBSOCKET_PING_INTERVAL_SECS: u64 = 30;

/// Timeout de inactivitat WebSocket (segons).
pub const WEBSOCKET_TIMEOUT_SECS: u64 = 60;

/// Expiració del token JWT (dies).
pub const JWT_EXPIRATION_DAYS: u32 = 7;

/// Expiració del refresh token (dies).
pub const JWT_REFRESH_EXPIRATION_DAYS: u32 = 30;

/// Limit de paginació per defecte.
pub const DEFAULT_LIMIT: usize = 50;

/// Màxim de paginació.
pub const MAX_LIMIT: usize = 100;

/// Mida de la clau AES-256 (en bytes).
pub const AES_KEY_SIZE: usize = 32;

/// Mida del IV per a AES-GCM (en bytes).
pub const AES_GCM_IV_SIZE: usize = 12;

/// Kyber-1024 public key size (bytes).
pub const KYBER_PUBLIC_KEY_SIZE: usize = 1568;

/// Kyber-1024 secret key size (bytes).
pub const KYBER_SECRET_KEY_SIZE: usize = 3168;

/// Kyber-1024 ciphertext size (bytes).
pub const KYBER_CIPHERTEXT_SIZE: usize = 1088;

/// Salt per a Argon2 (bytes).
pub const ARGON2_SALT_SIZE: usize = 16;

/// Memory cost per a Argon2 (KB).
pub const ARGON2_MEM_COST: u32 = 65536;

/// Time cost per a Argon2.
pub const ARGON2_TIME_COST: u32 = 2;

/// Parallelism per a Argon2.
pub const ARGON2_PARALLELISM: u32 = 1;

/// Rate limit per a login (intents / minuts).
pub const LOGIN_RATE_LIMIT: u32 = 5;
pub const LOGIN_RATE_WINDOW_SECS: u64 = 15 * 60;

/// Extensió del format de clau pública.
pub const PUB_KEY_FORMAT: &str = "base64";