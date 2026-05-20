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
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::db::DatabasePool;

use config::Config;
use middleware::{AppState, AuthClaims};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VoicePresenceUser {
    user_id: Uuid,
    username: String,
    joined_at: String,
    is_deafened: bool,
    is_suppressed: bool,
    is_speaking: bool,
}

#[derive(Debug, Default)]
struct VoicePresenceState {
    channel_users: HashMap<Uuid, Vec<VoicePresenceUser>>,
    user_channel: HashMap<Uuid, Uuid>,
}

async fn emit_voice_presence_update(
    db: &DatabasePool,
    io: &SocketIo,
    channel_id: Uuid,
    users: Vec<VoicePresenceUser>,
) {
    let Ok(Some(channel)) = db.get_channel(channel_id).await else {
        return;
    };

    let Ok(member_ids) = db.list_server_member_ids(channel.server_id).await else {
        return;
    };

    let payload = serde_json::json!({
        "channelId": channel_id,
        "users": users,
    });

    for member_id in member_ids {
        let room = format!("user:{}", member_id);
        if let Err(e) = io.to(room).emit("voice-presence-updated", &payload).await {
            tracing::warn!("Error enviant voice-presence-updated: {:?}", e);
        }
    }
}

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
    let io_for_ns = io.clone();
    let jwt_secret = config.jwt_secret.clone();
    let socket_db = db_pool.clone();
    let voice_presence = Arc::new(RwLock::new(VoicePresenceState::default()));

    io.ns("/", move |socket: SocketRef, Data(auth): Data<serde_json::Value>| {
        let secret = jwt_secret.clone();
        let db = socket_db.clone();
        let io = io_for_ns.clone();
        let voice_presence = voice_presence.clone();
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

            let db_for_presence = db.clone();
            let io_for_presence = io.clone();
            let presence_for_join = voice_presence.clone();
            let user_id_for_voice = claims.user_id;
            let username_for_voice = claims.username.clone();
            socket.on("join-voice-channel", move |Data(data): Data<serde_json::Value>| {
                let db = db_for_presence.clone();
                let io = io_for_presence.clone();
                let presence = presence_for_join.clone();
                let username = username_for_voice.clone();
                async move {
                    let Some(channel_id_str) = data.get("channelId").and_then(|v| v.as_str()) else {
                        return;
                    };
                    let Ok(channel_id) = Uuid::parse_str(channel_id_str) else {
                        return;
                    };

                    let mut affected_channels = Vec::new();
                    {
                        let mut state = presence.write().await;

                        if let Some(prev_channel) = state.user_channel.get(&user_id_for_voice).copied() {
                            if prev_channel != channel_id {
                                if let Some(users) = state.channel_users.get_mut(&prev_channel) {
                                    users.retain(|u| u.user_id != user_id_for_voice);
                                }
                                affected_channels.push(prev_channel);
                            }
                        }

                        state.user_channel.insert(user_id_for_voice, channel_id);

                        let users = state.channel_users.entry(channel_id).or_default();
                        if !users.iter().any(|u| u.user_id == user_id_for_voice) {
                            users.push(VoicePresenceUser {
                                user_id: user_id_for_voice,
                                username,
                                joined_at: chrono::Utc::now().to_rfc3339(),
                                is_deafened: false,
                                is_suppressed: false,
                                is_speaking: false,
                            });
                        }
                        affected_channels.push(channel_id);
                    }

                    for affected_channel in affected_channels {
                        let users = {
                            let state = presence.read().await;
                            state
                                .channel_users
                                .get(&affected_channel)
                                .cloned()
                                .unwrap_or_default()
                        };
                        emit_voice_presence_update(&db, &io, affected_channel, users).await;
                    }
                }
            });

            let db_for_leave = db.clone();
            let io_for_leave = io.clone();
            let presence_for_leave = voice_presence.clone();
            socket.on("leave-voice-channel", move |Data(data): Data<serde_json::Value>| {
                let db = db_for_leave.clone();
                let io = io_for_leave.clone();
                let presence = presence_for_leave.clone();
                async move {
                    let requested_channel = data
                        .get("channelId")
                        .and_then(|v| v.as_str())
                        .and_then(|id| Uuid::parse_str(id).ok());

                    let mut affected_channel = None;
                    {
                        let mut state = presence.write().await;
                        let current_channel = state.user_channel.get(&user_id_for_voice).copied();
                        let target_channel = requested_channel.or(current_channel);

                        if let Some(channel_id) = target_channel {
                            if let Some(users) = state.channel_users.get_mut(&channel_id) {
                                users.retain(|u| u.user_id != user_id_for_voice);
                            }
                            state.user_channel.remove(&user_id_for_voice);
                            affected_channel = Some(channel_id);
                        }
                    }

                    if let Some(channel_id) = affected_channel {
                        let users = {
                            let state = presence.read().await;
                            state
                                .channel_users
                                .get(&channel_id)
                                .cloned()
                                .unwrap_or_default()
                        };
                        emit_voice_presence_update(&db, &io, channel_id, users).await;
                    }
                }
            });

            let db_for_snapshot = db.clone();
            let presence_for_snapshot = voice_presence.clone();
            socket.on("get-voice-presence", move |socket: SocketRef, Data(data): Data<serde_json::Value>| {
                let db = db_for_snapshot.clone();
                let presence = presence_for_snapshot.clone();
                async move {
                    let Some(server_id_str) = data.get("serverId").and_then(|v| v.as_str()) else {
                        return;
                    };
                    let Ok(server_id) = Uuid::parse_str(server_id_str) else {
                        return;
                    };

                    let Ok(role) = db.is_server_member(server_id, claims.user_id).await else {
                        return;
                    };
                    if role.is_none() {
                        return;
                    }

                    let Ok(channels) = db.list_channels_for_server(server_id, claims.user_id).await else {
                        return;
                    };

                    let state = presence.read().await;
                    let channel_entries: Vec<serde_json::Value> = channels
                        .into_iter()
                        .filter(|c| matches!(c.channel_type, crate::models::channel::ChannelType::Voice))
                        .map(|c| {
                            let users = state.channel_users.get(&c.id).cloned().unwrap_or_default();
                            serde_json::json!({
                                "channelId": c.id,
                                "users": users,
                            })
                        })
                        .collect();

                    let payload = serde_json::json!({
                        "serverId": server_id,
                        "channels": channel_entries,
                    });

                    if let Err(e) = socket.emit("voice-presence-snapshot", &payload) {
                        tracing::warn!("Error enviant voice-presence-snapshot: {:?}", e);
                    }
                }
            });

            let db_for_disconnect = db.clone();
            let io_for_disconnect = io.clone();
            let presence_for_disconnect = voice_presence.clone();
            socket.on_disconnect(move || {
                let db = db_for_disconnect.clone();
                let io = io_for_disconnect.clone();
                let presence = presence_for_disconnect.clone();
                async move {
                    let mut affected_channel = None;
                    {
                        let mut state = presence.write().await;
                        if let Some(channel_id) = state.user_channel.remove(&user_id_for_voice) {
                            if let Some(users) = state.channel_users.get_mut(&channel_id) {
                                users.retain(|u| u.user_id != user_id_for_voice);
                            }
                            affected_channel = Some(channel_id);
                        }
                    }

                    if let Some(channel_id) = affected_channel {
                        let users = {
                            let state = presence.read().await;
                            state.channel_users.get(&channel_id).cloned().unwrap_or_default()
                        };
                        emit_voice_presence_update(&db, &io, channel_id, users).await;
                    }
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
