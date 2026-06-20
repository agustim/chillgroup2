use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit as AesKeyInit, OsRng},
    Aes256Gcm, Key as AesKey, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use ml_kem::{Decapsulate, Kem, KeyExport, MlKem1024, Seed, ml_kem_1024};

/// Generate a new ML-KEM-1024 keypair.
/// Returns (dk_seed_bytes[64], ek_bytes[1568]).
pub fn generate_kem_keypair() -> (Vec<u8>, Vec<u8>) {
    let (dk, ek) = MlKem1024::generate_keypair();
    let dk_seed: Seed = dk.to_bytes();
    let dk_bytes: Vec<u8> = <Seed as AsRef<[u8]>>::as_ref(&dk_seed).to_vec();
    let ek_key = ek.to_bytes();
    let ek_bytes: Vec<u8> = ek_key.as_slice().to_vec();
    (dk_bytes, ek_bytes)
}

/// Unwrap a channel key from server key bundle.
/// Server format: encryptedKey = base64(nonce[12] || aes_gcm_ciphertext)
///                kemCiphertext = base64(ml-kem-1024 ciphertext[1568 bytes])
pub fn unwrap_channel_key(
    dk_bytes: &[u8],
    encrypted_key_b64: &str,
    kem_ciphertext_b64: &str,
) -> Result<[u8; 32], String> {
    let kem_ct_bytes = STANDARD.decode(kem_ciphertext_b64).map_err(|e| e.to_string())?;
    let wrapped = STANDARD.decode(encrypted_key_b64).map_err(|e| e.to_string())?;
    if wrapped.len() < 12 {
        return Err("encryptedKey too short".into());
    }

    // Restore DecapsulationKey from 64-byte seed
    if dk_bytes.len() != 64 {
        return Err(format!("dk must be 64 bytes, got {}", dk_bytes.len()));
    }
    let mut seed_arr = [0u8; 64];
    seed_arr.copy_from_slice(dk_bytes);
    let seed = Seed::from(seed_arr);
    let dk = ml_kem_1024::DecapsulationKey::from_seed(seed);

    // ML-KEM decapsulate: shared secret from KEM ciphertext
    let shared_secret = dk.decapsulate_slice(&kem_ct_bytes)
        .map_err(|_| "invalid KEM ciphertext size")?;

    let mut wrapping_key = [0u8; 32];
    wrapping_key.copy_from_slice(shared_secret.as_ref());

    // AES-256-GCM decrypt: nonce[..12] || ciphertext[12..]
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(&wrapping_key));
    let nonce = Nonce::from_slice(&wrapped[..12]);
    let channel_key_bytes = cipher
        .decrypt(nonce, &wrapped[12..])
        .map_err(|_| "AES-GCM unwrap failed")?;

    if channel_key_bytes.len() != 32 {
        return Err(format!("expected 32-byte channel key, got {}", channel_key_bytes.len()));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&channel_key_bytes);
    Ok(key)
}

/// Decrypt an AES-256-GCM message payload.
/// encrypted_payload_b64: base64(ciphertext), iv_b64: base64(12-byte nonce)
pub fn decrypt_message(
    channel_key: &[u8; 32],
    encrypted_payload_b64: &str,
    iv_b64: &str,
) -> Result<String, String> {
    let ciphertext = STANDARD.decode(encrypted_payload_b64).map_err(|e| e.to_string())?;
    let iv = STANDARD.decode(iv_b64).map_err(|e| e.to_string())?;
    if iv.len() != 12 {
        return Err(format!("invalid IV length: {}", iv.len()));
    }
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(channel_key));
    let nonce = Nonce::from_slice(&iv);
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_slice())
        .map_err(|_| "AES-GCM decrypt failed")?;
    String::from_utf8(plaintext).map_err(|e| e.to_string())
}

