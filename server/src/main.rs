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
    routing::{get, put, post},
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
    let config = Config::from_env().map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
    info!("✅ Configuració carregada correctament");

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

    // Crear router amb totes les rutes
    // Routes sense auth (health + auth)
    let public_app = Router::new()
        .merge(routes::health::router(state.clone()))
        .merge(routes::auth::router(state.clone()))
        .layer(CorsLayer::permissive());

    // Routes amb auth - totes les rutes juntes amb middleware d'autenticació
    let protected_app = Router::new()
        .route("/api/servers", get(routes::servers::list_servers).post(routes::servers::create_server))
        .route("/api/servers/{server_id}", get(routes::servers::get_server))
        .route("/api/servers/{server_id}/channels", get(routes::channels::list_channels).post(routes::channels::create_channel))
        .route("/api/servers/{server_id}/members", get(routes::servers::list_server_members).post(routes::servers::invite_server_member))
        .route("/api/servers/{server_id}/members/{user_id}/role", put(routes::servers::update_member_role))
        .route("/api/channels/{channel_id}/keys", get(routes::channels::get_channel_keys))
        .route("/api/channels/{channel_id}/invite", post(routes::channels::invite_to_channel))
        .route("/api/channels/{channel_id}", put(routes::channels::update_channel))
        .route("/api/channels/{channel_id}/messages", get(routes::messages::list_messages).post(routes::messages::send_message))
        .route("/api/messages/{message_id}", put(routes::messages::edit_message))
        .route("/api/livekit/token", get(routes::livekit::generate_token))
        .layer(from_fn(middleware::extract_claims))
        .with_state(state.clone());

    // Rutes d'usuari (amb auth)
    let user_app = routes::user::router(state.clone());
    let protected_app = protected_app.merge(user_app);

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