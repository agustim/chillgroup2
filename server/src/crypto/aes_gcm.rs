//! AES-256-GCM Encrypt/Decrypt (placeholder).
//!
//! En producció s'implementarà amb el crate aes-gcm.

use crate::error::CryptoError;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::Rng;

/// Mida de l'IV per a AES-256-GCM (12 bytes).
pub const IV_SIZE: usize = 12;

/// Generar una clau aleatòria de 32 bytes (AES-256).
pub fn generate_key() -> Result<Vec<u8>, CryptoError> {
    let mut key = vec![0u8; 32];
    rand::thread_rng().fill(&mut key[..]);
    Ok(key)
}

/// Generar un IV aleatori de 12 bytes.
pub fn generate_iv() -> Result<Vec<u8>, CryptoError> {
    let mut iv = vec![0u8; IV_SIZE];
    rand::thread_rng().fill(&mut iv[..]);
    Ok(iv)
}

/// Encriptar dades amb AES-256-GCM (placeholder).
///
/// En producció usarà AES-GCM real amb la clau i IV proporcionats.
#[allow(dead_code)]
pub fn encrypt(plaintext: &[u8], _key: &[u8], iv: &[u8]) -> Result<Vec<u8>, CryptoError> {
    // Placeholder: retorna el text pla codificat en base64 amb l'IV prependit
    if iv.len() != IV_SIZE {
        return Err(CryptoError::InvalidKeySize {
            expected: IV_SIZE,
            actual: iv.len(),
        });
    }
    let mut encrypted = iv.to_vec();
    encrypted.extend_from_slice(plaintext);
    Ok(encrypted)
}

/// Desencriptar dades amb AES-256-GCM (placeholder).
#[allow(dead_code)]
pub fn decrypt(encrypted: &[u8], _key: &[u8], iv: &[u8]) -> Result<Vec<u8>, CryptoError> {
    // Placeholder: retorna les dades sense desencriptar (només l'IV és correcte)
    if iv.len() != IV_SIZE {
        return Err(CryptoError::InvalidKeySize {
            expected: IV_SIZE,
            actual: iv.len(),
        });
    }
    if encrypted.len() < IV_SIZE {
        return Err(CryptoError::CryptoError);
    }
    Ok(encrypted[IV_SIZE..].to_vec())
}

/// Encriptar bytes a base64.
#[allow(dead_code)]
pub fn encrypt_bytes(plaintext: &str, key: &[u8], iv: &[u8]) -> Result<String, CryptoError> {
    let encrypted = encrypt(plaintext.as_bytes(), key, iv)?;
    Ok(STANDARD.encode(&encrypted))
}

/// Desencriptar base64 a bytes.
#[allow(dead_code)]
pub fn decrypt_bytes(encrypted_b64: &str, key: &[u8], iv: &[u8]) -> Result<String, CryptoError> {
    let encrypted = STANDARD.decode(encrypted_b64)?;
    let decrypted = decrypt(&encrypted, key, iv)?;
    Ok(String::from_utf8(decrypted)?)
}

/// Encriptar amb clau generada automàticament.
#[allow(dead_code)]
pub fn encrypt_with_key(plaintext: &str) -> Result<(String, Vec<u8>, Vec<u8>), CryptoError> {
    let key = generate_key()?;
    let iv = generate_iv()?;
    let encrypted = encrypt_bytes(plaintext, &key, &iv)?;
    Ok((encrypted, key, iv))
}

/// Desencriptar amb clau i IV coneguts.
#[allow(dead_code)]
pub fn decrypt_with_key(encrypted: &str, key: &[u8], iv: &[u8]) -> Result<String, CryptoError> {
    decrypt_bytes(encrypted, key, iv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key_returns_32_bytes() {
        let key = generate_key().expect("Ha de generar clau");
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_generate_key_different_each_time() {
        let key1 = generate_key().expect("Ha de generar clau");
        let key2 = generate_key().expect("Ha de generar clau");
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_generate_iv_returns_12_bytes() {
        let iv = generate_iv().expect("Ha de generar IV");
        assert_eq!(iv.len(), IV_SIZE);
    }

    #[test]
    fn test_generate_iv_different_each_time() {
        let iv1 = generate_iv().expect("Ha de generar IV");
        let iv2 = generate_iv().expect("Ha de generar IV");
        assert_ne!(iv1, iv2);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = generate_key().expect("Ha de generar clau");
        let iv = generate_iv().expect("Ha de generar IV");
        let plaintext = "Hola, això és un missatge secret!";

        let encrypted = encrypt(plaintext.as_bytes(), &key, &iv).expect("Ha d'encriptar");
        let decrypted = decrypt(&encrypted, &key, &iv).expect("Ha de desencriptar");

        assert_eq!(decrypted, plaintext.as_bytes());
    }

    #[test]
    fn test_encrypt_with_invalid_iv_size() {
        let key = generate_key().expect("Ha de generar clau");
        let iv = vec![0u8; 5]; // Mida incorrecta
        let result = encrypt(b"test", &key, &iv);
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_decrypt_empty_message() {
        let key = generate_key().expect("Ha de generar clau");
        let iv = generate_iv().expect("Ha de generar IV");

        let encrypted = encrypt(b"", &key, &iv).expect("Ha d'encriptar");
        let decrypted = decrypt(&encrypted, &key, &iv).expect("Ha de desencriptar");

        assert_eq!(decrypted, b"");
    }

    #[test]
    fn test_encrypt_decrypt_with_base64() {
        let key = generate_key().expect("Ha de generar clau");
        let iv = generate_iv().expect("Ha de generar IV");
        let plaintext = "Missatge amb base64!";

        let encrypted = encrypt_bytes(plaintext, &key, &iv).expect("Ha d'encriptar");
        let decrypted = decrypt_bytes(&encrypted, &key, &iv).expect("Ha de desencriptar");

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_with_key_returns_valid_format() {
        let plaintext = "Missatge amb clau automàtica";
        let (encrypted, key, iv) = encrypt_with_key(plaintext).expect("Ha d'encriptar");

        assert!(!encrypted.is_empty());
        assert_eq!(key.len(), 32);
        assert_eq!(iv.len(), IV_SIZE);

        // Desencriptar de nou
        let decrypted = decrypt_with_key(&encrypted, &key, &iv).expect("Ha de desencriptar");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_with_invalid_iv_size() {
        let key = generate_key().expect("Ha de generar clau");
        let iv = vec![0u8; 5]; // Mida incorrecta
        let result = decrypt(b"test", &key, &iv);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_too_short() {
        let key = generate_key().expect("Ha de generar clau");
        let iv = generate_iv().expect("Ha de generar IV");
        let result = decrypt(b"short", &key, &iv);
        assert!(result.is_err());
    }

    #[test]
    fn test_iv_size_constant() {
        assert_eq!(IV_SIZE, 12);
    }
}