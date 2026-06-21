use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;
use thiserror::Error;

const SALT_KEY: &[u8] = b"_salt";
const VERIFY_KEY: &[u8] = b"_verify";
const VERIFY_PLAIN: &[u8] = b"chillgroup-vault-v1";

#[derive(Debug, Error)]
pub enum VaultError {
    #[error("Contrasenya incorrecta")]
    WrongPassphrase,
    #[error("Vault corrupte o incompatible")]
    Corrupted,
    #[error("Error d'accés: {0}")]
    Sled(#[from] sled::Error),
    #[error("Error de serialització: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("Error criptogràfic")]
    Crypto,
}

pub struct Vault {
    db: sled::Db,
    key: [u8; 32],
}

impl Vault {
    /// Returns true if the vault at `path` has already been initialized with a passphrase.
    pub fn exists(path: &Path) -> bool {
        sled::open(path)
            .ok()
            .and_then(|db| db.get(SALT_KEY).ok().flatten())
            .is_some()
    }

    /// Create a new encrypted vault.
    pub fn create(path: &Path, passphrase: &str) -> Result<Self, VaultError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let db = sled::open(path)?;
        let salt: [u8; 32] = rand::random();
        db.insert(SALT_KEY, salt.as_ref())?;

        let key = derive_key(passphrase.as_bytes(), &salt)?;
        let vault = Self { db, key };
        // Store a known plaintext encrypted so we can detect wrong passphrase on open
        vault.encrypt_insert(VERIFY_KEY, VERIFY_PLAIN)?;
        Ok(vault)
    }

    /// Open an existing encrypted vault. Returns `WrongPassphrase` if passphrase is incorrect.
    pub fn open(path: &Path, passphrase: &str) -> Result<Self, VaultError> {
        let db = sled::open(path)?;
        let salt = db.get(SALT_KEY)?.ok_or(VaultError::Corrupted)?;
        let key = derive_key(passphrase.as_bytes(), &salt)?;
        let vault = Self { db, key };

        // Verify by decrypting the known token — AES-GCM auth tag fails on wrong key
        let plain = vault.decrypt_get(VERIFY_KEY)?.ok_or(VaultError::Corrupted)?;
        if plain != VERIFY_PLAIN {
            return Err(VaultError::WrongPassphrase);
        }
        Ok(vault)
    }

    pub fn set<V: Serialize + ?Sized>(&self, key: &str, value: &V) -> Result<(), VaultError> {
        let plaintext = serde_json::to_vec(value)?;
        self.encrypt_insert(key.as_bytes(), &plaintext)
    }

    pub fn get<V: DeserializeOwned>(&self, key: &str) -> Result<Option<V>, VaultError> {
        let Some(plaintext) = self.decrypt_get(key.as_bytes())? else { return Ok(None) };
        Ok(Some(serde_json::from_slice(&plaintext)?))
    }

    pub fn remove(&self, key: &str) -> Result<(), VaultError> {
        self.db.remove(key)?;
        Ok(())
    }

    pub fn save_session(&self, token: &str, user_id: &str, username: &str, device_id: &str) -> Result<(), VaultError> {
        self.set("session_token", token)?;
        self.set("session_user_id", user_id)?;
        self.set("session_username", username)?;
        self.set("session_device_id", device_id)?;
        Ok(())
    }

    pub fn load_session(&self) -> Result<Option<(String, String, String, String)>, VaultError> {
        let Some(token) = self.get::<String>("session_token")? else { return Ok(None) };
        let user_id   = self.get::<String>("session_user_id")?.unwrap_or_default();
        let username  = self.get::<String>("session_username")?.unwrap_or_default();
        let device_id = self.get::<String>("session_device_id")?.unwrap_or_default();
        Ok(Some((token, user_id, username, device_id)))
    }

    pub fn clear_session(&self) -> Result<(), VaultError> {
        self.db.remove("session_token")?;
        self.db.remove("session_user_id")?;
        self.db.remove("session_username")?;
        self.db.remove("session_device_id")?;
        Ok(())
    }

    // ML-KEM-1024 keypair (dk = decapsulation key, ek = encapsulation/public key)
    pub fn save_kem_keypair(&self, dk: &[u8], ek: &[u8]) -> Result<(), VaultError> {
        self.set("kem_dk", dk)?;
        self.set("kem_ek", ek)?;
        Ok(())
    }

