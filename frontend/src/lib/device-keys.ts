import { ml_kem1024 } from '@noble/post-quantum/ml-kem.js'
import { ml_dsa87 } from '@noble/post-quantum/ml-dsa.js'

import {
  deleteChannelKey,
  deleteDeviceSecretKeys,
  deleteNamedKeypair,
  getAllChannelKeys,
  getChannelKey,
  getDevicePublicKeys,
  getDeviceSecretKeys,
  getNamedKeypair,
  listNamedKeypairs,
  listChannelKeys,
  storeChannelKey,
  storeDevicePublicKey,
  storeDeviceSecretKeys,
  upsertNamedKeypair,
} from './storage'

export { listChannelKeys } from './storage'

function uint8ArrayToBase64(data: Uint8Array): string {
  let binary = ''
  const chunkSize = 0x8000
  for (let i = 0; i < data.length; i += chunkSize) {
    const chunk = data.subarray(i, i + chunkSize)
    binary += String.fromCharCode(...chunk)
  }
  return btoa(binary)
}

function base64ToUint8Array(value: string): Uint8Array {
  const binary = atob(value)
  const out = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    out[i] = binary.charCodeAt(i)
  }
  return out
}

function toArrayBuffer(data: Uint8Array): ArrayBuffer {
  const out = new ArrayBuffer(data.byteLength)
  new Uint8Array(out).set(data)
  return out
}

// ── Backup encryption (PBKDF2 + AES-256-GCM, client-side) ────

const KDF_ITERATIONS = 600_000

interface EncryptedBackupBundle {
  encrypted: true
  version: 1
  algorithm: 'AES-GCM'
  kdf: 'PBKDF2'
  kdfHash: 'SHA-256'
  kdfIterations: number
  salt: string  // base64, 16 bytes
  iv: string    // base64, 12 bytes
  ciphertext: string  // base64
}

async function deriveKey(password: string, salt: Uint8Array): Promise<CryptoKey> {
  const safeSalt = toArrayBuffer(salt)
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(password),
    { name: 'PBKDF2' },
    false,
    ['deriveKey']
  )
  return crypto.subtle.deriveKey(
    { name: 'PBKDF2', salt: safeSalt, iterations: KDF_ITERATIONS, hash: 'SHA-256' },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  )
}

/**
 * Xifra un string JSON amb AES-256-GCM derivat de la contrasenya (PBKDF2).
 * Retorna un JSON wrapper amb salt, iv i ciphertext en base64.
 */
export async function encryptBackup(plaintext: string, password: string): Promise<string> {
  const salt = crypto.getRandomValues(new Uint8Array(16))
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const safeIv = toArrayBuffer(iv)
  const key = await deriveKey(password, salt)
  const ciphertextBuf = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv: safeIv },
    key,
    new TextEncoder().encode(plaintext)
  )
  const bundle: EncryptedBackupBundle = {
    encrypted: true,
    version: 1,
    algorithm: 'AES-GCM',
    kdf: 'PBKDF2',
    kdfHash: 'SHA-256',
    kdfIterations: KDF_ITERATIONS,
    salt: uint8ArrayToBase64(salt),
    iv: uint8ArrayToBase64(iv),
    ciphertext: uint8ArrayToBase64(new Uint8Array(ciphertextBuf)),
  }
  return JSON.stringify(bundle, null, 2)
}

/**
 * Desxifra un backup xifrat amb encryptBackup.
 * Si el fitxer no està xifrat (no té `encrypted: true`), el retorna tal qual.
 * Llança un error si la contrasenya és incorrecta o el fitxer és corrupte.
 */
