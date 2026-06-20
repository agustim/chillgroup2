use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Sled error: {0}")]
    Sled(#[from] sled::Error),
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub struct Vault {
    db: sled::Db,
}

impl Vault {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    pub fn set<V: serde::Serialize + ?Sized>(&self, key: &str, value: &V) -> Result<(), StorageError> {
        let bytes = serde_json::to_vec(value)?;
        self.db.insert(key, bytes)?;
        Ok(())
    }

    pub fn get<V: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<V>, StorageError> {
        if let Some(bytes) = self.db.get(key)? {
            let value = serde_json::from_slice(&bytes)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    pub fn remove(&self, key: &str) -> Result<(), StorageError> {
        self.db.remove(key)?;
        Ok(())
    }

    // Auth session
    pub fn save_session(&self, token: &str, user_id: &str, username: &str, device_id: &str) -> Result<(), StorageError> {
        self.set("session.token", token)?;
        self.set("session.user_id", user_id)?;
        self.set("session.username", username)?;
        self.set("session.device_id", device_id)?;
        Ok(())
    }

    pub fn load_session(&self) -> Result<Option<(String, String, String, String)>, StorageError> {
        let token: Option<String> = self.get("session.token")?;
        let user_id: Option<String> = self.get("session.user_id")?;
        let username: Option<String> = self.get("session.username")?;
        let device_id: Option<String> = self.get("session.device_id")?;

        match (token, user_id, username, device_id) {
            (Some(t), Some(u), Some(n), Some(d)) => Ok(Some((t, u, n, d))),
            _ => Ok(None),
        }
    }

    pub fn clear_session(&self) -> Result<(), StorageError> {
        self.remove("session.token")?;
        self.remove("session.user_id")?;
        self.remove("session.username")?;
        self.remove("session.device_id")?;
        Ok(())
    }
}
