//! Configuració de l'aplicació carregada des de variables d'entorn.

use std::env;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

impl LogLevel {
    pub fn as_tracing_filter(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            other => Err(format!(
                "BACKEND_DEBUG invàlid: '{}'. Valors permesos: error, warn, info, debug",
                other
            )),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub backend_debug: LogLevel,
    pub database_url: String,
    pub open_register: bool,
    pub admin_user: Option<String>,
    pub admin_password: Option<String>,
    pub ttl_cleanup_interval_minutes: u64,
    pub livekit_host: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    pub jwt_secret: String,
    pub jwt_expiration_days: u32,
    pub server_master_key: [u8; 32],
    pub static_dir: Option<String>,
    /// Mida màxima permesa per fitxer en bytes. 0 = sense límit. Per defecte: 104857600 (100 MB).
    pub max_file_size_bytes: u64,
    /// Orígens permesos per CORS (comma-separated). Buit = refusa tots.
    pub allowed_origins: Vec<String>,
    /// Base URL de l'endpoint compatible amb OpenAI per a l'assistent de veu.
    pub assistant_openai_base_url: String,
    /// API key per a l'endpoint de l'assistent (None = sense capçalera Authorization).
    pub assistant_openai_api_key: Option<String>,
    /// Model de transcripció (STT). Per defecte: whisper-1.
    pub assistant_stt_model: String,
    /// Model de resum (chat). Per defecte: gpt-4o-mini.
    pub assistant_summary_model: String,
    /// Idioma opcional (hint per Whisper, p.ex. "ca", "es", "en").
    pub assistant_language: Option<String>,
}

fn decode_hex_key_32(value: &str) -> Result<[u8; 32], String> {
    let trimmed = value.trim();
    if trimmed.len() != 64 {
        return Err("SERVER_MASTER_KEY ha de tenir 64 caracters hex (32 bytes)".to_string());
    }

    let mut out = [0u8; 32];
    for (i, chunk) in trimmed.as_bytes().chunks(2).enumerate() {
        let hex = std::str::from_utf8(chunk).map_err(|_| "SERVER_MASTER_KEY conté bytes invàlids")?;
        out[i] = u8::from_str_radix(hex, 16)
            .map_err(|_| "SERVER_MASTER_KEY no és hex vàlid")?;
    }
    Ok(out)
}

fn normalize_sqlite_url(value: &str) -> String {
    if !value.starts_with("sqlite") || value.starts_with("sqlite::memory:") || value.contains("mode=") {
        return value.to_string();
    }

    let separator = if value.contains('?') { '&' } else { '?' };
    format!("{}{}mode=rwc", value, separator)
}

impl Config {
    /// Carregar configuració des de variables d'entorn i .env.
    /// Retorna el Config i la ruta del fitxer .env carregat (si s'ha trobat).
    pub fn from_env(config_location: Option<&str>) -> Result<(Self, Option<PathBuf>), String> {
        let env_path = match config_location {
            Some(location) => {
                let path = PathBuf::from(location);
                if path.is_dir() {
                    path.join(".env")
                } else {
                    path
                }
            }
            None => env::current_dir()
                .map_err(|e| format!("No s'ha pogut obtenir el directori actual: {}", e))?
                .join(".env"),
        };

        let loaded_env_path = if env_path.exists() {
            dotenvy::from_path(&env_path).map_err(|e| format!("Error carregant .env: {}", e))?;
            Some(env_path)
        } else {
            None
        };

        fn get_var(key: &str) -> Result<String, String> {
            env::var(key).map_err(|_| format!("La variable d'entorn {} és obligatòria", key))
        }

        fn get_bool_var(key: &str, default: bool) -> Result<bool, String> {
            match env::var(key) {
                Ok(value) => match value.trim().to_ascii_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => Ok(true),
                    "false" | "0" | "no" | "off" => Ok(false),
                    _ => Err(format!("La variable {} ha de ser true/false", key)),
                },
                Err(_) => Ok(default),
            }
        }

