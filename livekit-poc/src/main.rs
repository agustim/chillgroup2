use livekit::e2ee::key_provider::{KeyProvider, KeyProviderOptions};
use livekit::e2ee::{E2eeOptions, EncryptionType};
use livekit::prelude::*;
use livekit_api::access_token::{AccessToken, VideoGrants};
use std::time::Duration;

const LIVEKIT_URL: &str = "wss://childgroup-y9gcz2qj.livekit.cloud";
const LIVEKIT_API_KEY: &str = "APIJvkiqjbk5Eys";
const LIVEKIT_API_SECRET: &str = "pZNWt9Iw8QETph2QWO5dWFVqv7jzHVxwzelhwYOxCBT";
const TEST_ROOM: &str = "rust-e2ee-poc";

fn make_token(identity: &str) -> Result<String, Box<dyn std::error::Error>> {
    let token = AccessToken::with_api_key(LIVEKIT_API_KEY, LIVEKIT_API_SECRET)
        .with_identity(identity)
        .with_name(identity)
        .with_grants(VideoGrants {
            room_join: true,
            room: TEST_ROOM.into(),
            can_publish: true,
            can_subscribe: true,
            ..Default::default()
        })
        .to_jwt()?;
    Ok(token)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    println!("=== LiveKit Rust SDK E2EE POC ===\n");

    // Step 1: token
    let token = make_token("rust-poc")?;
    println!("[OK] Token generated");

    // Step 2: E2EE setup — shared-key mode (same as the JS client uses)
    let key_provider = KeyProvider::with_shared_key(
        KeyProviderOptions::default(),
        b"chillgroup-e2ee-poc-shared-key-!".to_vec(), // 32 bytes
    );
    let e2ee_options = E2eeOptions {
        encryption_type: EncryptionType::Gcm,
        key_provider,
    };
    println!("[OK] E2EE options built (EncryptionType::Gcm, shared key)");

    // Step 3: connect
    // RoomOptions is #[non_exhaustive] — must mutate after default()
    let mut connect_opts = RoomOptions::default();
    connect_opts.encryption = Some(e2ee_options);

    println!("Connecting to {} ...", LIVEKIT_URL);
    let (room, mut rx) = Room::connect(LIVEKIT_URL, &token, connect_opts).await?;
    println!("[OK] Connected to room '{}'", room.name());

    // Step 4: send a data packet (goes through E2EE data channel)
    room.local_participant()
        .publish_data(DataPacket {
            payload: b"hello from rust e2ee poc".to_vec(),
            reliable: true,
            ..Default::default()
        })
        .await?;
    println!("[OK] E2EE data packet sent");

    // Step 5: stay a few seconds, watch for events
    println!("Waiting 5s for room events...");
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(5)) => {}
        event = rx.recv() => {
            println!("[EVENT] {:?}", event);
        }
    }

    room.close().await?;
    println!("\n=== RESULT: LiveKit E2EE Rust SDK functional ===");

    Ok(())
}