export async function decryptBackup(fileText: string, password: string): Promise<string> {
  let parsed: unknown
  try {
    parsed = JSON.parse(fileText)
  } catch {
    throw new Error('Format JSON invàlid')
  }

  if (typeof parsed !== 'object' || parsed === null || !(parsed as EncryptedBackupBundle).encrypted) {
    return fileText  // no xifrat, retornar tal qual
  }

  const bundle = parsed as EncryptedBackupBundle
  const salt = base64ToUint8Array(bundle.salt)
  const iv = toArrayBuffer(base64ToUint8Array(bundle.iv))
  const ciphertext = toArrayBuffer(base64ToUint8Array(bundle.ciphertext))
  const key = await deriveKey(password, salt)

  let plaintextBuf: ArrayBuffer
  try {
    plaintextBuf = await crypto.subtle.decrypt({ name: 'AES-GCM', iv }, key, ciphertext)
  } catch {
    throw new Error('Contrasenya incorrecta o fitxer corrupte')
  }

  return new TextDecoder().decode(plaintextBuf)
}

/**
 * Comprova si un text de fitxer correspon a un backup xifrat.
 */
export function isEncryptedBackup(fileText: string): boolean {
  try {
    const parsed = JSON.parse(fileText) as Partial<EncryptedBackupBundle>
    return parsed.encrypted === true
  } catch {
    return false
  }
}

export interface DeviceKeypairBundle {
  version: 1
  kemAlgorithm: 'ML-KEM-1024'
  dsaAlgorithm: 'ML-DSA-87'
  deviceId: string
  createdAt: number
  kemPublicKey: string
  kemSecretKey: string
  dsaPublicKey: string
  dsaSecretKey: string
}

export interface SymmetricChannelsBundle {
  version: 1
  exportedAt: number
  channels: Array<{
    channelId: string
    keyVersion: number
    key: string
    acquiredAt: number
  }>
}

export interface AsymmetricChannelsBundle {
  version: 1
  exportedAt: number
  channels: Array<{
    channelId: string
    keyVersion: number
    keyVersionId?: string | null
    key: string
    acquiredAt: number
  }>
}

export interface NamedKeypairItem {
  deviceId: string
  createdAt: number
  updatedAt: number
}

export class KeypairDeviceIdExistsError extends Error {
  constructor(deviceId: string) {
    super(`Ja existeix un parell de claus amb el deviceId "${deviceId}"`)
    this.name = 'KeypairDeviceIdExistsError'
  }
}

function normalizeDeviceId(deviceId: string): string {
  return deviceId.trim().toLowerCase()
}

export async function hasLocalDeviceKeypair(deviceId: string): Promise<boolean> {
  const secretKeys = await getDeviceSecretKeys(deviceId)
  if (secretKeys?.kemSecretKey && secretKeys.dsaSecretKey) {
    return true
  }

  const named = await getNamedKeypair(deviceId)
  if (named?.secretKey && named.publicKey && named.dsaSecretKey && named.dsaPublicKey) {
    return true
  }

  return false
}

export async function generateAndStoreDeviceKeypair(
  deviceId: string,
  overwrite = false
): Promise<{ kemPublicKey: string; dsaPublicKey: string }> {
  const safeDeviceId = deviceId.trim()
  if (!safeDeviceId) {
    throw new Error('Has d\'indicar un deviceId pel parell de claus')
  }

  const existing = await listNamedKeypairs()
  const duplicated = existing.some((item) => normalizeDeviceId(item.name) === normalizeDeviceId(safeDeviceId))
  if (duplicated && !overwrite) {
    throw new KeypairDeviceIdExistsError(safeDeviceId)
  }

  const { secretKey, publicKey } = ml_kem1024.keygen()
  const dsa = ml_dsa87.keygen()
  await upsertNamedKeypair(safeDeviceId, safeDeviceId, secretKey, publicKey, dsa.secretKey, dsa.publicKey)
  await storeDeviceSecretKeys(safeDeviceId, secretKey, dsa.secretKey)
  await storeDevicePublicKey(safeDeviceId, publicKey, dsa.publicKey)

  return {
    kemPublicKey: uint8ArrayToBase64(publicKey),
    dsaPublicKey: uint8ArrayToBase64(dsa.publicKey),
  }
}

