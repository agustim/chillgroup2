use rust_socketio::{asynchronous::ClientBuilder, Payload};
use serde_json::Value;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum RealtimeEvent {
    Message {
        channel_id: String,
        author_id: String,
        author_username: String,
        content: String,
        message_id: String,
        created_at: String,
    },
    ChannelsUpdated {
        server_id: String,
    },
    PresenceUpdated {
        user_id: String,
        online: bool,
    },
    Connected,
    Disconnected,
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
                        let _ = tx
                            .send(RealtimeEvent::Message {
                                message_id: get("messageId"),
                                channel_id: get("channelId"),
                                author_id: get("authorId"),
                                author_username: get("authorUsername"),
                                content: get("content"),
                                created_at: get("createdAt"),
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
