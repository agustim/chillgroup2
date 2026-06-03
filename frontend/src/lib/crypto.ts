//! Llibrera de criptografia per a ChillGroup v2.
//!
//! Implementa AES-GCM-256 per a encriptació de missatges.

/**
 * Generar una clau AES-256 per a encriptació de missatges.
 */
export async function generateKey(): Promise<CryptoKey> {
  return await crypto.subtle.generateKey(
    { name: 'AES-GCM', length: 256 },
    true,
    ['encrypt', 'decrypt']
  )
}

/**
 * Encriptar un missatge amb AES-GCM-256.
 * Retorna lobjecte amb encrypted (base64) i iv (base64).
 */
export async function encryptMessage(
  key: CryptoKey,
  plaintext: string
): Promise<{ encrypted: string; iv: string }> {
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const encoder = new TextEncoder()
  const plaintextBuffer = encoder.encode(plaintext)

  const encryptedBuffer = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    key,
    plaintextBuffer
  )

  return {
    encrypted: btoa(String.fromCharCode(...new Uint8Array(encryptedBuffer))),
    iv: btoa(String.fromCharCode(...iv)),
  }
}

/**
 * Desencriptar un missatge amb AES-GCM-256.
 */
export async function decryptMessage(
  key: CryptoKey,
  encrypted: string,
  iv: string
): Promise<string> {
  const encryptedBuffer = new Uint8Array(
    atob(encrypted)
      .split('')
      .map((c) => c.charCodeAt(0))
  )
  const ivBuffer = new Uint8Array(
    atob(iv)
      .split('')
      .map((c) => c.charCodeAt(0))
  )

  const decryptedBuffer = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: ivBuffer },
    key,
    encryptedBuffer
  )

  const decoder = new TextDecoder()
  return decoder.decode(decryptedBuffer)
}

/**
 * Importar una clau des de bytes.
 */
async function importKey(keyBytes: Uint8Array): Promise<CryptoKey> {
  return await crypto.subtle.importKey(
    'raw',
    keyBytes.buffer as ArrayBuffer,
    { name: 'AES-GCM', length: 256 },
    true,
    ['encrypt', 'decrypt']
  )
}

/**
 * Exportar una clau a bytes.
 */
async function exportKey(key: CryptoKey): Promise<Uint8Array> {
  const raw = await crypto.subtle.exportKey('raw', key)
  return new Uint8Array(raw)
}

/**
 * Generar una clau simètrica des de bytes (per a canals simètrics).
 */
export function generateSymmetricKey(): Uint8Array {
  const key = new Uint8Array(32)
  crypto.getRandomValues(key)
  return key
}

/**
 * Encriptar amb clau en format bytes.
 */
export async function encryptWithBytes(
  keyBytes: Uint8Array,
  plaintext: string
): Promise<{ encrypted: string; iv: string }> {
  const key = await importKey(keyBytes)
  return await encryptMessage(key, plaintext)
}

/**
 * Desencriptar amb clau en format bytes.
 */
export async function decryptWithBytes(
  keyBytes: Uint8Array,
  encrypted: string,
  iv: string
): Promise<string> {
  const key = await importKey(keyBytes)
  return await decryptMessage(key, encrypted, iv)
}

/**
 * Encriptar un missatge en text pla directament (convenient per a tests).
 */
export async function encryptPlainText(plaintext: string): Promise<{
  encrypted: string
  iv: string
  keyBytes: Uint8Array
}> {
  const keyBytes = generateSymmetricKey()
  const { encrypted, iv } = await encryptWithBytes(keyBytes, plaintext)
  return { encrypted, iv, keyBytes }
}

/**
 * Verificar que una encriptació produeix resultats diferents (IV aleatori).
 */
export async function verifyUniqueEncryption(plaintext: string): Promise<{
  enc1: { encrypted: string; iv: string }
  enc2: { encrypted: string; iv: string }
  areDifferent: boolean
}> {
  const keyBytes = generateSymmetricKey()
  const enc1 = await encryptWithBytes(keyBytes, plaintext)
  const enc2 = await encryptWithBytes(keyBytes, plaintext)
  return {
    enc1,
    enc2,
    areDifferent: enc1.encrypted !== enc2.encrypted || enc1.iv !== enc2.iv,
  }
}