export async function importAndStoreDeviceKeypair(
  bundleText: string,
  overwrite = false
): Promise<DeviceKeypairBundle> {
  let parsed: DeviceKeypairBundle
  try {
    parsed = JSON.parse(bundleText) as DeviceKeypairBundle
  } catch {
    throw new Error('Format JSON invàlid per importar clau de dispositiu')
  }

  if (
    parsed.version !== 1 ||
    parsed.kemAlgorithm !== 'ML-KEM-1024' ||
    parsed.dsaAlgorithm !== 'ML-DSA-87' ||
    !parsed.deviceId ||
    !parsed.kemPublicKey ||
    !parsed.kemSecretKey ||
    !parsed.dsaPublicKey ||
    !parsed.dsaSecretKey
  ) {
    throw new Error('El fitxer importat no té el format de backup esperat')
  }

  const publicKey = base64ToUint8Array(parsed.kemPublicKey)
  const secretKey = base64ToUint8Array(parsed.kemSecretKey)
  const dsaPublicKey = base64ToUint8Array(parsed.dsaPublicKey)
  const dsaSecretKey = base64ToUint8Array(parsed.dsaSecretKey)
  const safeDeviceId = parsed.deviceId.trim()

  const existing = await listNamedKeypairs()
  const duplicated = existing.some((item) => normalizeDeviceId(item.name) === normalizeDeviceId(safeDeviceId))
  if (duplicated && !overwrite) {
    throw new KeypairDeviceIdExistsError(safeDeviceId)
  }

  await upsertNamedKeypair(safeDeviceId, safeDeviceId, secretKey, publicKey, dsaSecretKey, dsaPublicKey)
  await storeDeviceSecretKeys(safeDeviceId, secretKey, dsaSecretKey)
  await storeDevicePublicKey(safeDeviceId, publicKey, dsaPublicKey)

  return {
    ...parsed,
    deviceId: safeDeviceId,
  }
}

export async function exportDeviceKeypair(deviceId: string): Promise<string> {
  const found = await getNamedKeypair(deviceId)
  if (!found) {
    throw new Error('No existeix cap keypair amb aquest deviceId')
  }

  const resolvedDeviceId = found.summary.deviceId ?? found.summary.name

  const secretKey = found.secretKey
  const publicKey = found.publicKey

  const bundle: DeviceKeypairBundle = {
    version: 1,
    kemAlgorithm: 'ML-KEM-1024',
    dsaAlgorithm: 'ML-DSA-87',
    deviceId: resolvedDeviceId,
    createdAt: Date.now(),
    kemPublicKey: uint8ArrayToBase64(publicKey),
    kemSecretKey: uint8ArrayToBase64(secretKey),
    dsaPublicKey: uint8ArrayToBase64(found.dsaPublicKey ?? new Uint8Array()),
    dsaSecretKey: uint8ArrayToBase64(found.dsaSecretKey ?? new Uint8Array()),
  }

  return JSON.stringify(bundle, null, 2)
}

export async function deleteDeviceKeypair(deviceId: string): Promise<void> {
  await deleteNamedKeypair(deviceId)
  await deleteDeviceSecretKeys(deviceId)
}

export async function listDeviceKeypairs(): Promise<NamedKeypairItem[]> {
  const items = await listNamedKeypairs()
  return items.map((item) => ({
    deviceId: item.deviceId ?? item.name,
    createdAt: item.createdAt,
    updatedAt: item.updatedAt,
  }))
}

export async function getDeviceKeySummary(deviceId: string): Promise<{
  hasKeypair: boolean
  kemPublicKeyPreview: string | null
  dsaPublicKeyPreview: string | null
  hasSigningKeypair: boolean
}> {
  const secretKeys = await getDeviceSecretKeys(deviceId)
  const publicKeys = await getDevicePublicKeys(deviceId)

  if (!secretKeys?.kemSecretKey || !publicKeys?.kemPublicKey) {
    return { hasKeypair: false, kemPublicKeyPreview: null, dsaPublicKeyPreview: null, hasSigningKeypair: false }
  }

  const kemEncoded = uint8ArrayToBase64(publicKeys.kemPublicKey)
  const dsaEncoded = publicKeys.dsaPublicKey ? uint8ArrayToBase64(publicKeys.dsaPublicKey) : null
  return {
    hasKeypair: true,
    kemPublicKeyPreview: `${kemEncoded.slice(0, 16)}...${kemEncoded.slice(-12)}`,
    dsaPublicKeyPreview: dsaEncoded ? `${dsaEncoded.slice(0, 16)}...${dsaEncoded.slice(-12)}` : null,
    hasSigningKeypair: !!secretKeys.dsaSecretKey,
  }
}

