//! Entry point del servidor ChillGroup v2.

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
    routing::{get, post, put, delete},
    middleware::from_fn,
};
use tower_http::cors::CorsLayer;
use tracing_subscriber::{EnvFilter, FmtSubscriber};

use config::Config;
use middleware::AppState;
use error::AppError;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Inicialitzar tracing
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("No s'ha pogut configurar el subscriber de tracing");

    // Carregar configuració
    let config = Config::from_env().expect("Error carregant configuració");
    tracing::info!("Configuració carregada");

    // Connectar base de dades
    let db = db::connect_db(&config)
        .await
        .expect("Error connectant base de dades");
    tracing::info!("Base de dades connectada");

    // Crear estat compartit
    let state = AppState { db, config };

    // Crear router amb totes les rutes
    // Routes sense auth (health + auth)
    let public_app = Router::new()
        .merge(routes::health::router(state.clone()))
        .merge(routes::auth::router(state.clone()))
        .layer(CorsLayer::permissive());

    // Routes amb auth - totes les rutes juntes
    let protected_app = Router::new()
        .route("/api/servers", get(routes::servers::list_servers).post(routes::servers::create_server))
        .route("/api/servers/{server_id}", get(routes::servers::get_server).delete(routes::servers::delete_server))
        .route("/api/servers/{server_id}/channels", get(routes::channels::list_channels).post(routes::channels::create_channel))
        .route("/api/channels/{channel_id}/keys", get(routes::channels::get_channel_keys))
        .route("/api/channels/{channel_id}/invite", post(routes::channels::invite_to_channel))
        .route("/api/channels/{channel_id}", put(routes::channels::update_channel).delete(routes::channels::delete_channel))
        .route("/api/channels/{channel_id}/messages", get(routes::messages::list_messages).post(routes::messages::send_message))
        .route("/api/messages/{message_id}", put(routes::messages::edit_message).delete(routes::messages::delete_message))
        .route("/api/livekit/token", post(routes::livekit::generate_token))
        .with_state(state.clone());

    // Combinar rutes públiques i protegides
    let app = public_app.merge(protected_app);

    // Iniciar servidor
    let addr = format!("{}:{}", state.config.server_host, state.config.server_port);
    tracing::info!("Servidor escoltant a {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}