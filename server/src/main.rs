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
    middleware::{from_fn, from_fn_with_state},
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use socketioxide::{SocketIo, extract::{Data, SocketRef}};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use tracing::info;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

use config::Config;
use crate::db::DatabasePool;
use crate::crypto::hash;
use middleware::{AppState, auth::UserPresenceState};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct VoicePresenceUser {
    #[serde(skip_serializing)]
    socket_id: String,
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
    socket_channel: HashMap<String, Uuid>,
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

async fn emit_friend_presence_update(
    db: &DatabasePool,
    io: &SocketIo,
    user_id: Uuid,
    username: &str,
    status: &str,
) {
    let Ok(owner_ids) = db.list_friend_owner_ids_for_user(user_id).await else {
        return;
    };

    let payload = serde_json::json!({
        "userId": user_id,
        "username": username,
        "status": status,
    });

    for owner_id in owner_ids {
        let room = format!("user:{}", owner_id);
        if let Err(e) = io.to(room).emit("friend-presence-updated", &payload).await {
            tracing::warn!("Error enviant friend-presence-updated: {:?}", e);
        }
    }
}

async fn emit_server_member_presence_update(
    db: &DatabasePool,
    io: &SocketIo,
    user_id: Uuid,
    username: &str,
    status: &str,
) {
    let Ok(server_ids) = db.list_server_ids_for_user(user_id).await else {
        return;
    };

    for server_id in server_ids {
        let payload = serde_json::json!({
            "serverId": server_id,
            "userId": user_id,
            "username": username,
            "status": status,
        });
        let room = format!("server:{}", server_id);
        if let Err(e) = io.to(room).emit("server-member-presence-updated", &payload).await {
            tracing::warn!("Error enviant server-member-presence-updated: {:?}", e);
        }
    }
}

async fn register_user_socket(
    db: &DatabasePool,
    io: &SocketIo,
    presence: &Arc<RwLock<UserPresenceState>>,
    user_id: Uuid,
    username: &str,
    socket_id: &str,
) {
    let should_broadcast = {
        let mut state = presence.write().await;
        let sockets = state.online_sockets.entry(user_id).or_default();
        let was_empty = sockets.is_empty();
        sockets.insert(socket_id.to_string());
        was_empty && !sockets.is_empty()
    };

    if should_broadcast {
        emit_friend_presence_update(db, io, user_id, username, "online").await;
        emit_server_member_presence_update(db, io, user_id, username, "online").await;
    }
}

async fn unregister_user_socket(
    db: &DatabasePool,
    io: &SocketIo,
    presence: &Arc<RwLock<UserPresenceState>>,
    user_id: Uuid,
    username: &str,
    socket_id: &str,
) {
    let should_broadcast = {
        let mut state = presence.write().await;
        if let Some(sockets) = state.online_sockets.get_mut(&user_id) {
            sockets.remove(socket_id);
            if sockets.is_empty() {
                state.online_sockets.remove(&user_id);
                true
            } else {
                false
            }
        } else {
            false
        }
    };

    if should_broadcast {
        emit_friend_presence_update(db, io, user_id, username, "offline").await;
        emit_server_member_presence_update(db, io, user_id, username, "offline").await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Carregar configuració abans d'inicialitzar tracing per poder aplicar BACKEND_DEBUG.
    let (config, env_path) = Config::from_env().map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;

    // Inicialitzar tracing amb nivells de log
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::new(config.backend_debug.as_tracing_filter()))
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("No s'ha pogut configurar el subscriber de tracing");

    info!("🚀 Inicialitzant ChillGroup v2...");

    info!("✅ Configuració carregada correctament (des de: {})", env_path.display());

    // Connectar base de dades amb comprovació
    let db_pool = db::connect_db(&config)
        .await
        .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
    info!("✅ Base de dades connectada correctament");

    if !config.open_register {
        let admin_username = config
            .admin_user
            .as_deref()
            .ok_or_else(|| "ADMIN_USER és obligatori quan OPEN_REGISTER=false".to_string())?;
        let admin_password = config
            .admin_password
            .as_deref()
            .ok_or_else(|| "ADMIN_PASSWORD és obligatori quan OPEN_REGISTER=false".to_string())?;

        match db_pool.find_user_auth_by_username(admin_username).await {
            Ok(Some((_id, _name, _hash, is_admin))) => {
                if !is_admin {
                    db_pool
                        .update_user_role_by_username(admin_username, "admin")
                        .await
                        .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
                    info!("✅ Usuari existent promogut a admin: {}", admin_username);
                }
            }
            Ok(None) => {
                let password_hash = hash::hash_password(admin_password)
                    .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e.to_string()))?;
                db_pool
                    .create_user_with_role(admin_username, &password_hash, "admin")
                    .await
                    .map_err(|e| Box::<dyn std::error::Error + Send + Sync>::from(e))?;
                info!("✅ Admin inicial creat: {}", admin_username);
            }
            Err(e) => return Err(Box::<dyn std::error::Error + Send + Sync>::from(e)),
        }
    }

    // Inicialitzar Socket.IO i la purga periòdica de TTL
    let (socket_layer, io) = SocketIo::new_layer();
    services::ttl_cleanup::spawn_ttl_cleanup(db_pool.clone(), io.clone(), config.ttl_cleanup_interval_minutes);
    let io_for_ns = io.clone();
    let jwt_secret = config.jwt_secret.clone();
    let socket_db = db_pool.clone();
    let voice_presence = Arc::new(RwLock::new(VoicePresenceState::default()));
    let user_presence = Arc::new(RwLock::new(UserPresenceState::default()));
    let user_presence_for_ns = user_presence.clone();

    io.ns("/", move |socket: SocketRef, Data(auth): Data<serde_json::Value>| {
        let secret = jwt_secret.clone();
        let db = socket_db.clone();
        let io = io_for_ns.clone();
        let voice_presence = voice_presence.clone();
        let user_presence = user_presence_for_ns.clone();
        async move {
            let token = auth.get("token").and_then(|t| t.as_str()).unwrap_or("");
            let mut validation = Validation::new(Algorithm::HS256);
            validation.leeway = 5;

            let decoded = decode::<crate::middleware::AuthClaims>(
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
            if let Ok(server_ids) = db.list_server_ids_for_user(claims.user_id).await {
                for server_id in server_ids {
                    socket.join(format!("server:{}", server_id));
                }
            }
            register_user_socket(
                &db,
                &io,
                &user_presence,
                claims.user_id,
                &claims.username,
                &socket.id.to_string(),
            ).await;

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

            let db_for_server_room = db.clone();
            let user_id_for_server_room = claims.user_id;
            socket.on("join-server-presence", move |socket: SocketRef, Data(data): Data<serde_json::Value>| {
                let db = db_for_server_room.clone();
                async move {
                    let Some(server_id_str) = data.get("serverId").and_then(|v| v.as_str()) else {
                        return;
                    };
                    let Ok(server_id) = Uuid::parse_str(server_id_str) else {
                        return;
                    };

                    let Ok(role) = db.is_server_member(server_id, user_id_for_server_room).await else {
                        return;
                    };
                    if role.is_none() {
                        return;
                    }

                    socket.join(format!("server:{}", server_id));
                }
            });

            let db_for_presence = db.clone();
            let io_for_presence = io.clone();
            let presence_for_join = voice_presence.clone();
            let user_id_for_voice = claims.user_id;
            let username_for_voice = claims.username.clone();
            socket.on("join-voice-channel", move |socket: SocketRef, Data(data): Data<serde_json::Value>| {
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

                    let socket_id = socket.id.to_string();
                    let mut affected_channels = Vec::new();
                    {
                        let mut state = presence.write().await;

                        if let Some(prev_channel) = state.socket_channel.get(&socket_id).copied() {
                            if prev_channel != channel_id {
                                if let Some(users) = state.channel_users.get_mut(&prev_channel) {
                                    users.retain(|u| u.socket_id != socket_id);
                                }
                                affected_channels.push(prev_channel);
                            }
                        }

                        state.socket_channel.insert(socket_id.clone(), channel_id);

                        let users = state.channel_users.entry(channel_id).or_default();
                        if !users.iter().any(|u| u.socket_id == socket_id) {
                            users.push(VoicePresenceUser {
                                socket_id,
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
            socket.on("leave-voice-channel", move |socket: SocketRef, Data(data): Data<serde_json::Value>| {
                let db = db_for_leave.clone();
                let io = io_for_leave.clone();
                let presence = presence_for_leave.clone();
                async move {
                    let socket_id = socket.id.to_string();
                    let requested_channel = data
                        .get("channelId")
                        .and_then(|v| v.as_str())
                        .and_then(|id| Uuid::parse_str(id).ok());

                    let mut affected_channel = None;
                    {
                        let mut state = presence.write().await;
                        let current_channel = state.socket_channel.get(&socket_id).copied();
                        let target_channel = requested_channel.or(current_channel);

                        if let Some(channel_id) = target_channel {
                            if let Some(users) = state.channel_users.get_mut(&channel_id) {
                                users.retain(|u| u.socket_id != socket_id);
                            }
                            state.socket_channel.remove(&socket_id);
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

            let db_for_state = db.clone();
            let io_for_state = io.clone();
            let presence_for_state = voice_presence.clone();
            socket.on("voice-state-updated", move |socket: SocketRef, Data(data): Data<serde_json::Value>| {
                let db = db_for_state.clone();
                let io = io_for_state.clone();
                let presence = presence_for_state.clone();
                async move {
                    let socket_id = socket.id.to_string();
                    let requested_channel = data
                        .get("channelId")
                        .and_then(|v| v.as_str())
                        .and_then(|id| Uuid::parse_str(id).ok());

                    let is_suppressed = data
                        .get("isSuppressed")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let is_deafened = data
                        .get("isDeafened")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let is_speaking = data
                        .get("isSpeaking")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    let mut affected_channel = None;
                    {
                        let mut state = presence.write().await;
                        let current_channel = state.socket_channel.get(&socket_id).copied();
                        let target_channel = requested_channel.or(current_channel);

                        if let Some(channel_id) = target_channel {
                            if let Some(users) = state.channel_users.get_mut(&channel_id) {
                                if let Some(user) = users.iter_mut().find(|u| u.socket_id == socket_id) {
                                    user.is_suppressed = is_suppressed;
                                    user.is_deafened = is_deafened;
                                    user.is_speaking = is_speaking;
                                    affected_channel = Some(channel_id);
                                }
                            }
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

            let db_for_member_presence_snapshot = db.clone();
            let user_presence_for_member_snapshot = user_presence.clone();
            let user_id_for_member_snapshot = claims.user_id;
            socket.on("get-server-member-presence", move |socket: SocketRef, Data(data): Data<serde_json::Value>| {
                let db = db_for_member_presence_snapshot.clone();
                let user_presence = user_presence_for_member_snapshot.clone();
                async move {
                    let Some(server_id_str) = data.get("serverId").and_then(|v| v.as_str()) else {
                        return;
                    };
                    let Ok(server_id) = Uuid::parse_str(server_id_str) else {
                        return;
                    };

                    let Ok(role) = db.is_server_member(server_id, user_id_for_member_snapshot).await else {
                        return;
                    };
                    if role.is_none() {
                        return;
                    }

                    let Ok(Some(server_info)) = db.get_server_full_info(server_id, user_id_for_member_snapshot).await else {
                        return;
                    };

                    let presence = user_presence.read().await;
                    let members: Vec<serde_json::Value> = server_info
                        .members
                        .into_iter()
                        .map(|member| {
                            let status = if presence.online_sockets.contains_key(&member.user_id) {
                                "online"
                            } else {
                                "offline"
                            };
                            serde_json::json!({
                                "userId": member.user_id,
                                "username": member.username,
                                "status": status,
                            })
                        })
                        .collect();

                    let payload = serde_json::json!({
                        "serverId": server_id,
                        "members": members,
                    });

                    if let Err(e) = socket.emit("server-member-presence-snapshot", &payload) {
                        tracing::warn!("Error enviant server-member-presence-snapshot: {:?}", e);
                    }
                }
            });

            let db_for_disconnect = db.clone();
            let io_for_disconnect = io.clone();
            let presence_for_disconnect = voice_presence.clone();
            let user_presence_for_disconnect = user_presence.clone();
            let username_for_disconnect = claims.username.clone();
            let user_id_for_disconnect = claims.user_id;
            socket.on_disconnect(move |socket: SocketRef| {
                let db = db_for_disconnect.clone();
                let io = io_for_disconnect.clone();
                let presence = presence_for_disconnect.clone();
                let user_presence = user_presence_for_disconnect.clone();
                let username = username_for_disconnect.clone();
                async move {
                    let socket_id = socket.id.to_string();
                    let mut affected_channel = None;
                    {
                        let mut state = presence.write().await;
                        if let Some(channel_id) = state.socket_channel.remove(&socket_id) {
                            if let Some(users) = state.channel_users.get_mut(&channel_id) {
                                users.retain(|u| u.socket_id != socket_id);
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

                    unregister_user_socket(
                        &db,
                        &io,
                        &user_presence,
                        user_id_for_disconnect,
                        &username,
                        &socket_id,
                    ).await;
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
                    let Ok(channel_id) = Uuid::parse_str(channel_id_str) else {
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
        user_presence: user_presence.clone(),
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
    let friends_routes = routes::friends::router(state.clone());
    let admin_routes = routes::admin::router(state.clone());
    let plans_routes = routes::plans::router(state.clone());
    let invitation_routes = routes::auth::protected_router(state.clone());

    let protected_app = server_routes
        .merge(channel_routes)
        .merge(message_routes)
        .merge(livekit_routes)
        .merge(friends_routes)
        .merge(user_routes)
        .merge(admin_routes)
        .merge(plans_routes)
        .merge(invitation_routes)
        .layer(from_fn(middleware::extract_claims))
        .layer(from_fn_with_state(state.clone(), middleware::insert_state));

    // Combinar rutes públiques i protegides
    let mut app = public_app.merge(protected_app).layer(CorsLayer::permissive());

    // Servir fitxers estàtics del frontend si STATIC_DIR existeix
    let static_dir = state.config.static_dir
        .clone()
        .unwrap_or_else(|| "./static".to_string());
    let static_path = std::path::Path::new(&static_dir);
    if static_path.exists() {
        info!("📦 Servint frontend estàtic des de: {}", static_dir);
        let index = static_path.join("index.html");
        let serve_dir = ServeDir::new(static_path)
            .not_found_service(ServeFile::new(index));
        app = app.fallback_service(serve_dir);
    } else {
        info!("ℹ️  Directori estàtic no trobat ({}), mode API only", static_dir);
    }

    let app = app.layer(socket_layer);

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