async function getSymmetricChannelKeyCount(): Promise<number> {
  const keys = await listChannelKeys()
  return keys.filter((k) => k.type === 'symmetric').length
}

export async function listSymmetricChannelKeys(): Promise<
  Array<{
    channelId: string
    keyVersion: number
    acquiredAt: number
    preview: string
  }>
> {
  const keys = await getAllChannelKeys()
  return keys
    .filter((entry) => entry.type === 'symmetric')
    .map((entry) => {
      const encoded = uint8ArrayToBase64(entry.keyBytes)
      return {
        channelId: entry.channelId,
        keyVersion: entry.keyVersion,
        acquiredAt: entry.acquiredAt,
        preview: `${encoded.slice(0, 10)}...${encoded.slice(-8)}`,
      }
    })
    .sort((a, b) => b.acquiredAt - a.acquiredAt)
}

export async function deleteSymmetricChannelKey(channelId: string): Promise<void> {
  await deleteChannelKey(channelId)
}

export async function exportSymmetricChannelKeys(): Promise<string> {
  const keys = await getAllChannelKeys()
  const channels = keys
    .filter((entry) => entry.type === 'symmetric')
    .map((entry) => ({
      channelId: entry.channelId,
      keyVersion: entry.keyVersion,
      key: uint8ArrayToBase64(entry.keyBytes),
      acquiredAt: entry.acquiredAt,
    }))

  const bundle: SymmetricChannelsBundle = {
    version: 1,
    exportedAt: Date.now(),
    channels,
  }

  return JSON.stringify(bundle, null, 2)
}

export async function exportAsymmetricChannelKeys(): Promise<string> {
  const keys = await getAllChannelKeys()
  const channels = keys
    .filter((entry) => entry.type === 'asymmetric')
    .map((entry) => ({
      channelId: entry.channelId,
      keyVersion: entry.keyVersion,
      keyVersionId: entry.keyVersionId ?? null,
      key: uint8ArrayToBase64(entry.keyBytes),
      acquiredAt: entry.acquiredAt,
    }))

  const bundle: AsymmetricChannelsBundle = {
    version: 1,
    exportedAt: Date.now(),
    channels,
  }

  return JSON.stringify(bundle, null, 2)
}

export async function importSymmetricChannelKeys(bundleText: string): Promise<number> {
  let parsed: SymmetricChannelsBundle
  try {
    parsed = JSON.parse(bundleText) as SymmetricChannelsBundle
  } catch {
    throw new Error('Format JSON invàlid per importar claus simètriques')
  }

  if (parsed.version !== 1 || !Array.isArray(parsed.channels)) {
    throw new Error('El fitxer de claus simètriques no és compatible')
  }

  let imported = 0
  for (const item of parsed.channels) {
    if (!item.channelId || !item.key) {
      continue
    }
    const keyBytes = base64ToUint8Array(item.key)
    await storeChannelKey(item.channelId, keyBytes, 'symmetric', item.keyVersion ?? 1)
    imported++
  }

  return imported
}

export async function importAsymmetricChannelKeys(bundleText: string): Promise<number> {
  let parsed: AsymmetricChannelsBundle
  try {
    parsed = JSON.parse(bundleText) as AsymmetricChannelsBundle
  } catch {
    throw new Error('Format JSON invàlid per importar claus asimètriques')
  }

  if (parsed.version !== 1 || !Array.isArray(parsed.channels)) {
    throw new Error('El fitxer de claus asimètriques no és compatible')
  }

  let imported = 0
  for (const item of parsed.channels) {
    if (!item.channelId || !item.key) {
      continue
    }
    const keyBytes = base64ToUint8Array(item.key)
    await storeChannelKey(item.channelId, keyBytes, 'asymmetric', item.keyVersion ?? 1, item.keyVersionId ?? null)
    imported++
  }

  return imported
}

