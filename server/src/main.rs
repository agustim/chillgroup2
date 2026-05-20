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
use socketioxide::{SocketIo, extract::{Data, SocketRef}};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

use config::Config;
use middleware::{AppState, AuthClaims};

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

    // Inicialitzar Socket.IO
    let (socket_layer, io) = SocketIo::new_layer();
    let jwt_secret = config.jwt_secret.clone();
    let socket_db = db_pool.clone();

    io.ns("/", move |socket: SocketRef, Data(auth): Data<serde_json::Value>| {
        let secret = jwt_secret.clone();
        let db = socket_db.clone();
        async move {
            // Verificar JWT de l'auth del socket
            let token = auth.get("token").and_then(|t| t.as_str()).unwrap_or("");
            let mut validation = Validation::new(Algorithm::HS256);
            validation.leeway = 5;

            let decoded = decode::<AuthClaims>(
                token,
                &DecodingKey::from_secret(secret.as_bytes()),
                &validation,
            );

            let claims = if let Ok(decoded) = decoded {
                decoded.claims
            } else {
                info!("Socket connectat amb token invàlid, desconnectant");
                let _ = socket.disconnect();
                return;
            };

            info!("Socket autenticat correctament: {}", socket.id);
            socket.join(format!("user:{}", claims.user_id));

            // Entrar a la room personal de l'usuari (per notificacions futures)
            socket.on("join-channel", |socket: SocketRef, Data(data): Data<serde_json::Value>| async move {
                if let Some(channel_id) = data.get("channelId").and_then(|v| v.as_str()) {
                    socket.join(format!("channel:{}", channel_id));
                    info!("Socket {} s'ha unit a channel:{}", socket.id, channel_id);
                }
            });

            socket.on("leave-channel", |socket: SocketRef, Data(data): Data<serde_json::Value>| async move {
                if let Some(channel_id) = data.get("channelId").and_then(|v| v.as_str()) {
                    socket.leave(format!("channel:{}", channel_id));
                    info!("Socket {} ha sortit de channel:{}", socket.id, channel_id);
                }
            });

            let db_for_read = db.clone();
            let user_id_for_read = claims.user_id;
            socket.on("channel-read", move |socket: SocketRef, Data(data): Data<serde_json::Value>| {
                let db = db_for_read.clone();
                async move {
                    let Some(channel_id_str) = data.get("channelId").and_then(|v| v.as_str()) else {
                        return;
                    };
                    let Ok(channel_id) = uuid::Uuid::parse_str(channel_id_str) else {
                        return;
                    };

                    if let Err(e) = db.mark_channel_read(user_id_for_read, channel_id, None).await {
                        tracing::warn!("Error marcant canal com llegit via socket: {}", e);
                        return;
                    }

                    let payload = serde_json::json!({
                        "channelId": channel_id,
                        "unreadCount": 0,
                    });
                    if let Err(e) = socket.emit("unread-updated", &payload) {
                        tracing::warn!("Error enviant unread-updated (read): {:?}", e);
                    }
                }
            });
        }
    });

    // Crear estat compartit
    let state = AppState {
        db: db_pool.clone(),
        config,
        io: io.clone(),
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

    // Combinar rutes públiques i protegides amb la capa de Socket.IO
    let app = public_app
        .merge(protected_app)
        .layer(socket_layer)
        .layer(CorsLayer::permissive());

    // Iniciar servidor
    let addr = format!("{}:{}", state.config.server_host, state.config.server_port);
    info!("📡 Servidor escoltant a {}", addr);
    info!("🔌 Socket.IO disponible a ws://{}/socket.io/", addr);
    info!("🔑 LiveKit host: {}", state.config.livekit_host);
    info!("🔒 JWT expiration: {} days", state.config.jwt_expiration_days);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    info!("🛑 Servidor aturat");
    Ok(())
}
