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
        livekit_token_cache: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
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
async fn crypto_flow_symmetric_key_version_id_present_in_db_response() {
    // Verifica que get_latest_channel_key_version retorna el UUID (key_version_id)
    // necessari per incloure'l a la resposta del servidor (fix: keyVersionId absent en simètric).
    let state = make_state().await;

    let owner_id = state.db.create_user("kvid_owner", "hash").await.expect("owner");
    let server_id = Uuid::new_v4();
    state.db
        .create_server_with_owner(server_id, "kvid-server", None, owner_id)
        .await
        .expect("server");
    let channel_id = Uuid::new_v4();
    state.db
        .create_channel(channel_id, server_id, "kvid-ch", "text", "symmetric", None, false)
        .await
        .expect("channel");

    let expected_id = state.db
        .create_channel_key_version(channel_id, 1, "enc_placeholder", "nonce_placeholder", owner_id)
        .await
        .expect("create key version");

    let latest = state.db
        .get_latest_channel_key_version(channel_id)
        .await
        .expect("db query")
        .expect("exists");

    assert_eq!(latest.0, expected_id, "key_version_id (UUID) ha de ser el retornat a la posició 0 de la tupla");
    assert_eq!(latest.1, 1, "version ha de ser 1");

    // Rotar clau i verificar que get_latest retorna la versió nova
    let expected_v2 = state.db
        .create_channel_key_version(channel_id, 2, "enc_v2", "nonce_v2", owner_id)
        .await
        .expect("create key version v2");

    let latest_v2 = state.db
        .get_latest_channel_key_version(channel_id)
        .await
        .expect("db query v2")
        .expect("exists v2");

    assert_eq!(latest_v2.0, expected_v2, "key_version_id v2 ha de coincidir");
    assert_eq!(latest_v2.1, 2, "version ha de ser 2");
    assert_ne!(latest_v2.0, expected_id, "ha de ser un UUID diferent al de la v1");
}

#[tokio::test]
async fn crypto_flow_symmetric_kem_aes_roundtrip() {
    // Verifica el flux complet que fa el servidor (encapsular + AES-GCM wrap)
    // i el client (decapsular + AES-GCM unwrap) per a canals simètrics.
    // Replica exactament la lògica de wrap_channel_key_for_device (servidor)
    // i unwrapKeyWithKem (frontend).
    use aes_gcm::{Aes256Gcm, Nonce, aead::{Aead, KeyInit}};
    use ml_kem::{Kem, MlKem1024, kem::{Decapsulate, Encapsulate}};
    use rand::RngCore;

    // Channel key real (32 bytes AES-256, com genera el servidor)
    let mut channel_key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut channel_key);

    // Generar keypair ML-KEM-1024 (simula el dispositiu client)
    let (dk, ek) = MlKem1024::generate_keypair();

    // ── Costat servidor: wrap_channel_key_for_device ──────────────────────────
    let (kem_ciphertext, shared_secret_send) = ek.encapsulate();

    let mut wrapping_key = [0u8; 32];
    wrapping_key.copy_from_slice(shared_secret_send.as_slice());

    let cipher = Aes256Gcm::new_from_slice(&wrapping_key).expect("cipher");
    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let encrypted = cipher.encrypt(nonce, channel_key.as_ref()).expect("encrypt");

    // Format: nonce (12 bytes) || ciphertext, igual que wrap_channel_key_for_device
    let mut wrapped = Vec::with_capacity(12 + encrypted.len());
    wrapped.extend_from_slice(&nonce_bytes);
    wrapped.extend_from_slice(&encrypted);

    // ── Costat client: unwrapKeyWithKem ───────────────────────────────────────
    let shared_secret_recv = dk.decapsulate(&kem_ciphertext);

    assert_eq!(
        shared_secret_send.as_slice(),
        shared_secret_recv.as_slice(),
        "shared secrets han de coincidir entre encapsular i decapsular"
    );

    let mut unwrapping_key = [0u8; 32];
    unwrapping_key.copy_from_slice(shared_secret_recv.as_slice());

    // Client llegeix: iv = slice(0,12), ciphertext = slice(12..)
    let iv = Nonce::from_slice(&wrapped[..12]);
    let ciphertext = &wrapped[12..];
    let cipher2 = Aes256Gcm::new_from_slice(&unwrapping_key).expect("cipher2");
    let decrypted = cipher2.decrypt(iv, ciphertext).expect("decrypt");

    assert_eq!(
        decrypted.as_slice(),
        channel_key.as_ref(),
        "channel_key recuperada ha de ser idèntica a l'original"
    );
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
