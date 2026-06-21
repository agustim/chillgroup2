use rust_socketio::{asynchronous::ClientBuilder, Payload};
use serde_json::Value;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub struct VoicePresenceUser {
    pub user_id: String,
    pub username: String,
}

#[derive(Debug, Clone)]
pub enum RealtimeEvent {
    // Socket event `message` — camelCase del servidor Rust
    Message {
        message_id: String,
        channel_id: String,
        sender_user_id: String,
        sender_username: String,
        encrypted_payload: String,  // plaintext per canals none
        iv: String,
        timestamp: String,
        key_version: Option<i32>,
        expires_at: Option<String>,
    },
    ChannelsUpdated {
        server_id: String,
    },
    VoicePresenceSnapshot {
        server_id: String,
        channels: Vec<(String, Vec<VoicePresenceUser>)>,
    },
    VoicePresenceUpdated {
        channel_id: String,
        users: Vec<VoicePresenceUser>,
    },
    Connected,
    Disconnected,
}

fn parse_presence_users(arr: &[Value]) -> Vec<VoicePresenceUser> {
    arr.iter().filter_map(|u| {
        let obj = u.as_object()?;
        let user_id = obj.get("userId").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let username = obj.get("username").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if user_id.is_empty() { return None; }
        Some(VoicePresenceUser { user_id, username })
    }).collect()
}

pub async fn connect(
    server_url: &str,
    token: &str,
    tx: mpsc::Sender<RealtimeEvent>,
) -> Result<rust_socketio::asynchronous::Client, anyhow::Error> {
    let tx_msg = tx.clone();
    let tx_channels = tx.clone();
    let tx_connected = tx.clone();
    let tx_disconnected = tx.clone();
    let tx_vp_snapshot = tx.clone();
    let tx_vp_updated = tx.clone();

    let client = ClientBuilder::new(server_url)
        .auth(serde_json::json!({ "token": token }))
        .on("connect", move |_, _| {
            let tx = tx_connected.clone();
            Box::pin(async move {
                let _ = tx.send(RealtimeEvent::Connected).await;
            })
        })
        .on("disconnect", move |_, _| {
            let tx = tx_disconnected.clone();
            Box::pin(async move {
                let _ = tx.send(RealtimeEvent::Disconnected).await;
            })
        })
        .on("message", move |payload, _| {
            let tx = tx_msg.clone();
            Box::pin(async move {
                if let Payload::Text(values) = payload {
                    if let Some(Value::Object(msg)) = values.into_iter().next() {
                        let get = |k: &str| {
                            msg.get(k)
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string()
                        };
                        let key_version = msg.get("keyVersion")
                            .and_then(|v| v.as_i64())
                            .map(|v| v as i32);
                        let expires_at = msg.get("expiresAt")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        let _ = tx
                            .send(RealtimeEvent::Message {
                                message_id: get("messageId"),
                                channel_id: get("channelId"),
                                sender_user_id: get("senderUserId"),
                                sender_username: get("senderUsername"),
                                encrypted_payload: get("encryptedPayload"),
                                iv: get("iv"),
                                timestamp: get("timestamp"),
                                key_version,
                                expires_at,
                            })
                            .await;
                    }
                }
            })
        })
        .on("server-channels-updated", move |payload, _| {
            let tx = tx_channels.clone();
            Box::pin(async move {
                if let Payload::Text(values) = payload {
                    if let Some(Value::Object(data)) = values.into_iter().next() {
                        let server_id = data
                            .get("serverId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let _ = tx.send(RealtimeEvent::ChannelsUpdated { server_id }).await;
                    }
                }
            })
        })
        .on("voice-presence-snapshot", move |payload, _| {
            let tx = tx_vp_snapshot.clone();
            Box::pin(async move {
                if let Payload::Text(values) = payload {
                    if let Some(Value::Object(data)) = values.into_iter().next() {
                        let server_id = data.get("serverId")
                            .and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let channels = data.get("channels")
                            .and_then(|v| v.as_array())
                            .map(|arr| arr.iter().filter_map(|ch| {
                                let obj = ch.as_object()?;
                                let channel_id = obj.get("channelId")
                                    .and_then(|v| v.as_str())?.to_string();
                                let users = obj.get("users")
                                    .and_then(|v| v.as_array())
                                    .map(|a| parse_presence_users(a))
                                    .unwrap_or_default();
                                Some((channel_id, users))
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        let _ = tx.send(RealtimeEvent::VoicePresenceSnapshot { server_id, channels }).await;
                    }
                }
            })
        })
        .on("voice-presence-updated", move |payload, _| {
            let tx = tx_vp_updated.clone();
            Box::pin(async move {
                if let Payload::Text(values) = payload {
                    if let Some(Value::Object(data)) = values.into_iter().next() {
                        let channel_id = data.get("channelId")
                            .and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let users = data.get("users")
                            .and_then(|v| v.as_array())
                            .map(|a| parse_presence_users(a))
                            .unwrap_or_default();
                        let _ = tx.send(RealtimeEvent::VoicePresenceUpdated { channel_id, users }).await;
                    }
                }
            })
        })
        .connect()
        .await?;

    Ok(client)
}

pub async fn join_channel(
    client: &rust_socketio::asynchronous::Client,
    channel_id: &str,
) -> Result<(), anyhow::Error> {
    client
        .emit("join-channel", serde_json::json!({ "channelId": channel_id }))
        .await?;
    Ok(())
}

pub async fn leave_channel(
    client: &rust_socketio::asynchronous::Client,
    channel_id: &str,
) -> Result<(), anyhow::Error> {
    client
        .emit("leave-channel", serde_json::json!({ "channelId": channel_id }))
        .await?;
    Ok(())
}
