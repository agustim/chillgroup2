import { ml_kem1024 } from '@noble/post-quantum/ml-kem.js'
import { ml_dsa87 } from '@noble/post-quantum/ml-dsa.js'

import {
  deleteChannelKey,
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

export async function getSymmetricChannelKeyCount(): Promise<number> {
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

export async function getChannelKeyPreview(channelId: string): Promise<string | null> {
  const key = await getChannelKey(channelId)
  if (!key) {
    return null
  }

  const encoded = uint8ArrayToBase64(key)
  return `${encoded.slice(0, 10)}...${encoded.slice(-8)}`
}
