//! Configuració de l'aplicació carregada des de variables d'entorn.

use std::env;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Config {
    pub server_host: String,
    pub server_port: u16,
    pub database_url: String,
    pub ttl_cleanup_interval_minutes: u64,
    pub livekit_host: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    pub jwt_secret: String,
    pub jwt_expiration_days: u32,
}

impl Config {
    /// Carregar configuració des de variables d'entorn i .env.
    pub fn from_env() -> Result<Self, String> {
        dotenvy::dotenv().ok();

        fn get_var(key: &str) -> Result<String, String> {
            env::var(key).map_err(|_| format!("La variable d'entorn {} és obligatòria", key))
        }

        Ok(Self {
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            server_port: env::var("SERVER_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            database_url: get_var("DATABASE_URL")?,
            ttl_cleanup_interval_minutes: env::var("TTL_CLEANUP_INTERVAL_MINUTES")
                .ok()
                .and_then(|i| i.parse().ok())
                .unwrap_or(5),
            livekit_host: get_var("LIVEKIT_HOST")?,
            livekit_api_key: get_var("LIVEKIT_API_KEY")?,
            livekit_api_secret: get_var("LIVEKIT_API_SECRET")?,
            jwt_secret: get_var("JWT_SECRET")?,
            jwt_expiration_days: env::var("JWT_EXPIRATION_DAYS")
                .ok()
                .and_then(|d| d.parse().ok())
                .unwrap_or(7),
        })
    }

    /// Comprovar si és SQLite.
    pub fn is_sqlite(&self) -> bool {
        self.database_url.starts_with("sqlite")
    }
}