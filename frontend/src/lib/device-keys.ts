import { ml_kem1024 } from '@noble/post-quantum/ml-kem.js'

import {
  deleteChannelKey,
  deleteNamedKeypair,
  getAllChannelKeys,
  getChannelKey,
  getNamedKeypair,
  getDevicePublicKey,
  getKeypair,
  listNamedKeypairs,
  listChannelKeys,
  storeChannelKey,
  storeDevicePublicKey,
  storeKeypair,
  upsertNamedKeypair,
} from './storage'

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
  algorithm: 'ML-KEM-1024'
  deviceId: string
  createdAt: number
  publicKey: string
  secretKey: string
}

export interface SymmetricChannelsBundle {
  version: 1
  exportedAt: number
  channels: Array<{
    channelId: string
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
  const secretKey = await getKeypair(deviceId)
  if (secretKey) {
    return true
  }

  const named = await listNamedKeypairs()
  if (named.some((item) => item.deviceId === deviceId)) {
    return true
  }

  return !!secretKey
}

export async function generateAndStoreDeviceKeypair(
  deviceId: string,
  overwrite = false
): Promise<{ publicKey: string }> {
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
  await upsertNamedKeypair(safeDeviceId, safeDeviceId, secretKey, publicKey)
  await storeKeypair(safeDeviceId, secretKey)
  await storeDevicePublicKey(safeDeviceId, publicKey)

  return { publicKey: uint8ArrayToBase64(publicKey) }
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

  if (parsed.version !== 1 || parsed.algorithm !== 'ML-KEM-1024' || !parsed.deviceId || !parsed.publicKey || !parsed.secretKey) {
    throw new Error('El fitxer importat no té el format de backup esperat')
  }

  const publicKey = base64ToUint8Array(parsed.publicKey)
  const secretKey = base64ToUint8Array(parsed.secretKey)
  const safeDeviceId = parsed.deviceId.trim()

  const existing = await listNamedKeypairs()
  const duplicated = existing.some((item) => normalizeDeviceId(item.name) === normalizeDeviceId(safeDeviceId))
  if (duplicated && !overwrite) {
    throw new KeypairDeviceIdExistsError(safeDeviceId)
  }

  await upsertNamedKeypair(safeDeviceId, safeDeviceId, secretKey, publicKey)

  await storeKeypair(safeDeviceId, secretKey)
  await storeDevicePublicKey(safeDeviceId, publicKey)

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
    algorithm: 'ML-KEM-1024',
    deviceId: resolvedDeviceId,
    createdAt: Date.now(),
    publicKey: uint8ArrayToBase64(publicKey),
    secretKey: uint8ArrayToBase64(secretKey),
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
  publicKeyPreview: string | null
}> {
  const secretKey = await getKeypair(deviceId)
  const publicKey = await getDevicePublicKey(deviceId)

  if (!secretKey || !publicKey) {
    return { hasKeypair: false, publicKeyPreview: null }
  }

  const encoded = uint8ArrayToBase64(publicKey)
  return {
    hasKeypair: true,
    publicKeyPreview: `${encoded.slice(0, 16)}...${encoded.slice(-12)}`,
  }
}

export async function getSymmetricChannelKeyCount(): Promise<number> {
  const keys = await listChannelKeys()
  return keys.filter((k) => k.type === 'symmetric').length
}

export async function listSymmetricChannelKeys(): Promise<
  Array<{
    channelId: string
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
    await storeChannelKey(item.channelId, keyBytes, 'symmetric')
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
