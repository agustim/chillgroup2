//! Entry point del servidor ChillGroup v2.

#![allow(dead_code)]

mod config;
mod db;
mod routes;
mod services;
mod repositories;
mod models;
mod crypto;
mod middleware;
mod error;

use axum::{
    Router,
    middleware::from_fn,
};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use tracing::info;

use config::Config;
use middleware::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Inicialitzar tracing amb nivells de log
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::level_filters::LevelFilter::INFO.into()))
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("No s'ha pogut configurar el subscriber de tracing");

    info!("🚀 Inicialitzant ChillGroup v2...");

    // Carregar configuració
    let (config, env_path) = Config::from_env().map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
    info!("✅ Configuració carregada correctament (des de: {})", env_path.display());

    // Connectar base de dades amb comprovació
    let db_pool = db::connect_db(&config)
        .await
        .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
    info!("✅ Base de dades connectada correctament");

    // Crear estat compartit
    let state = AppState {
        db: db_pool.clone(),
        config,
    };

    // Crear router amb tots els routers dels fitxers de routes
    // Rutes sense auth (health + auth)
    let public_app = Router::new()
        .merge(routes::health::router(state.clone()))
        .merge(routes::auth::router(state.clone()))
        .layer(CorsLayer::permissive());

    // Rutes amb auth - mergejant els routers dels fitxers de routes
    let server_routes = routes::servers::router(state.clone());
    let channel_routes = routes::channels::router(state.clone());
    let message_routes = routes::messages::router(state.clone());
    let livekit_routes = routes::livekit::router(state.clone());
    let user_routes = routes::user::router(state.clone());

    let protected_app = server_routes
        .merge(channel_routes)
        .merge(message_routes)
        .merge(livekit_routes)
        .merge(user_routes)
        .layer(from_fn(middleware::extract_claims));

    // Combinar rutes públiques i protegides
    let app = public_app.merge(protected_app).layer(CorsLayer::permissive());

    // Iniciar servidor
    let addr = format!("{}:{}", state.config.server_host, state.config.server_port);
    info!("📡 Servidor escoltant a {}", addr);
    info!("🔑 LiveKit host: {}", state.config.livekit_host);
    info!("🔒 JWT expiration: {} days", state.config.jwt_expiration_days);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    info!("🛑 Servidor aturat");
    Ok(())
}
