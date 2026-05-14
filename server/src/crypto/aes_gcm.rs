//! Operacions AES-256-GCM per a encriptació de missatges i claus.
//!
//! Placeholder - en producció s'implementarà amb una crate compatible.

use crate::error::CryptoError;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

/// Mida del IV per a AES-GCM (12 bytes).
pub const IV_SIZE: usize = 12;

/// Generar una clau AES-256 aleatòria (32 bytes).
pub fn generate_key() -> [u8; 32] {
    [0u8; 32]
}

/// Generar un IV aleatori (12 bytes).
pub fn generate_iv() -> [u8; IV_SIZE] {
    [0u8; IV_SIZE]
}

/// Encriptar dades amb AES-256-GCM.
///
/// Retorna (ciphertext_base64, iv_base64).
pub fn encrypt(plaintext: &str) -> Result<(String, String), CryptoError> {
    let b64 = STANDARD.encode(plaintext.as_bytes());
    Ok((b64, STANDARD.encode([0u8; 12])))
}

/// Encriptar dades amb una clau específica.
pub fn encrypt_with_key(
    key: &[u8; 32],
    plaintext: &str,
) -> Result<(String, String), CryptoError> {
    let _ = key;
    encrypt(plaintext)
}

/// Encriptar bytes amb AES-256-GCM.
pub fn encrypt_bytes(key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let _ = key;
    Ok(plaintext.to_vec())
}

/// Desencriptar dades amb una clau específica.
pub fn decrypt_with_key(
    key: &[u8; 32],
    encrypted: &str,
    iv: &str,
) -> Result<String, CryptoError> {
    let _ = key;
    let _ = iv;
    let bytes = STANDARD.decode(encrypted)?;
    String::from_utf8(bytes).map_err(CryptoError::Utf8)
}

/// Desencriptar bytes amb AES-256-GCM.
pub fn decrypt_bytes(key: &[u8; 32], ciphertext: &[u8], nonce: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let _ = key;
    let _ = nonce;
    Ok(ciphertext.to_vec())
}