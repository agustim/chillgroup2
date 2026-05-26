import { ml_kem1024 } from '@noble/post-quantum/ml-kem.js'
import { ml_dsa87 } from '@noble/post-quantum/ml-dsa.js'
import type { EncryptionType, Message } from '../types'
import { decryptWithBytes, encryptWithBytes, generateSymmetricKey } from './crypto'
import {
  getChannelKey,
  getChannelKeyVersion,
  getDevicePublicKeys,
  getDeviceSecretKeys,
  getLatestChannelKey,
  storeChannelKey,
  storeDevicePublicKey,
} from './storage'
import { channelGetKey, channelUploadKeys, channelGetMemberDevices, deviceUpdatePublicKey, channelGetAllKeyBundles } from './api'
import { generateAndStoreDeviceKeypair } from './device-keys'

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
  const cleaned = value.trim().replace(/\s+/g, '')
  const normalized = cleaned
    .replace(/-/g, '+')
    .replace(/_/g, '/')
    .padEnd(Math.ceil(cleaned.length / 4) * 4, '=')
  const binary = atob(normalized)
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

function buildSignaturePayload(
  keyVersionId: string,
  deviceId: string,
  kemCiphertext: string,
  encryptedKey: string
): Uint8Array {
  return new TextEncoder().encode(`${keyVersionId}:${deviceId}:${kemCiphertext}:${encryptedKey}`)
}

const SIGN_TEST_PAYLOAD = new TextEncoder().encode('chillgroup-e2ee-signing-check')

async function syncCurrentDevicePublicKeys(deviceId: string): Promise<void> {
  let publicKeys = await getDevicePublicKeys(deviceId)
  let secretKeys = await getDeviceSecretKeys(deviceId)
  let needsRepair = !publicKeys?.kemPublicKey || !publicKeys.dsaPublicKey || !secretKeys?.kemSecretKey || !secretKeys.dsaSecretKey

  if (!needsRepair && publicKeys && secretKeys?.dsaSecretKey) {
    try {
      ml_kem1024.encapsulate(publicKeys.kemPublicKey)
      const signature = ml_dsa87.sign(SIGN_TEST_PAYLOAD, secretKeys.dsaSecretKey)
      const signatureOk = ml_dsa87.verify(signature, SIGN_TEST_PAYLOAD, publicKeys.dsaPublicKey!)
      if (!signatureOk) {
        needsRepair = true
      }
    } catch {
      needsRepair = true
    }
  }

  if (needsRepair) {
    const generated = await generateAndStoreDeviceKeypair(deviceId, true)
    const upload = await deviceUpdatePublicKey(generated.kemPublicKey, generated.dsaPublicKey)
    if (!upload.success) {
      throw new Error(upload.error.message || 'No s\'ha pogut regenerar i sincronitzar la clau pública del dispositiu actual')
    }
    return
  }

  const kemPublicKey = uint8ArrayToBase64(publicKeys.kemPublicKey)
  const dsaPublicKey = uint8ArrayToBase64(publicKeys.dsaPublicKey)
  const upload = await deviceUpdatePublicKey(kemPublicKey, dsaPublicKey)
  if (!upload.success) {
    throw new Error(upload.error.message || 'No s\'ha pogut sincronitzar la clau pública del dispositiu actual')
  }
}

async function cacheChannelMemberPublicKeys(channelId: string): Promise<Map<string, Uint8Array | null>> {
  const devicesResult = await channelGetMemberDevices(channelId)
  const signerKeys = new Map<string, Uint8Array | null>()
  if (!devicesResult.success) return signerKeys

  for (const device of devicesResult.data) {
    try {
      const kemKey = base64ToUint8Array(device.kemPublicKey)
      const dsaKey = device.dsaPublicKey ? base64ToUint8Array(device.dsaPublicKey) : undefined
      await storeDevicePublicKey(device.deviceId, kemKey, dsaKey)
      signerKeys.set(device.deviceId, dsaKey ?? null)
    } catch {
      signerKeys.set(device.deviceId, null)
    }
  }

  return signerKeys
}

