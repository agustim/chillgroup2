use std::{collections::HashMap, time::Duration};

use aws_credential_types::{provider::SharedCredentialsProvider, Credentials};
use aws_sdk_s3::{
    config::{Builder as S3ConfigBuilder, Region},
    types::{Delete, ObjectIdentifier},
    Client as S3Client,
};
use serde_json::json;
use socketioxide::SocketIo;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::DatabasePool;

fn build_s3_client() -> S3Client {
    let endpoint = std::env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".to_string());
    let access_key = std::env::var("S3_ACCESS_KEY_ID").unwrap_or_else(|_| "rustfsadmin".to_string());
    let secret_key = std::env::var("S3_SECRET_ACCESS_KEY").unwrap_or_else(|_| "rustfsadmin".to_string());
    let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let force_path_style = std::env::var("S3_FORCE_PATH_STYLE")
        .ok()
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(true);

    let creds = Credentials::new(access_key, secret_key, None, None, "chillgroup-ttl-cleanup");
    let conf = S3ConfigBuilder::new()
        .region(Region::new(region))
        .credentials_provider(SharedCredentialsProvider::new(creds))
        .endpoint_url(endpoint)
        .force_path_style(force_path_style)
        .build();
    S3Client::from_conf(conf)
}

async fn delete_s3_objects(keys: Vec<String>) {
    if keys.is_empty() {
        return;
    }
    let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "chillgroup-attachments".to_string());
    let s3 = build_s3_client();

    let identifiers: Vec<ObjectIdentifier> = keys
        .iter()
        .filter_map(|k| ObjectIdentifier::builder().key(k).build().ok())
        .collect();

    if identifiers.is_empty() {
        return;
    }

    let delete = match Delete::builder().set_objects(Some(identifiers)).build() {
        Ok(d) => d,
        Err(e) => {
            warn!("Error construint Delete S3: {e}");
            return;
        }
    };

    match s3.delete_objects().bucket(&bucket).delete(delete).send().await {
        Ok(out) => {
            let errors = out.errors();
            if !errors.is_empty() {
                for err in errors {
                    warn!("Error esborrant objecte S3 {:?}: {:?}", err.key(), err.message());
                }
            } else {
                info!("S3: {} objectes esborrats", keys.len());
            }
        }
        Err(e) => warn!("Error cridant delete_objects S3: {e}"),
    }
}

pub fn spawn_ttl_cleanup(db: DatabasePool, io: SocketIo, interval_minutes: u64) {
    let interval_seconds = interval_minutes.max(1) * 60;

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_seconds));

        loop {
            ticker.tick().await;

            match db.delete_expired_messages().await {
                Ok((deleted, object_keys)) if !deleted.is_empty() => {
                    info!("Purga TTL completada: {} missatges eliminats, {} fitxers S3 a esborrar", deleted.len(), object_keys.len());

                    // Esborrar fitxers S3 dels attachments eliminats
                    delete_s3_objects(object_keys).await;

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
