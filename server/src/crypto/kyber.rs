//! Placeholder — Kyber-1024 KEM no disponible a crates.io encara.
//! Quan hi hagi una crate compatible, reemplaçar aquesta implementació.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::RngCore;

/// Placeholder: genera un "clau pública" simulada.
/// En producció, aquesta funció cridaria a Kyber-1024 real.
pub fn generate_keypair_placeholder() -> String {
    let mut key = [0u8; 32];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut key);
    STANDARD.encode(key)
}