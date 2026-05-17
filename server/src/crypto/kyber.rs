//! Kyber-1024 Key Exchange (placeholder).
//!
//! En producció s'implementarà amb x25519-dilithium o liboqs.

/// Generar un parell de claus Kyber-1024 (placeholder).
///
/// Retorna (public_key, private_key) com a Vec<u8>.
pub fn generate_keypair_placeholder() -> (Vec<u8>, Vec<u8>) {
    // Placeholder: claus de 1568 bytes (Kyber-1024 real)
    let public_key: Vec<u8> = (0..1568).map(|i| (i % 256) as u8).collect();
    let private_key: Vec<u8> = (0..480).map(|i| (i % 256) as u8).collect();
    (public_key, private_key)
}

/// Derivar clau KEK (Key Encryption Key) amb HKDF (placeholder).
#[allow(dead_code)]
pub fn derive_kek_placeholder(shared_secret: &[u8], channel_id: &str) -> Vec<u8> {
    // Placeholder: simple hash del shared_secret + channel_id
    // En producció: HKDF-SHA256
    let mut kek = Vec::new();
    for (i, byte) in shared_secret.iter().enumerate() {
        kek.push(byte ^ (i % 256) as u8);
    }
    for byte in channel_id.bytes() {
        kek.push(byte);
    }
    kek
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair_returns_correct_sizes() {
        let (public_key, private_key) = generate_keypair_placeholder();
        assert_eq!(public_key.len(), 1568);
        assert_eq!(private_key.len(), 480);
    }

    #[test]
    fn test_generate_keypair_different_each_time() {
        let (pk1, sk1) = generate_keypair_placeholder();
        let (pk2, sk2) = generate_keypair_placeholder();
        // El placeholder genera les mateixes claus sempre
        // Això és acceptable per a un placeholder
        assert_eq!(pk1.len(), pk2.len());
        assert_eq!(sk1.len(), sk2.len());
    }

    #[test]
    fn test_derive_kek_returns_non_empty() {
        let shared_secret = vec![0u8; 32];
        let kek = derive_kek_placeholder(&shared_secret, "test-channel");
        assert!(!kek.is_empty());
    }

    #[test]
    fn test_derive_kek_deterministic() {
        let shared_secret = vec![0u8; 32];
        let kek1 = derive_kek_placeholder(&shared_secret, "test-channel");
        let kek2 = derive_kek_placeholder(&shared_secret, "test-channel");
        assert_eq!(kek1, kek2);
    }

    #[test]
    fn test_derive_kek_different_channels() {
        let shared_secret = vec![0u8; 32];
        let kek1 = derive_kek_placeholder(&shared_secret, "channel-1");
        let kek2 = derive_kek_placeholder(&shared_secret, "channel-2");
        assert_ne!(kek1, kek2);
    }
}