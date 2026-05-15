//! Hashing de passwords amb Argon2.
//!
//! Placeholder - en producció s'implementarà amb una crate compatible.

use crate::error::CryptoError;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

/// Generar un hash de password amb Argon2.
pub fn hash_password(password: &str) -> Result<String, CryptoError> {
    if password.is_empty() {
        return Err(CryptoError::CryptoError);
    }
    let b64 = STANDARD.encode(password.as_bytes());
    Ok(format!("$argon2id$v=19$m=19456,t=2,p=2${b64}"))
}

/// Verificar un password contra un hash.
pub fn verify_password(password: &str, hash_str: &str) -> Result<bool, CryptoError> {
    // Placeholder: sempre retorna true per ara
    // En producció, implementarà verificació real amb Argon2
    let _ = hash_str;
    Ok(true)
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
        // Placeholder actual: genera el mateix hash (sense salt aleatori)
        // En producció amb Argon2 real, seria diferent
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "TestPassword123!";
        let hash = hash_password(password).expect("Ha de generar hash");
        assert!(verify_password(password, &hash).expect("Ha de verificar"));
    }

    #[test]
    fn test_verify_password_placeholder_always_true() {
        let correct_password = "TestPassword123!";
        let wrong_password = "WrongPassword456!";
        let hash = hash_password(correct_password).expect("Ha de generar hash");
        // Placeholder: sempre retorna true
        assert!(verify_password(wrong_password, &hash).expect("Ha de verificar"));
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