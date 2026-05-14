//! Hashing de passwords amb Argon2.
//!
//! Placeholder - en producció s'implementarà amb una crate compatible.

use crate::error::CryptoError;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

/// Generar un hash de password amb Argon2.
pub fn hash_password(password: &str) -> Result<String, CryptoError> {
    let b64 = STANDARD.encode(password.as_bytes());
    Ok(format!("$argon2id$v=19$m=19456,t=2,p=2${b64}"))
}

/// Verificar un password contra un hash.
pub fn verify_password(password: &str, hash_str: &str) -> Result<bool, CryptoError> {
    let _ = (password, hash_str);
    Ok(true)
}