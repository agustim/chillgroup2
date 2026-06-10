//! Hashing de passwords amb Argon2.
//!
//! Implementació real amb `argon2` per generar i verificar hashes segurs.

use crate::error::CryptoError;
use argon2::{password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString}, Argon2};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

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

/// Rejects invitation codes that are trivially brute-forceable.
/// SHA-256 without a server-side salt means a weak code is rainbow-table crackable.
pub fn validate_admin_invitation_entropy(code: &str) -> Result<(), String> {
    const MIN_LEN: usize = 20;
    if code.len() < MIN_LEN {
        return Err(format!(
            "ONE_ADMIN_INVITATION massa curt ({} caràcters). Mínim {} caràcters.",
            code.len(), MIN_LEN
        ));
    }
    let has_upper = code.chars().any(|c| c.is_uppercase());
    let has_lower = code.chars().any(|c| c.is_lowercase());
    let has_digit = code.chars().any(|c| c.is_ascii_digit());
    if !has_upper || !has_lower || !has_digit {
        return Err(
            "ONE_ADMIN_INVITATION ha de contenir majúscules, minúscules i dígits.".to_string()
        );
    }
    Ok(())
}

/// Hash d'un codi d'invitació admin one-shot.
pub fn hash_admin_invitation_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.trim().as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{:02x}", byte));
    }
    out
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

    #[test]
    fn test_hash_admin_invitation_code_is_deterministic() {
        let hash1 = hash_admin_invitation_code("ADMIN-CODE-ONE");
        let hash2 = hash_admin_invitation_code("ADMIN-CODE-ONE");
        let hash3 = hash_admin_invitation_code("ADMIN-CODE-TWO");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_eq!(hash1.len(), 64);
    }

    #[test]
    fn test_validate_admin_invitation_entropy_valid() {
        assert!(validate_admin_invitation_entropy("ValidCode12345678901").is_ok());
        assert!(validate_admin_invitation_entropy("MyS3cur3AdminC0de!XYZ").is_ok());
    }

    #[test]
    fn test_validate_admin_invitation_entropy_too_short() {
        let result = validate_admin_invitation_entropy("Short1A");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("massa curt"));
    }

    #[test]
    fn test_validate_admin_invitation_entropy_no_uppercase() {
        let result = validate_admin_invitation_entropy("alllowercase12345678");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_admin_invitation_entropy_no_lowercase() {
        let result = validate_admin_invitation_entropy("ALLUPPERCASE12345678");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_admin_invitation_entropy_no_digit() {
        let result = validate_admin_invitation_entropy("NoDigitsHereAtAllXXX");
        assert!(result.is_err());
    }
}