use std::time::Duration;

use tracing::{info, warn};

use crate::db::DatabasePool;

pub fn spawn_ttl_cleanup(db: DatabasePool, interval_minutes: u64) {
    let interval_seconds = interval_minutes.max(1) * 60;

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_seconds));

        loop {
            ticker.tick().await;

            match db.delete_expired_messages().await {
                Ok(removed) if removed > 0 => {
                    info!("Purga TTL completada: {} missatges eliminats", removed);
                }
                Ok(_) => {}
                Err(e) => {
                    warn!("Error executant la purga TTL: {}", e);
                }
            }
        }
    });
}