        let open_register = get_bool_var("OPEN_REGISTER", true)?;
        let admin_user = env::var("ADMIN_USER").ok().filter(|s| !s.trim().is_empty());
        let admin_password = env::var("ADMIN_PASSWORD").ok().filter(|s| !s.trim().is_empty());

        if !open_register && (admin_user.is_none() || admin_password.is_none()) {
            return Err(
                "Quan OPEN_REGISTER=false, ADMIN_USER i ADMIN_PASSWORD són obligatoris".to_string(),
            );
        }

        Ok((Self {
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            server_port: env::var("SERVER_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            backend_debug: env::var("BACKEND_DEBUG")
                .ok()
                .map(|level| level.parse())
                .transpose()?
                .unwrap_or(LogLevel::Info),
            database_url: get_var("DATABASE_URL")?,
            open_register,
            admin_user,
            admin_password,
            ttl_cleanup_interval_minutes: env::var("TTL_CLEANUP_INTERVAL_MINUTES")
                .ok()
                .and_then(|i| i.parse().ok())
                .unwrap_or(5),
            livekit_host: get_var("LIVEKIT_HOST")?,
            livekit_api_key: get_var("LIVEKIT_API_KEY")?,
            livekit_api_secret: get_var("LIVEKIT_API_SECRET")?,
            jwt_secret: {
                let secret = get_var("JWT_SECRET")?;
                if secret.len() < 32 {
                    return Err("JWT_SECRET ha de tenir almenys 32 caràcters".to_string());
                }
                secret
            },
            jwt_expiration_days: env::var("JWT_EXPIRATION_DAYS")
                .ok()
                .and_then(|d| d.parse().ok())
                .unwrap_or(7),
            server_master_key: decode_hex_key_32(&get_var("SERVER_MASTER_KEY")?)?,
            static_dir: env::var("STATIC_DIR").ok().filter(|s| !s.trim().is_empty()),
            max_file_size_bytes: env::var("MAX_FILE_SIZE")
                .ok()
                .and_then(|v| v.trim().parse::<u64>().ok())
                .unwrap_or(100 * 1024 * 1024), // 100 MB per defecte
            allowed_origins: env::var("ALLOWED_ORIGINS")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            assistant_openai_base_url: env::var("ASSISTANT_OPENAI_BASE_URL")
                .ok()
                .map(|s| s.trim().trim_end_matches('/').to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
            assistant_openai_api_key: env::var("ASSISTANT_OPENAI_API_KEY")
                .ok()
                .filter(|s| !s.trim().is_empty()),
            assistant_stt_model: env::var("ASSISTANT_STT_MODEL")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "whisper-1".to_string()),
            assistant_summary_model: env::var("ASSISTANT_SUMMARY_MODEL")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "gpt-4o-mini".to_string()),
            assistant_language: env::var("ASSISTANT_LANGUAGE")
                .ok()
                .filter(|s| !s.trim().is_empty()),
        }, loaded_env_path))
    }

    /// Comprovar si és SQLite.
    pub fn is_sqlite(&self) -> bool {
        self.database_url.starts_with("sqlite")
    }

    /// Retorna la URL de SQLite amb mode de creació si falta.
    pub fn sqlite_database_url(&self) -> String {
        normalize_sqlite_url(&self.database_url)
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_sqlite_url;

    #[test]
    fn afegeix_mode_rwc_a_fitxer_sqlite() {
        assert_eq!(
            normalize_sqlite_url("sqlite://chillgroup.db"),
            "sqlite://chillgroup.db?mode=rwc"
        );
    }

    #[test]
    fn preserva_mode_rwc_existents_i_memoria() {
        assert_eq!(
            normalize_sqlite_url("sqlite://chillgroup.db?cache=shared"),
            "sqlite://chillgroup.db?cache=shared&mode=rwc"
        );
        assert_eq!(
            normalize_sqlite_url("sqlite://chillgroup.db?mode=rwc"),
            "sqlite://chillgroup.db?mode=rwc"
        );
        assert_eq!(normalize_sqlite_url("sqlite::memory:"), "sqlite::memory:");
    }
}