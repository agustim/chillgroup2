//! Hashing de passwords amb Argon2.
//!
//! Implementació real amb `argon2` per generar i verificar hashes segurs.

use crate::error::CryptoError;
use argon2::{password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString}, Argon2};
use rand::rngs::OsRng;

/// Generar un hash de password amb Argon2.
pub fn hash_password(password: &str) -> Result<String, CryptoError> {
    if password.is_empty() {
        return Err(CryptoError::CryptoError);
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| CryptoError::CryptoError)?;

    Ok(password_hash.to_string())
}

/// Verificar un password contra un hash.
pub fn verify_password(password: &str, hash_str: &str) -> Result<bool, CryptoError> {
    let parsed_hash = PasswordHash::new(hash_str).map_err(|_| CryptoError::CryptoError)?;
    let argon2 = Argon2::default();
    Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_returns_non_empty() {
        let password = "TestPassword123!";
        let hash = hash_password(password).expect("Ha de generar hash");
        assert!(!hash.is_empty());
        assert!(hash.starts_with("$argon2id$"));
    }

    #[test]
    fn test_hash_password_different_each_time() {
        let password = "TestPassword123!";
        let hash1 = hash_password(password).expect("Ha de generar hash");
        let hash2 = hash_password(password).expect("Ha de generar hash");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "TestPassword123!";
        let hash = hash_password(password).expect("Ha de generar hash");
        assert!(verify_password(password, &hash).expect("Ha de verificar"));
    }

    #[test]
    fn test_verify_password_incorrect() {
        let correct_password = "TestPassword123!";
        let wrong_password = "WrongPassword456!";
        let hash = hash_password(correct_password).expect("Ha de generar hash");
        assert!(!verify_password(wrong_password, &hash).expect("Ha de verificar"));
    }

    #[test]
    fn test_hash_password_empty() {
        let result = hash_password("");
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_password_contains_argon2_marker() {
        let hash = hash_password("test").expect("Ha de generar hash");
        assert!(hash.contains("$argon2id$"));
    }
}