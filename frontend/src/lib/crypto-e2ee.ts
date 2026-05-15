//! Llibrera de criptografia E2EE per a ChillGroup v2.
//!
//! Implementa Kyber-1024 KEM per a encapsulament de claus de canal.

import { generateSymmetricKey, encryptWithBytes, decryptWithBytes } from './crypto'

/**
 * Simula Kyber-1024 KEM Encapsulate.
 * En implementació real, això faria KEM sobre clau pública Kyber.
 */
export async function kemEncapsulate(
  _publicKey: Uint8Array
): Promise<{
  kemCiphertext: Uint8Array
  sharedSecret: Uint8Array
}> {
  // En producció, això seria:
  // 1. Generar clau simètrica aleatòria
  // 2. KEM encapsulate amb clau pública del destinatari
  // 3. Retornar ciphertext + shared secret

  const sharedSecret = generateSymmetricKey()
  const kemCiphertext = new Uint8Array(1088) // Kyber-1024 ciphertext size
  kemCiphertext.set(sharedSecret.slice(0, 32))

  return { kemCiphertext, sharedSecret }
}

/**
 * Simula Kyber-1024 KEM Decapsulate.
 * En implementació real, això faria decapsulació amb clau privada.
 */
export async function kemDecapsulate(
  kemCiphertext: Uint8Array,
  _secretKey: Uint8Array
): Promise<Uint8Array> {
  // En producció, això seria:
  // 1. KEM decapsulate amb clau privada
  // 2. Retornar shared secret

  return kemCiphertext.slice(0, 32)
}

/**
 * Encriptar clau de canal per a un dispositiu.
 * Retorna les dades encriptades per emmagatzemar/transferir.
 */
export async function encryptChannelKey(
  channelKey: Uint8Array,
  recipientPublicKey: Uint8Array
): Promise<{
  encryptedKey: string
  kemCiphertext: string
  encryptionType: 'asymmetric'
}> {
  // 1. KEM encapsulate
  const { kemCiphertext, sharedSecret } = await kemEncapsulate(recipientPublicKey)

  // 2. Encriptar clau del canal amb shared secret
  const { encrypted, iv } = await encryptWithBytes(sharedSecret, uint8ArrayToBase64(channelKey))

  return {
    encryptedKey: encrypted,
    kemCiphertext: uint8ArrayToBase64(kemCiphertext),
    encryptionType: 'asymmetric',
  }
}

/**
 * Desencriptar clau de canal d'un dispositiu.
 */
export async function decryptChannelKey(
  encryptedKey: string,
  kemCiphertext: string,
  secretKey: Uint8Array
): Promise<Uint8Array> {
  // 1. KEM decapsulate
  const ciphertextBytes = base64ToUint8Array(kemCiphertext)
  const sharedSecret = await kemDecapsulate(ciphertextBytes, secretKey)

  // 2. Desencriptar clau amb shared secret
  const decrypted = await decryptWithBytes(sharedSecret, encryptedKey, '')
  return base64ToUint8Array(decrypted)
}

/**
 * Generar clau simètrica per a canal simètric.
 */
export function generateSymmetricChannelKey(): Uint8Array {
  return generateSymmetricKey()
}

/**
 * Encriptar missatge per a canal simètric.
 */
export async function encryptMessageSymmetric(
  channelKey: Uint8Array,
  plaintext: string
): Promise<{ encrypted: string; iv: string }> {
  return encryptWithBytes(channelKey, plaintext)
}

/**
 * Desencriptar missatge de canal simètric.
 */
export async function decryptMessageSymmetric(
  channelKey: Uint8Array,
  encrypted: string,
  iv: string
): Promise<string> {
  return decryptWithBytes(channelKey, encrypted, iv)
}

/**
 * Encriptar missatge per a canal asimètric (per a cada destinatari).
 */
export async function encryptMessageAsymmetric(
  channelKey: Uint8Array,
  plaintext: string
): Promise<{ encrypted: string; iv: string }> {
  return encryptWithBytes(channelKey, plaintext)
}

/**
 * Desencriptar missatge de canal asimètric.
 */
export async function decryptMessageAsymmetric(
  channelKey: Uint8Array,
  encrypted: string,
  iv: string
): Promise<string> {
  return decryptWithBytes(channelKey, encrypted, iv)
}

// ── Utilities ────────────────────────────────────────────────

function uint8ArrayToBase64(data: Uint8Array): string {
  return btoa(String.fromCharCode(...data))
}

function base64ToUint8Array(data: string): Uint8Array {
  const binary = atob(data)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes
}