    pub fn load_kem_keypair(&self) -> Result<Option<(Vec<u8>, Vec<u8>)>, VaultError> {
        let Some(dk) = self.get::<Vec<u8>>("kem_dk")? else { return Ok(None) };
        let Some(ek) = self.get::<Vec<u8>>("kem_ek")? else { return Ok(None) };
        Ok(Some((dk, ek)))
    }

    pub fn save_dsa_keypair(&self, sk_seed: &[u8], vk: &[u8]) -> Result<(), VaultError> {
        self.set("dsa_sk", sk_seed)?;
        self.set("dsa_vk", vk)?;
        Ok(())
    }

    pub fn load_dsa_keypair(&self) -> Result<Option<(Vec<u8>, Vec<u8>)>, VaultError> {
        let Some(sk) = self.get::<Vec<u8>>("dsa_sk")? else { return Ok(None) };
        let Some(vk) = self.get::<Vec<u8>>("dsa_vk")? else { return Ok(None) };
        Ok(Some((sk, vk)))
    }

    pub fn save_channel_key(&self, channel_id: &str, key: &[u8]) -> Result<(), VaultError> {
        self.set(&format!("chkey_{channel_id}"), key)
    }

    pub fn load_channel_key(&self, channel_id: &str) -> Result<Option<Vec<u8>>, VaultError> {
        self.get::<Vec<u8>>(&format!("chkey_{channel_id}"))
    }

    pub fn save_channel_key_current_version(&self, channel_id: &str, version: i32, version_id: &str) -> Result<(), VaultError> {
        self.set(&format!("chkeyver_{channel_id}"), &version)?;
        self.set(&format!("chkeyverid_{channel_id}"), version_id)?;
        Ok(())
    }

    pub fn load_channel_key_current_version(&self, channel_id: &str) -> Result<Option<(i32, String)>, VaultError> {
        let Some(v) = self.get::<i32>(&format!("chkeyver_{channel_id}"))? else { return Ok(None) };
        let vid = self.get::<String>(&format!("chkeyverid_{channel_id}"))?.unwrap_or_default();
        Ok(Some((v, vid)))
    }

    pub fn save_channel_key_version(
        &self,
        channel_id: &str,
        version: i32,
        key: &[u8],
        key_version_id: Option<&str>,
    ) -> Result<(), VaultError> {
        self.set(&format!("chkey_{channel_id}_v{version}"), key)?;
        if let Some(vid) = key_version_id {
            self.set(&format!("chkeyid_{channel_id}_v{version}"), vid)?;
        }
        Ok(())
    }

    pub fn load_channel_key_version(&self, channel_id: &str, version: i32) -> Result<Option<Vec<u8>>, VaultError> {
        self.get::<Vec<u8>>(&format!("chkey_{channel_id}_v{version}"))
    }

    pub fn load_channel_key_version_id(&self, channel_id: &str, version: i32) -> Result<Option<String>, VaultError> {
        self.get::<String>(&format!("chkeyid_{channel_id}_v{version}"))
    }

    fn encrypt_insert(&self, key: &[u8], plaintext: &[u8]) -> Result<(), VaultError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, plaintext).map_err(|_| VaultError::Crypto)?;
        // Format: nonce (12 bytes) || ciphertext
        let mut stored = Vec::with_capacity(12 + ciphertext.len());
        stored.extend_from_slice(&nonce);
        stored.extend_from_slice(&ciphertext);
        self.db.insert(key, stored)?;
        Ok(())
    }

    fn decrypt_get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, VaultError> {
        let Some(stored) = self.db.get(key)? else { return Ok(None) };
        if stored.len() < 12 {
            return Err(VaultError::Corrupted);
        }
        let (nonce_bytes, ciphertext) = stored.split_at(12);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let nonce = Nonce::from_slice(nonce_bytes);
        cipher.decrypt(nonce, ciphertext)
            .map(Some)
            .map_err(|_| VaultError::WrongPassphrase)
    }
}

// Argon2id: 64 MB memory, 3 iterations, 1 thread — ~300 ms on modern hardware
fn derive_key(passphrase: &[u8], salt: &[u8]) -> Result<[u8; 32], VaultError> {
    let params = Params::new(65536, 3, 1, Some(32)).map_err(|_| VaultError::Crypto)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase, salt, &mut key)
        .map_err(|_| VaultError::Crypto)?;
    Ok(key)
}
