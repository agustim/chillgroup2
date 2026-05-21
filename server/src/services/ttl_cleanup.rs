use std::{collections::HashMap, time::Duration};

use serde_json::json;
use socketioxide::SocketIo;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::DatabasePool;

pub fn spawn_ttl_cleanup(db: DatabasePool, io: SocketIo, interval_minutes: u64) {
    let interval_seconds = interval_minutes.max(1) * 60;

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_seconds));

        loop {
            ticker.tick().await;

            match db.delete_expired_messages().await {
                Ok(deleted) if !deleted.is_empty() => {
                    info!("Purga TTL completada: {} missatges eliminats", deleted.len());

                    // Agrupar per canal i notificar els clients subscrits
                    let mut by_channel: HashMap<Uuid, Vec<String>> = HashMap::new();
                    for (msg_id, channel_id) in deleted {
                        by_channel
                            .entry(channel_id)
                            .or_default()
                            .push(msg_id.to_string());
                    }

                    for (channel_id, message_ids) in by_channel {
                        let room = format!("channel:{channel_id}");
                        let payload = json!({
                            "channelId": channel_id.to_string(),
                            "messageIds": message_ids,
                        });
                        if let Err(e) = io.to(room).emit("messages-expired", &payload).await {
                            warn!("Error emetent messages-expired al canal {channel_id}: {e}");
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("Error executant la purga TTL: {}", e);
                }
            }
        }
    });
}