async function verifyBundleSignature(
  channelId: string,
  keyVersionId: string,
  deviceId: string,
  encryptedKey: string,
  kemCiphertext: string,
  signature: string,
  signedByDeviceId: string
): Promise<boolean> {
  let signerKeys = await getDevicePublicKeys(signedByDeviceId)
  if (!signerKeys?.dsaPublicKey) {
    const refreshed = await cacheChannelMemberPublicKeys(channelId)
    const refreshedKey = refreshed.get(signedByDeviceId)
    if (refreshedKey) {
      signerKeys = { kemPublicKey: new Uint8Array(), dsaPublicKey: refreshedKey }
    }
  }

  if (!signerKeys?.dsaPublicKey) {
    return false
  }

  const payload = buildSignaturePayload(keyVersionId, deviceId, kemCiphertext, encryptedKey)
  return ml_dsa87.verify(base64ToUint8Array(signature), payload, signerKeys.dsaPublicKey)
}

// ── Distribuir clau de canal a tots els membres ──────────────────

export async function distributeChannelKey(
  channelId: string,
  channelKey: Uint8Array,
  keyVersion = 1,
  keyVersionId?: string | null,
  signerDeviceId?: string
): Promise<{
  discoveredDevices: string[]
  skippedSelfDevices: string[]
  skippedMissingKemDevices: string[]
  uploadedBundleDevices: string[]
  failedDevices: Array<{ deviceId: string; reason: string }>
}> {
  if (signerDeviceId) {
    try {
      await syncCurrentDevicePublicKeys(signerDeviceId)
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Error sincronitzant la clau pública local'
      console.error('[E2EE] No s\'ha pogut sincronitzar la clau pública del dispositiu actual abans de redistribuir', {
        channelId,
        signerDeviceId,
        error: msg,
      })
    }
  }

  const devicesResult = await channelGetMemberDevices(channelId)
  if (!devicesResult.success || !devicesResult.data.length) {
    return {
      discoveredDevices: [],
      skippedSelfDevices: [],
      skippedMissingKemDevices: [],
      uploadedBundleDevices: [],
      failedDevices: [],
    }
  }

  const discoveredDevices = devicesResult.data.map((device) => device.deviceId)
  const skippedSelfDevices: string[] = []
  const skippedMissingKemDevices: string[] = []

  // Obtenir bundles existents per evitar reujats per conflicte (KEM és no-determinista)
  const devicesWithExistingBundle = new Set<string>()
  if (keyVersionId) {
    const existingBundles = await channelGetAllKeyBundles(channelId)
    if (existingBundles.success) {
      for (const b of existingBundles.data) {
        if (b.keyVersionId === keyVersionId) {
          devicesWithExistingBundle.add(b.deviceId)
        }
      }
    }
  }

  let signerSecretKey: Uint8Array | null = null
  if (signerDeviceId && keyVersionId) {
    const deviceSecrets = await getDeviceSecretKeys(signerDeviceId)
    signerSecretKey = deviceSecrets?.dsaSecretKey ?? null
    if (!signerSecretKey) {
      throw new Error('No hi ha clau de signatura local vàlida per redistribuir (ML-DSA)')
    }
  }

  const bundles: Array<{
    deviceId: string
    encryptedKey: string
    kemCiphertext: string
    keyVersion?: number
    signature?: string
    signedByDeviceId?: string
  }> = []
  const failedDevices: string[] = []
  const failureDetails: Array<{ deviceId: string; kemPublicKeyLength: number; decodedLength?: number; reason: string }> = []

  for (const { deviceId, kemPublicKey, dsaPublicKey } of devicesResult.data) {
    if (devicesWithExistingBundle.has(deviceId)) {
      continue
    }
    if (!kemPublicKey) {
      skippedMissingKemDevices.push(deviceId)
      continue
    }
    try {
      const pubKeyBytes = base64ToUint8Array(kemPublicKey)
      const dsaKeyBytes = dsaPublicKey ? base64ToUint8Array(dsaPublicKey) : undefined
      await storeDevicePublicKey(deviceId, pubKeyBytes, dsaKeyBytes)
      const { encryptedKey, kemCiphertext } = await wrapKeyWithKem(channelKey, pubKeyBytes)
      const payload = keyVersionId
        ? buildSignaturePayload(keyVersionId, deviceId, kemCiphertext, encryptedKey)
        : null
      const signature = payload && signerSecretKey
        ? uint8ArrayToBase64(ml_dsa87.sign(payload, signerSecretKey))
        : undefined
      bundles.push({
        deviceId,
        encryptedKey,
        kemCiphertext,
        keyVersion,
        signature,
        signedByDeviceId: signature && signerDeviceId ? signerDeviceId : undefined,
      })
    } catch (err) {
      failedDevices.push(deviceId)
      const reason = err instanceof Error ? err.message : 'Error desconegut encapsulant KEM'
      let decodedLength: number | undefined
      try {
        decodedLength = base64ToUint8Array(kemPublicKey).length
      } catch {
        decodedLength = undefined
      }
      failureDetails.push({
        deviceId,
        kemPublicKeyLength: kemPublicKey.length,
        decodedLength,
        reason,
      })
    }
  }

  if (bundles.length > 0) {
    const uploadResult = await channelUploadKeys(channelId, bundles)
    if (!uploadResult.success) {
      if (uploadResult.error.code === 3008 && keyVersionId) {
        const latestBundles = await channelGetAllKeyBundles(channelId)
        if (latestBundles.success) {
          const coveredDevices = new Set(
            latestBundles.data
              .filter((bundle) => bundle.keyVersionId === keyVersionId)
              .map((bundle) => bundle.deviceId)
          )
          const unresolved = bundles
            .map((bundle) => bundle.deviceId)
            .filter((deviceId) => !coveredDevices.has(deviceId))

          if (unresolved.length === 0) {
            // El servidor ja tenia els bundles necessaris; tractem el conflicte com idempotent.
            return {
              discoveredDevices,
              skippedSelfDevices,
              skippedMissingKemDevices,
              uploadedBundleDevices: bundles.map((bundle) => bundle.deviceId),
              failedDevices: failureDetails.map((detail) => ({
                deviceId: detail.deviceId,
                reason: detail.reason,
              })),
            }
          }
        }
      }
      throw new Error(uploadResult.error.message || 'No s\'ha pogut pujar el bundle de clau de canal')
    }
  }

  if (failedDevices.length > 0) {
    const reasonSummary = failureDetails
      .map((d) => `${d.deviceId}(${d.decodedLength ?? 'n/a'} bytes: ${d.reason})`)
      .join('; ')
    const logPayload = {
      channelId,
      keyVersion,
      keyVersionId,
      signerDeviceId,
      failedDevices,
      failureDetails,
    }

    if (bundles.length === 0) {
      console.error('[E2EE] Error redistribuint clau de canal (cap bundle vàlid)', logPayload)
      throw new Error(`No s'ha pogut xifrar la clau per ${failedDevices.length} dispositiu(s): ${reasonSummary}`)
    }

    console.warn('[E2EE] Redistribució parcial: alguns dispositius no han rebut la clau', {
      ...logPayload,
      uploadedBundles: bundles.length,
    })
  }

  return {
    discoveredDevices,
    skippedSelfDevices,
    skippedMissingKemDevices,
    uploadedBundleDevices: bundles.map((bundle) => bundle.deviceId),
    failedDevices: failureDetails.map((detail) => ({
      deviceId: detail.deviceId,
      reason: detail.reason,
    })),
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
  const secretKeys = await getDeviceSecretKeys(myDeviceId)
  if (!secretKeys?.kemSecretKey) return null

  try {
    if (
      encryptionType === 'asymmetric' && (
        !result.data.keyVersionId ||
        !result.data.signature ||
        !result.data.signedByDeviceId ||
        !(await verifyBundleSignature(
          channelId,
          result.data.keyVersionId,
          result.data.deviceId,
          result.data.encryptedKey,
          result.data.kemCiphertext,
          result.data.signature,
          result.data.signedByDeviceId,
        ))
      )
    ) {
      return null
    }

    const channelKey = await unwrapKeyWithKem(result.data.encryptedKey, result.data.kemCiphertext, secretKeys.kemSecretKey)
    await storeChannelKey(
      channelId,
      channelKey,
      encryptionType,
      result.data.keyVersion ?? 1,
      result.data.keyVersionId ?? null,
    )
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
): Promise<{ encryptedPayload: string; iv: string; keyVersion?: number | null }> {
  if (encryptionType === 'none') {
    return { encryptedPayload: plaintext, iv: '', keyVersion: null }
  }

  let latest = await getLatestChannelKey(channelId)
  let key = latest?.keyBytes ?? null
  let keyVersion = latest?.keyVersion ?? 1

  if (!key) {
    key = await getChannelKey(channelId)
  }

  if (!key) {
    if (myDeviceId) {
      key = await fetchAndStoreChannelKey(channelId, encryptionType, myDeviceId) ?? null
      if (key) {
        latest = await getLatestChannelKey(channelId)
        keyVersion = latest?.keyVersion ?? 1
      }
    }
  }

  if (!key) {
    throw new Error('No hi ha cap clau disponible per aquest canal')
  }

  const { encrypted, iv } = await encryptWithBytes(key, plaintext)
  return { encryptedPayload: encrypted, iv, keyVersion }
}

export async function decryptChannelMessage(
  channelId: string,
  encryptionType: EncryptionType,
  encryptedPayload: string,
  iv: string,
  keyVersion?: number | null
): Promise<string> {
  if (encryptionType === 'none') {
    return encryptedPayload
  }

  const key = keyVersion
    ? await getChannelKeyVersion(channelId, keyVersion)
    : await getChannelKey(channelId)

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
          message.iv,
          message.keyVersion
        )
        return [message.messageId, decrypted] as const
      } catch {
        return [message.messageId, '[No es pot desxifrar: falta clau local]'] as const
      }
    })
  )

  return Object.fromEntries(entries)
}