/// Encrypt a message with AES-256-GCM.
/// Returns (encrypted_payload_b64, iv_b64).
pub fn encrypt_message(channel_key: &[u8; 32], plaintext: &str) -> (String, String) {
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(channel_key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .expect("AES-GCM encrypt cannot fail");
    (STANDARD.encode(&ciphertext), STANDARD.encode(nonce.as_slice()))
}

/// Simulate server-side key wrapping (for tests).
/// Returns (encryptedKey_b64, kemCiphertext_b64) matching the server format.
#[cfg(test)]
pub fn server_wrap_channel_key(ek_bytes: &[u8], channel_key: &[u8; 32]) -> (String, String) {
    use ml_kem::{Encapsulate, TryKeyInit, Key, ml_kem_1024::EncapsulationKey};

    // Restore EK from bytes
    let mut ek_arr: Key<EncapsulationKey> = Default::default();
    ek_arr.as_mut_slice().copy_from_slice(ek_bytes);
    let ek = EncapsulationKey::new(&ek_arr).expect("invalid EK bytes");

    // ML-KEM encapsulate (uses getrandom feature)
    let (kem_ct, shared_secret) = ek.encapsulate();

    let mut wrapping_key = [0u8; 32];
    wrapping_key.copy_from_slice(shared_secret.as_ref());

    // AES-256-GCM wrap channel key: nonce[12] || ciphertext
    let cipher = Aes256Gcm::new(AesKey::<Aes256Gcm>::from_slice(&wrapping_key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let wrapped_key = cipher.encrypt(&nonce, channel_key.as_slice()).unwrap();

    let mut envelope = Vec::with_capacity(12 + wrapped_key.len());
    envelope.extend_from_slice(&nonce);
    envelope.extend_from_slice(&wrapped_key);

    (STANDARD.encode(&envelope), STANDARD.encode(kem_ct.as_slice()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kem_roundtrip() {
        let (dk_bytes, ek_bytes) = generate_kem_keypair();
        assert_eq!(dk_bytes.len(), 64,  "dk seed must be 64 bytes");
        assert_eq!(ek_bytes.len(), 1568, "ek must be 1568 bytes");

        // Fake 32-byte channel key (as server would have)
        let channel_key: [u8; 32] = rand::random();

        let (enc_key_b64, kem_ct_b64) = server_wrap_channel_key(&ek_bytes, &channel_key);

        let unwrapped = unwrap_channel_key(&dk_bytes, &enc_key_b64, &kem_ct_b64)
            .expect("unwrap_channel_key failed");

        assert_eq!(unwrapped, channel_key, "unwrapped key must match original");
    }

    #[test]
    fn test_message_roundtrip() {
        let key: [u8; 32] = rand::random();
        let plaintext = "Hola, món xifrat!";

        let (ciphertext_b64, iv_b64) = encrypt_message(&key, plaintext);
        let decrypted = decrypt_message(&key, &ciphertext_b64, &iv_b64)
            .expect("decrypt_message failed");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_full_e2e_chain() {
        // Full chain: generate keypair → server wraps channel key → client unwraps → encrypt/decrypt message
        let (dk_bytes, ek_bytes) = generate_kem_keypair();
        let channel_key: [u8; 32] = rand::random();

        let (enc_key_b64, kem_ct_b64) = server_wrap_channel_key(&ek_bytes, &channel_key);
        let unwrapped = unwrap_channel_key(&dk_bytes, &enc_key_b64, &kem_ct_b64).unwrap();

        let plaintext = "Missatge secret per al canal E2EE";
        let (ciphertext_b64, iv_b64) = encrypt_message(&unwrapped, plaintext);
        let decrypted = decrypt_message(&unwrapped, &ciphertext_b64, &iv_b64).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key: [u8; 32] = rand::random();
        let wrong_key: [u8; 32] = rand::random();
        let (ct, iv) = encrypt_message(&key, "secret");
        assert!(decrypt_message(&wrong_key, &ct, &iv).is_err());
    }
}
