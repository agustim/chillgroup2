//! Tests d'integració del flux E2EE: crear canal → guardar clau → recuperar clau → verificar.

use chillgroup_server::{
    config::{Config, LogLevel},
    db::{connect_db, ChannelKeyBundleWriteResult},
    middleware::{auth::UserPresenceState, AppState},
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
        server_master_key: [6u8; 32],
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
    }
}

#[tokio::test]
async fn crypto_flow_symmetric_key_version_created_and_retrieved() {
    let state = make_state().await;

    let owner_id = state.db.create_user("crypto_owner", "hash").await.expect("owner");
    let server_id = Uuid::new_v4();
    state.db
        .create_server_with_owner(server_id, "crypto-server", None, owner_id)
        .await
        .expect("create server");

    let channel_id = Uuid::new_v4();
    state.db
        .create_channel(channel_id, server_id, "secret-channel", "text", "symmetric", None, false)
        .await
        .expect("create channel");

    // Crear versió de clau simètrica (simulem un encrypted_key + nonce base64)
    let encrypted_key = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    let nonce = "BBBBBBBBBBBBBBBB";

    let key_version_id = state.db
        .create_channel_key_version(channel_id, 1, encrypted_key, nonce, owner_id)
        .await
        .expect("create key version");

    // Recuperar la versió creada
    let latest = state.db
        .get_latest_channel_key_version(channel_id)
        .await
        .expect("get latest key version")
        .expect("should have a key version");

    assert_eq!(latest.0, key_version_id, "key_version_id ha de coincidir");
    assert_eq!(latest.1, 1, "versió ha de ser 1");
    assert_eq!(latest.2, encrypted_key, "encrypted_key ha de coincidir");
    assert_eq!(latest.3, nonce, "nonce ha de coincidir");
}

#[tokio::test]
async fn crypto_flow_asymmetric_bundle_store_and_retrieve() {
    let state = make_state().await;

    let creator_id = state.db.create_user("asym_creator", "hash").await.expect("creator");
    let recipient_id = state.db.create_user("asym_recipient", "hash").await.expect("recipient");

    let server_id = Uuid::new_v4();
    state.db
        .create_server_with_owner(server_id, "asym-server", None, creator_id)
        .await
        .expect("create server");

    let channel_id = Uuid::new_v4();
    state.db
        .create_channel(channel_id, server_id, "e2ee-channel", "text", "asymmetric", None, false)
        .await
        .expect("create channel");

    // Crear versió de clau
    let key_version_id = state.db
        .create_channel_key_version(channel_id, 1, "enc-key", "nonce-x", creator_id)
        .await
        .expect("create key version");

    // Crear un device per al recipient
    let device_id = state.db
        .upsert_device_for_user(recipient_id, "test-device", None)
        .await
        .expect("create device");

    // Guardar bundle asimètric per al device del recipient
    let encrypted_key_for_device = "CIPHER_KEY_FOR_DEVICE_AAAAAAAAAAAAA==";
    let kem_ciphertext = "KEM_CIPHERTEXT_BBBBBBBBBBBBBBBBB==";

    let result = state.db
        .store_channel_key_bundle_for_device(
            key_version_id,
            device_id,
            encrypted_key_for_device,
            kem_ciphertext,
            None,
            None,
        )
        .await
        .expect("store bundle");

    assert!(
        matches!(result, ChannelKeyBundleWriteResult::Inserted),
        "el bundle s'ha d'inserir"
    );

    // Recuperar el bundle per al device — retorna (key_version_id, version, encrypted_key, kem_ciphertext, sig, signed_by)
    let bundle = state.db
        .get_latest_channel_key_bundle_for_device(channel_id, device_id)
        .await
        .expect("get bundle")
        .expect("bundle should exist");

    assert_eq!(bundle.2, encrypted_key_for_device, "encrypted_key ha de coincidir");
    assert_eq!(bundle.3, kem_ciphertext, "kem_ciphertext ha de coincidir");
}

#[tokio::test]
async fn crypto_flow_bundle_idempotent_same_payload() {
    let state = make_state().await;

    let creator_id = state.db.create_user("idemp_creator", "hash").await.expect("creator");
    let server_id = Uuid::new_v4();
    state.db
        .create_server_with_owner(server_id, "idemp-server", None, creator_id)
        .await
        .expect("server");
    let channel_id = Uuid::new_v4();
    state.db
        .create_channel(channel_id, server_id, "idemp-ch", "text", "asymmetric", None, false)
        .await
        .expect("channel");

    let key_version_id = state.db
        .create_channel_key_version(channel_id, 1, "enc", "nc", creator_id)
        .await
        .expect("key version");

    let device_id = state.db
        .upsert_device_for_user(creator_id, "idemp-dev", None)
        .await
        .expect("device");

    // Primera inserció
    let r1 = state.db
        .store_channel_key_bundle_for_device(key_version_id, device_id, "ENC", "KEM", None, None)
        .await
        .expect("first store");
    assert!(matches!(r1, ChannelKeyBundleWriteResult::Inserted));

    // Mateixa inserció: ha de retornar Unchanged
    let r2 = state.db
        .store_channel_key_bundle_for_device(key_version_id, device_id, "ENC", "KEM", None, None)
        .await
        .expect("second store");
    assert!(matches!(r2, ChannelKeyBundleWriteResult::Unchanged));
}

#[tokio::test]
async fn crypto_flow_bundle_conflict_different_payload() {
    let state = make_state().await;

    let creator_id = state.db.create_user("conflict_creator", "hash").await.expect("creator");
    let server_id = Uuid::new_v4();
    state.db
        .create_server_with_owner(server_id, "conflict-server", None, creator_id)
        .await
        .expect("server");
    let channel_id = Uuid::new_v4();
    state.db
        .create_channel(channel_id, server_id, "conflict-ch", "text", "asymmetric", None, false)
        .await
        .expect("channel");

    let key_version_id = state.db
        .create_channel_key_version(channel_id, 1, "enc", "nc", creator_id)
        .await
        .expect("key version");

    let device_id = state.db
        .upsert_device_for_user(creator_id, "conflict-dev", None)
        .await
        .expect("device");

    state.db
        .store_channel_key_bundle_for_device(key_version_id, device_id, "ENC_V1", "KEM_V1", None, None)
        .await
        .expect("first store");

    // Payload diferent → ha de retornar Conflict
    let r = state.db
        .store_channel_key_bundle_for_device(key_version_id, device_id, "ENC_V2", "KEM_V2", None, None)
        .await
        .expect("second store");
    assert!(matches!(r, ChannelKeyBundleWriteResult::Conflict));
}