export async function syncChannelKeys(
  channelId: string,
  encryptionType: EncryptionType,
  myDeviceId: string
): Promise<void> {
  if (encryptionType === 'none') return

  if (encryptionType === 'symmetric') {
    const existing = await getChannelKey(channelId)
    if (!existing) {
      await fetchAndStoreChannelKey(channelId, encryptionType, myDeviceId)
    }
    return
  }

  // asymmetric: fetch ALL bundles and store each version
  const secretKeys = await getDeviceSecretKeys(myDeviceId)
  if (!secretKeys?.kemSecretKey) return

  const result = await channelGetAllKeyBundles(channelId)
  if (!result.success || !result.data.length) return

  for (const bundle of result.data) {
    const { keyVersion, keyVersionId, encryptedKey, kemCiphertext, signature, signedByDeviceId, deviceId } = bundle
    if (keyVersion == null || !encryptedKey || !kemCiphertext) continue

    const existing = await getChannelKeyVersion(channelId, keyVersion)
    if (existing) continue

    if (
      keyVersionId && signature && signedByDeviceId &&
      !(await verifyBundleSignature(channelId, keyVersionId, deviceId, encryptedKey, kemCiphertext, signature, signedByDeviceId))
    ) {
      console.warn('[E2EE] Bundle signatura invàlida, saltant versió', { channelId, keyVersion })
      continue
    }

    try {
      const channelKey = await unwrapKeyWithKem(encryptedKey, kemCiphertext, secretKeys.kemSecretKey)
      await storeChannelKey(channelId, channelKey, encryptionType, keyVersion, keyVersionId ?? null)
    } catch {
      // ignore individual bundle failures
    }
  }
}
