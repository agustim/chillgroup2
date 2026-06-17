//! Tests d'integració del flux de canals: crear → convidar → accedir.

use chillgroup_server::{
    config::{Config, LogLevel},
    db::connect_db,
    middleware::{auth::UserPresenceState, AppState, AuthClaims},
};
use std::{collections::{HashMap, HashSet}, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

async fn make_state() -> AppState {
    let config = Config {
        server_host: "127.0.0.1".to_string(),
        server_port: 8080,
        database_url: "sqlite::memory:".to_string(),
        open_register: true,
        admin_user: None,
        admin_password: None,
        ttl_cleanup_interval_minutes: 5,
        livekit_host: "http://localhost:7880".to_string(),
        livekit_api_key: "test-key".to_string(),
        livekit_api_secret: "test-secret".to_string(),
        jwt_secret: "test-secret".to_string(),
        jwt_expiration_days: 7,
        backend_debug: LogLevel::Info,
        allowed_origins: Vec::new(),
        server_master_key: [5u8; 32],
        static_dir: None,
        max_file_size_bytes: 0,
    };
    let db = connect_db(&config).await.expect("sqlite test db");
    let (_layer, io) = socketioxide::SocketIo::new_layer();
    AppState {
        db,
        config,
        io,
        user_presence: Arc::new(RwLock::new(UserPresenceState {
            online_sockets: HashMap::<Uuid, HashSet<String>>::new(),
        })),
        livekit_token_cache: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    }
}

#[tokio::test]
async fn channel_flow_create_and_access() {
    let state = make_state().await;

    // 1. Crear owner i un segon membre
    let owner_id = state.db.create_user("flow_owner", "hash").await.expect("owner");
    let member_id = state.db.create_user("flow_member", "hash").await.expect("member");

    // 2. Crear servidor amb owner
    let server_id = Uuid::new_v4();
    state.db
        .create_server_with_owner(server_id, "test-server", None, owner_id)
        .await
        .expect("create server");

    // 3. Crear canal de text
    let channel_id = Uuid::new_v4();
    state.db
        .create_channel(channel_id, server_id, "general", "text", "symmetric", None, false)
        .await
        .expect("create channel");

    // 4. Verificar que l'owner pot accedir al canal (és membre del servidor)
    let can_access = state.db
        .user_can_access_channel(channel_id, owner_id)
        .await
        .expect("check access");
    assert!(can_access, "owner ha de poder accedir al canal");

    // 5. Afegir member al servidor
    state.db
        .add_server_member(server_id, member_id, "member")
        .await
        .expect("add member");

    // 6. Verificar que el membre ara pot accedir
    let member_can_access = state.db
        .user_can_access_channel(channel_id, member_id)
        .await
        .expect("check member access");
    assert!(member_can_access, "membre del servidor ha de poder accedir al canal");
}

#[tokio::test]
async fn channel_flow_private_channel_visibility() {
    // is_private controla la llista visible, no el bloqueig directe.
    // Un canal privat NO apareix a list_channels_for_server per a membres que no hi estan.
    let state = make_state().await;

    let owner_id = state.db.create_user("priv_owner", "hash").await.expect("owner");
    let member_id = state.db.create_user("priv_member", "hash").await.expect("member");

    let server_id = Uuid::new_v4();
    state.db
        .create_server_with_owner(server_id, "priv-server", None, owner_id)
        .await
        .expect("create server");

    let public_id = Uuid::new_v4();
    state.db
        .create_channel(public_id, server_id, "public", "text", "none", None, false)
        .await
        .expect("public channel");

    let private_id = Uuid::new_v4();
    state.db
        .create_channel(private_id, server_id, "private", "text", "asymmetric", None, true)
        .await
        .expect("private channel");

    state.db.add_server_member(server_id, member_id, "member").await.expect("add member");

    // Member no explícit del canal privat: no apareix a la llista
    let channels = state.db
        .list_channels_for_server(server_id, member_id)
        .await
        .expect("list channels");

    let ids: Vec<Uuid> = channels.iter().map(|c| c.id).collect();
    assert!(ids.contains(&public_id), "canal públic ha d'aparèixer");
    assert!(!ids.contains(&private_id), "canal privat no ha d'aparèixer per a membre no explícit");

    // Afegir el membre explícitament al canal privat
    state.db.add_channel_member(private_id, member_id).await.expect("add to private");

    let channels2 = state.db
        .list_channels_for_server(server_id, member_id)
        .await
        .expect("list channels after add");
    let ids2: Vec<Uuid> = channels2.iter().map(|c| c.id).collect();
    assert!(ids2.contains(&private_id), "canal privat ha d'aparèixer ara que és membre explícit");
}

#[tokio::test]
async fn channel_flow_member_permissions() {
    let state = make_state().await;

    let owner_id = state.db.create_user("perm_owner", "hash").await.expect("owner");
    let member_id = state.db.create_user("perm_member", "hash").await.expect("member");

    let server_id = Uuid::new_v4();
    state.db
        .create_server_with_owner(server_id, "perm-server", None, owner_id)
        .await
        .expect("create server");

    let channel_id = Uuid::new_v4();
    state.db
        .create_channel(channel_id, server_id, "perm-channel", "text", "none", None, false)
        .await
        .expect("create channel");

    state.db.add_server_member(server_id, member_id, "member").await.expect("add member");

    // Nivell per defecte: lectura+escriptura (2)
    let level = state.db
        .get_channel_permission_level(channel_id, member_id)
        .await
        .expect("get permission level")
        .unwrap_or(0);

    assert!(level >= 2, "membre ha de tenir almenys permisos de lectura+escriptura per defecte");
}

#[tokio::test]
async fn channel_flow_message_send_and_list() {
    let state = make_state().await;

    let owner_id = state.db.create_user("msg_owner", "hash").await.expect("owner");

    let server_id = Uuid::new_v4();
    state.db
        .create_server_with_owner(server_id, "msg-server", None, owner_id)
        .await
        .expect("create server");

    let channel_id = Uuid::new_v4();
    state.db
        .create_channel(channel_id, server_id, "msg-channel", "text", "none", None, false)
        .await
        .expect("create channel");

    let message_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();
    state.db
        .create_message(
            message_id,
            channel_id,
            owner_id,
            "msg_owner",
            device_id,
            "payload-enc",
            "iv",
            None,
            None,
            chrono::Utc::now(),
            None,
        )
        .await
        .expect("create message");

    let messages = state.db
        .list_messages(channel_id, 50, None, None, None)
        .await
        .expect("list messages");

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, message_id);
}