async function getChannelKeyPreview(channelId: string): Promise<string | null> {
  const key = await getChannelKey(channelId)
  if (!key) {
    return null
  }

  const encoded = uint8ArrayToBase64(key)
  return `${encoded.slice(0, 10)}...${encoded.slice(-8)}`
}

// ── Full Backup ───────────────────────────────────────────────

export interface FullBackupBundle {
  version: 1
  exportedAt: number
  deviceKeypairs: DeviceKeypairBundle[]
  symmetricChannels: SymmetricChannelsBundle['channels']
  asymmetricChannels: AsymmetricChannelsBundle['channels']
}

/**
 * Exporta totes les claus locals (keypairs de dispositiu + claus de canal) en un únic JSON.
 */
export async function exportFullBackup(): Promise<string> {
  const keypairItems = await listDeviceKeypairs()
  const deviceKeypairs: DeviceKeypairBundle[] = []

  for (const item of keypairItems) {
    try {
      const json = await exportDeviceKeypair(item.deviceId)
      deviceKeypairs.push(JSON.parse(json) as DeviceKeypairBundle)
    } catch {
      // ignorar keypairs que no es poden exportar
    }
  }

  const allChannelKeys = await getAllChannelKeys()

  const symmetricChannels = allChannelKeys
    .filter((entry) => entry.type === 'symmetric')
    .map((entry) => ({
      channelId: entry.channelId,
      keyVersion: entry.keyVersion,
      key: uint8ArrayToBase64(entry.keyBytes),
      acquiredAt: entry.acquiredAt,
    }))

  const asymmetricChannels = allChannelKeys
    .filter((entry) => entry.type === 'asymmetric')
    .map((entry) => ({
      channelId: entry.channelId,
      keyVersion: entry.keyVersion,
      keyVersionId: entry.keyVersionId ?? null,
      key: uint8ArrayToBase64(entry.keyBytes),
      acquiredAt: entry.acquiredAt,
    }))

  const bundle: FullBackupBundle = {
    version: 1,
    exportedAt: Date.now(),
    deviceKeypairs,
    symmetricChannels,
    asymmetricChannels,
  }

  return JSON.stringify(bundle, null, 2)
}

/**
 * Importa un backup complet generat per exportFullBackup.
 * Retorna el nombre d'elements importats de cada categoria.
 */
export async function importFullBackup(bundleText: string): Promise<{
  devices: number
  symmetricChannels: number
  asymmetricChannels: number
}> {
  let parsed: FullBackupBundle
  try {
    parsed = JSON.parse(bundleText) as FullBackupBundle
  } catch {
    throw new Error('Format JSON invàlid per importar el backup')
  }

  if (parsed.version !== 1 || !Array.isArray(parsed.deviceKeypairs)) {
    throw new Error('El fitxer de backup no és compatible (versió incorrecta)')
  }

  let devices = 0
  for (const kp of parsed.deviceKeypairs) {
    try {
      await importAndStoreDeviceKeypair(JSON.stringify(kp), true)
      devices++
    } catch {
      // ignorar keypairs invàlids
    }
  }

  let symmetricChannels = 0
  if (Array.isArray(parsed.symmetricChannels)) {
    const symBundle: SymmetricChannelsBundle = {
      version: 1,
      exportedAt: parsed.exportedAt,
      channels: parsed.symmetricChannels,
    }
    symmetricChannels = await importSymmetricChannelKeys(JSON.stringify(symBundle))
  }

  let asymmetricChannels = 0
  if (Array.isArray(parsed.asymmetricChannels)) {
    const asymBundle: AsymmetricChannelsBundle = {
      version: 1,
      exportedAt: parsed.exportedAt,
      channels: parsed.asymmetricChannels,
    }
    asymmetricChannels = await importAsymmetricChannelKeys(JSON.stringify(asymBundle))
  }

  return { devices, symmetricChannels, asymmetricChannels }
}
