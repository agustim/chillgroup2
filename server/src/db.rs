//! Connexió a la base de dades.

use sqlx::{Pool, Postgres};
use crate::config::Config;

/// Connexió a la base de dades PostgreSQL.
pub async fn connect_db(config: &Config) -> Result<Pool<Postgres>, sqlx::Error> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(20)
        .connect_lazy(&config.database_url)?;
    Ok(pool)
}