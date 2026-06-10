use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use axum::{extract::{Path, State}, Extension, Json};
use chillgroup_server::{
    config::{Config, LogLevel},
    db::connect_db,
    middleware::{auth::UserPresenceState, AppState, AuthClaims},
    routes::attachments::{
        complete_attachment, download_attachment, init_attachment, sign_part,
        CompleteAttachmentRequest, CompleteCrypto, CompletePartItem, InitAttachmentRequest,
        SignPartRequest,
    },
};
use reqwest::Client;
use tokio::sync::RwLock;
use uuid::Uuid;

fn s3_endpoint() -> String {
    std::env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".to_string())
}

async fn rustfs_available(endpoint: &str) -> bool {
    let client = Client::new();
    match client
        .get(endpoint)
        .timeout(Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

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
        server_master_key: [11u8; 32],
        static_dir: None,
        max_file_size_bytes: 100 * 1024 * 1024,
    };

    let db = connect_db(&config)
        .await
        .expect("sqlite test db should initialize");
    let (_layer, io) = socketioxide::SocketIo::new_layer();

    AppState {
        db,
        config,
        io,
        user_presence: std::sync::Arc::new(RwLock::new(UserPresenceState {
            online_sockets: HashMap::<Uuid, HashSet<String>>::new(),
        })),
        livekit_token_cache: std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
    }
}

fn claims_for(user_id: Uuid, username: &str) -> AuthClaims {
    AuthClaims {
        user_id,
        username: username.to_string(),
        device_id: Uuid::new_v4(),
        is_admin: false,
        exp: 0,
        iat: 0,
        jti: Uuid::new_v4().to_string(),
    }
}

async fn setup_server_channel(state: &AppState, owner_name: &str, channel_name: &str) -> (Uuid, Uuid) {
    let owner_id = state
        .db
        .create_user(owner_name, "hash")
        .await
        .expect("create owner");

    let server_id = Uuid::new_v4();
    state
        .db
        .create_server_with_owner(server_id, &format!("srv-{channel_name}"), None, owner_id)
        .await
        .expect("create server");

    let channel_id = Uuid::new_v4();
    state
        .db
        .create_channel(channel_id, server_id, channel_name, "text", "symmetric", None, false)
        .await
        .expect("create channel");

    (owner_id, channel_id)
}

#[tokio::test]
async fn multipart_attachment_flow_with_rustfs_roundtrip() {
    let endpoint = s3_endpoint();
    if !rustfs_available(&endpoint).await {
        eprintln!(
            "Skipping multipart_attachment_flow_with_rustfs_roundtrip: RustFS unavailable at {endpoint}"
        );
        return;
    }

    let state = make_state().await;
    let (owner_id, channel_id) =
        setup_server_channel(&state, "att_flow_owner", "att-flow-channel").await;
    let claims = claims_for(owner_id, "att_flow_owner");

    let ciphertext = b"encrypted attachment payload".to_vec();

    let (status, Json(init)) = init_attachment(
        State(state.clone()),
        Extension(claims.clone()),
        Path(channel_id),
        Json(InitAttachmentRequest {
            file_name: "demo.bin".to_string(),
            mime_type: "application/octet-stream".to_string(),
            size_bytes: ciphertext.len() as i64,
            created_at: None,
            chunk_size_bytes: ciphertext.len() as i64,
            chunk_count: 1,
        }),
    )
    .await
    .expect("init attachment should succeed");

    assert_eq!(status, axum::http::StatusCode::CREATED);

    let Json(sign) = sign_part(
        State(state.clone()),
        Extension(claims.clone()),
        Path((channel_id, init.attachment_id)),
        Json(SignPartRequest {
            upload_id: init.upload_id.clone(),
            part_number: 1,
        }),
    )
    .await
    .expect("sign part should succeed");

    let upload_resp = Client::new()
        .put(&sign.upload_url)
        .body(ciphertext.clone())
        .send()
        .await
        .expect("upload part should return response");
    assert!(
        upload_resp.status().is_success(),
        "upload part failed with status {}",
        upload_resp.status()
    );

    let etag = upload_resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string())
        .expect("upload part response should contain etag header");

    let key_version_id = Uuid::new_v4();
    let Json(complete) = complete_attachment(
        State(state.clone()),
        Extension(claims.clone()),
        Path((channel_id, init.attachment_id)),
        Json(CompleteAttachmentRequest {
            upload_id: init.upload_id,
            parts: vec![CompletePartItem {
                part_number: 1,
                etag,
            }],
            crypto: CompleteCrypto {
                algorithm: "aes-256-gcm".to_string(),
                file_iv: "AAAAAAAAAAAAAAAA".to_string(),
                wrapped_file_key: "BBBBBBBBBBBBBBBB".to_string(),
                key_version_id,
                key_version: 1,
                ciphertext_sha256: "ab".repeat(32),
            },
            thumbnail_attachment_id: None,
        }),
    )
    .await
    .expect("complete attachment should succeed");

    assert_eq!(complete.status, "ready");

    let Json(download) = download_attachment(
        State(state.clone()),
        Extension(claims),
        Path((channel_id, init.attachment_id)),
    )
    .await
    .expect("download metadata should succeed");

    assert_eq!(download.attachment_id, init.attachment_id);
    assert_eq!(download.size_bytes, ciphertext.len() as i64);
    assert_eq!(download.crypto.key_version_id, key_version_id);

    let download_resp = Client::new()
        .get(download.download_url)
        .send()
        .await
        .expect("download signed url should return response");
    assert!(
        download_resp.status().is_success(),
        "download failed with status {}",
        download_resp.status()
    );

    let downloaded = download_resp
        .bytes()
        .await
        .expect("downloaded body should be readable")
        .to_vec();
    assert_eq!(downloaded, ciphertext);
}
