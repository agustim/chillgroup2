import { ml_kem1024 } from '@noble/post-quantum/ml-kem.js'
import type { EncryptionType, Message } from '../types'
import { decryptWithBytes, encryptWithBytes, generateSymmetricKey } from './crypto'
import { getChannelKey, getKeypair, storeChannelKey } from './storage'
import { channelGetKey, channelUploadKeys, channelGetMemberDevices } from './api'

// ── Helpers base64 ───────────────────────────────────────────────

function uint8ArrayToBase64(data: Uint8Array): string {
  let binary = ''
  const chunkSize = 0x8000
  for (let i = 0; i < data.length; i += chunkSize) {
    binary += String.fromCharCode(...data.subarray(i, i + chunkSize))
  }
  return btoa(binary)
}

function base64ToUint8Array(value: string): Uint8Array {
  const binary = atob(value)
  const out = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) out[i] = binary.charCodeAt(i)
  return out
}

// ── KEM wrap / unwrap ────────────────────────────────────────────
// Usem ML-KEM-1024 per encapsular la clau de canal:
//   - encapsulate(recipientPublicKey) → { kemCiphertext, sharedSecret }
//   - XOR la clau de canal amb els primers 32 bytes del sharedSecret (simple wrap)
//   Alternativament usem AES-GCM amb sharedSecret com a clau → més robust.

async function wrapKeyWithKem(
  channelKey: Uint8Array,
  recipientPublicKey: Uint8Array
): Promise<{ encryptedKey: string; kemCiphertext: string }> {
  const { cipherText, sharedSecret } = ml_kem1024.encapsulate(recipientPublicKey)

  // Importar sharedSecret com a clau AES-GCM per encapsular la clau de canal
  const aesKey = await crypto.subtle.importKey('raw', sharedSecret.slice(0, 32), 'AES-GCM', false, ['encrypt'])
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const wrapped = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, aesKey, channelKey)

  // Concatenar iv + wrapped → base64
  const combined = new Uint8Array(iv.length + wrapped.byteLength)
  combined.set(iv, 0)
  combined.set(new Uint8Array(wrapped), iv.length)

  return {
    encryptedKey: uint8ArrayToBase64(combined),
    kemCiphertext: uint8ArrayToBase64(cipherText),
  }
}

async function unwrapKeyWithKem(
  encryptedKeyB64: string,
  kemCiphertextB64: string,
  mySecretKey: Uint8Array
): Promise<Uint8Array> {
  const cipherText = base64ToUint8Array(kemCiphertextB64)
  const sharedSecret = ml_kem1024.decapsulate(cipherText, mySecretKey)

  const combined = base64ToUint8Array(encryptedKeyB64)
  const iv = combined.slice(0, 12)
  const wrapped = combined.slice(12)

  const aesKey = await crypto.subtle.importKey('raw', sharedSecret.slice(0, 32), 'AES-GCM', false, ['decrypt'])
  const channelKeyBytes = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, aesKey, wrapped)
  return new Uint8Array(channelKeyBytes)
}

// ── Distribuir clau de canal a tots els membres ──────────────────

export async function distributeChannelKey(
  channelId: string,
  channelKey: Uint8Array
): Promise<void> {
  const devicesResult = await channelGetMemberDevices(channelId)
  if (!devicesResult.success || !devicesResult.data.length) return

  const bundles: Array<{ deviceId: string; encryptedKey: string; kemCiphertext: string }> = []

  for (const { deviceId, publicKey } of devicesResult.data) {
    if (!publicKey) continue
    try {
      const pubKeyBytes = base64ToUint8Array(publicKey)
      const { encryptedKey, kemCiphertext } = await wrapKeyWithKem(channelKey, pubKeyBytes)
      bundles.push({ deviceId, encryptedKey, kemCiphertext })
    } catch {
      // Saltar dispositius amb clau pública no vàlida
    }
  }

  if (bundles.length > 0) {
    await channelUploadKeys(channelId, bundles)
  }
}

// ── Descarregar i desencapsular clau de canal del servidor ───────

async function fetchAndStoreChannelKey(
  channelId: string,
  encryptionType: EncryptionType,
  myDeviceId: string
): Promise<Uint8Array | null> {
  const result = await channelGetKey(channelId)
  if (!result.success || !result.data) return null

  // Buscar clau privada local
  const secretKey = await getKeypair(myDeviceId)
  if (!secretKey) return null

  try {
    const channelKey = await unwrapKeyWithKem(result.data.encryptedKey, result.data.kemCiphertext, secretKey)
    await storeChannelKey(channelId, channelKey, encryptionType)
    return channelKey
  } catch {
    return null
  }
}

// ── API pública ──────────────────────────────────────────────────

export async function ensureChannelKey(
  channelId: string,
  encryptionType: EncryptionType,
  myDeviceId?: string
): Promise<Uint8Array | null> {
  if (encryptionType === 'none') {
    return null
  }

  // 1. Mirar a IndexedDB primer
  const existing = await getChannelKey(channelId)
  if (existing) {
    return existing
  }

  // 2. Si tenim deviceId, intentar descarregar-la del servidor
  if (myDeviceId) {
    const fromServer = await fetchAndStoreChannelKey(channelId, encryptionType, myDeviceId)
    if (fromServer) return fromServer
  }

  // 3. Si no existeix ni localment ni al servidor → no podem desxifrar
  return null
}

export async function encryptChannelMessage(
  channelId: string,
  encryptionType: EncryptionType,
  plaintext: string,
  myDeviceId?: string
): Promise<{ encryptedPayload: string; iv: string }> {
  if (encryptionType === 'none') {
    return { encryptedPayload: plaintext, iv: '' }
  }

  let key = await getChannelKey(channelId)
  if (!key) {
    if (myDeviceId) {
      key = await fetchAndStoreChannelKey(channelId, encryptionType, myDeviceId) ?? undefined
    }
  }

  if (!key) {
    throw new Error('No hi ha cap clau disponible per aquest canal')
  }

  const { encrypted, iv } = await encryptWithBytes(key, plaintext)
  return { encryptedPayload: encrypted, iv }
}

export async function decryptChannelMessage(
  channelId: string,
  encryptionType: EncryptionType,
  encryptedPayload: string,
  iv: string
): Promise<string> {
  if (encryptionType === 'none') {
    return encryptedPayload
  }

  const key = await getChannelKey(channelId)
  if (!key) {
    throw new Error('Falta la clau local del canal')
  }

  return decryptWithBytes(key, encryptedPayload, iv)
}

export async function decryptMessagesForChannel(
  channelId: string,
  encryptionType: EncryptionType,
  messages: Message[]
): Promise<Record<string, string>> {
  const entries = await Promise.all(
    messages.map(async (message) => {
      if (message.deletedAt) {
        return [message.messageId, 'Missatge eliminat'] as const
      }

      try {
        const decrypted = await decryptChannelMessage(
          channelId,
          encryptionType,
          message.encryptedPayload,
          message.iv
        )
        return [message.messageId, decrypted] as const
      } catch {
        return [message.messageId, '[No es pot desxifrar: falta clau local]'] as const
      }
    })
  )

  return Object.fromEntries(entries)
}
