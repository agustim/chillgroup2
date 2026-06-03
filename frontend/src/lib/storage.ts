//! IndexedDB wrapper per a ChillGroup v2.
//!
//! Gestiona l'emmagatzematge local de claus criptogràfiques i dades de sessió.

import {
  decryptBytesFromLocalVault,
  encryptBytesForLocalVault,
  hasLocalVault,
  isLocalVaultUnlocked,
} from './local-vault'

const DB_NAME = 'chillgroup-store'
const DB_VERSION = 3

/**
 * Obtenir la base de dades IndexedDB.
 */
function getDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION)

    request.onerror = () => reject(request.error)
    request.onsuccess = () => resolve(request.result)

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result

      // Object store per a keypairs del dispositiu
      if (!db.objectStoreNames.contains('keypairs')) {
        db.createObjectStore('keypairs', { keyPath: 'deviceId' })
      }

      // Object store V2 per a keypairs amb nom lògic (múltiples claus)
      if (!db.objectStoreNames.contains('keypairsV2')) {
        const store = db.createObjectStore('keypairsV2', { keyPath: 'nameNormalized' })
        store.createIndex('name', 'name', { unique: true })
        store.createIndex('deviceId', 'deviceId', { unique: false })
      }

      // Object store per a claus de canal (bytes)
      if (!db.objectStoreNames.contains('channelKeysBytes')) {
        const store = db.createObjectStore('channelKeysBytes', { keyPath: 'channelId' })
        store.createIndex('type', 'type', { unique: false })
        store.createIndex('expiresAt', 'expiresAt', { unique: false })
      }

      // Object store V3 per a claus de canal versionades
      if (!db.objectStoreNames.contains('channelKeysByVersion')) {
        const store = db.createObjectStore('channelKeysByVersion', { keyPath: 'compoundId' })
        store.createIndex('channelId', 'channelId', { unique: false })
        store.createIndex('channelAndVersion', ['channelId', 'keyVersion'], { unique: true })
        store.createIndex('type', 'type', { unique: false })
        store.createIndex('expiresAt', 'expiresAt', { unique: false })
      }

      // Object store per a claus de canal (CryptoKey serialitzat)
      if (!db.objectStoreNames.contains('channelKeys')) {
        db.createObjectStore('channelKeys', { keyPath: 'channelId' })
      }

      // Object store per a claus públiques del dispositiu
      if (!db.objectStoreNames.contains('devicePublicKeys')) {
        db.createObjectStore('devicePublicKeys', { keyPath: 'deviceId' })
      }

      // Object store per a claus de sessió LiveKit
      if (!db.objectStoreNames.contains('livekitSessionKeys')) {
        db.createObjectStore('livekitSessionKeys', { keyPath: 'sessionId' })
      }
    }
  })
}

/**
 * Convertir Uint8Array a base64 per a emmagatzematge.
 */
function uint8ArrayToBase64(data: Uint8Array): string {
  const bytes = new Uint8Array(data.buffer as ArrayBuffer)
  return btoa(String.fromCharCode(...bytes))
}

/**
 * Convertir base64 a Uint8Array.
 */
function base64ToUint8Array(data: string): Uint8Array {
  const binary = atob(data)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes
}

// ── KeyPairs (Dispositiu) ─────────────────────────────────────

export interface NamedKeypairRecord {
  name: string
  nameNormalized: string
  deviceId: string | null
  kyberSecretKey: string
  kyberPublicKey: string
  dsaSecretKey?: string
  dsaPublicKey?: string
  createdAt: number
  updatedAt: number
}

export interface NamedKeypairSummary {
  name: string
  deviceId: string | null
  createdAt: number
  updatedAt: number
}

function normalizeKeypairName(name: string): string {
  return name.trim().toLowerCase()
}

/**
 * Guardar el keypair del dispositiu a IndexedDB.
 */
export async function storeKeypair(
  deviceId: string,
  secretKey: Uint8Array,
  dsaSecretKey?: Uint8Array
): Promise<void> {
  return storeDeviceSecretKeys(deviceId, secretKey, dsaSecretKey)
}

export async function storeDeviceSecretKeys(
  deviceId: string,
  kemSecretKey: Uint8Array,
  dsaSecretKey?: Uint8Array
): Promise<void> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('keypairs', 'readwrite')
    const store = tx.objectStore('keypairs')
    store.put({
      deviceId,
      kyberSecretKey: uint8ArrayToBase64(kemSecretKey),
      dsaSecretKey: dsaSecretKey ? uint8ArrayToBase64(dsaSecretKey) : undefined,
      createdAt: Date.now(),
    })
    tx.oncomplete = () => {
      db.close()
      resolve()
    }
    tx.onerror = () => {
      db.close()
      reject(tx.error)
    }
  })
}

/**
 * Obtenir el keypair del dispositiu des de IndexedDB.
 */
export async function getKeypair(deviceId: string): Promise<Uint8Array | null> {
  const keypair = await getDeviceSecretKeys(deviceId)
  return keypair?.kemSecretKey ?? null
}

export async function getDeviceSecretKeys(deviceId: string): Promise<{
  kemSecretKey: Uint8Array
  dsaSecretKey: Uint8Array | null
} | null> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('keypairs', 'readonly')
    const store = tx.objectStore('keypairs')
    const request = store.get(deviceId)
    request.onsuccess = () => {
      const result = request.result
      if (!result) {
        db.close()
        resolve(null)
        return
      }
      const keyBytes = base64ToUint8Array(result.kyberSecretKey)
      const signingKeyBytes = typeof result.dsaSecretKey === 'string'
        ? base64ToUint8Array(result.dsaSecretKey)
        : null
      db.close()
      resolve({
        kemSecretKey: keyBytes,
        dsaSecretKey: signingKeyBytes,
      })
    }
    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

/**
 * Eliminar el keypair del dispositiu.
 */
export async function deleteKeypair(deviceId: string): Promise<void> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('keypairs', 'readwrite')
    const store = tx.objectStore('keypairs')
    store.delete(deviceId)
    tx.oncomplete = () => {
      db.close()
      resolve()
    }
    tx.onerror = () => {
      db.close()
      reject(tx.error)
    }
  })
}

/**
 * Llistar tots els keypairs emmagatzemats.
 */
export async function listKeypairs(): Promise<Array<{ deviceId: string; createdAt: number }>> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('keypairs', 'readonly')
    const store = tx.objectStore('keypairs')
    const request = store.getAll()
    request.onsuccess = () => {
      const result = request.result.map((item: any) => ({
        deviceId: item.deviceId,
        createdAt: item.createdAt,
      }))
      db.close()
      resolve(result)
    }
    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

/**
 * Guardar (o sobreescriure) un keypair nominal al store V2.
 */
export async function upsertNamedKeypair(
  name: string,
  deviceId: string | null,
  secretKey: Uint8Array,
  publicKey: Uint8Array,
  dsaSecretKey?: Uint8Array,
  dsaPublicKey?: Uint8Array
): Promise<void> {
  const normalized = normalizeKeypairName(name)
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('keypairsV2', 'readwrite')
    const store = tx.objectStore('keypairsV2')
    const now = Date.now()

    const getReq = store.get(normalized)
    getReq.onsuccess = () => {
      const prev = getReq.result as NamedKeypairRecord | undefined
      store.put({
        name: name.trim(),
        nameNormalized: normalized,
        deviceId,
        kyberSecretKey: uint8ArrayToBase64(secretKey),
        kyberPublicKey: uint8ArrayToBase64(publicKey),
        dsaSecretKey: dsaSecretKey ? uint8ArrayToBase64(dsaSecretKey) : prev?.dsaSecretKey,
        dsaPublicKey: dsaPublicKey ? uint8ArrayToBase64(dsaPublicKey) : prev?.dsaPublicKey,
        createdAt: prev?.createdAt ?? now,
        updatedAt: now,
      })
    }
    getReq.onerror = () => {
      db.close()
      reject(getReq.error)
    }

    tx.oncomplete = () => {
      db.close()
      resolve()
    }
    tx.onerror = () => {
      db.close()
      reject(tx.error)
    }
  })
}

/**
 * Obtenir un keypair nominal per nom.
 */
export async function getNamedKeypair(name: string): Promise<{
  summary: NamedKeypairSummary
  secretKey: Uint8Array
  publicKey: Uint8Array
  dsaSecretKey: Uint8Array | null
  dsaPublicKey: Uint8Array | null
} | null> {
  const normalized = normalizeKeypairName(name)
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('keypairsV2', 'readonly')
    const store = tx.objectStore('keypairsV2')
    const request = store.get(normalized)
    request.onsuccess = () => {
      const result = request.result as NamedKeypairRecord | undefined
      if (!result) {
        db.close()
        resolve(null)
        return
      }

      db.close()
      resolve({
        summary: {
          name: result.name,
          deviceId: result.deviceId,
          createdAt: result.createdAt,
          updatedAt: result.updatedAt,
        },
        secretKey: base64ToUint8Array(result.kyberSecretKey),
        publicKey: base64ToUint8Array(result.kyberPublicKey),
        dsaSecretKey: result.dsaSecretKey ? base64ToUint8Array(result.dsaSecretKey) : null,
        dsaPublicKey: result.dsaPublicKey ? base64ToUint8Array(result.dsaPublicKey) : null,
      })
    }
    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

/**
 * Llistar keypairs nominals (store V2).
 */
export async function listNamedKeypairs(): Promise<NamedKeypairSummary[]> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('keypairsV2', 'readonly')
    const store = tx.objectStore('keypairsV2')
    const request = store.getAll()
    request.onsuccess = () => {
      const records = (request.result as NamedKeypairRecord[])
        .map((item) => ({
          name: item.name,
          deviceId: item.deviceId,
          createdAt: item.createdAt,
          updatedAt: item.updatedAt,
        }))
        .sort((a, b) => b.updatedAt - a.updatedAt)
      db.close()
      resolve(records)
    }
    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

/**
 * Eliminar un keypair nominal.
 */
export async function deleteNamedKeypair(name: string): Promise<void> {
  const normalized = normalizeKeypairName(name)
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('keypairsV2', 'readwrite')
    const store = tx.objectStore('keypairsV2')
    store.delete(normalized)
    tx.oncomplete = () => {
      db.close()
      resolve()
    }
    tx.onerror = () => {
      db.close()
      reject(tx.error)
    }
  })
}

// ── Channel Keys (Bytes) ──────────────────────────────────────

function normalizeKeyVersion(keyVersion: number | undefined): number {
  return Number.isInteger(keyVersion) && (keyVersion as number) > 0 ? (keyVersion as number) : 1
}

function makeChannelVersionId(channelId: string, keyVersion: number): string {
  return `${channelId}::${keyVersion}`
}

async function encodeChannelKeyForStorage(keyBytes: Uint8Array): Promise<{
  keyBytes: string | null
  keyCiphertext: string | null
}> {
  if (hasLocalVault()) {
    if (!isLocalVaultUnlocked()) {
      throw new Error('Vault local bloquejat')
    }
    const encrypted = await encryptBytesForLocalVault(keyBytes)
    return {
      keyBytes: null,
      keyCiphertext: encrypted,
    }
  }

  return {
    keyBytes: uint8ArrayToBase64(keyBytes),
    keyCiphertext: null,
  }
}

async function decodeChannelKeyFromStorage(record: {
  keyBytes?: string | null
  keyCiphertext?: string | null
}): Promise<Uint8Array> {
  if (typeof record.keyCiphertext === 'string' && record.keyCiphertext.length > 0) {
    return decryptBytesFromLocalVault(record.keyCiphertext)
  }

  if (typeof record.keyBytes === 'string' && record.keyBytes.length > 0) {
    return base64ToUint8Array(record.keyBytes)
  }

  throw new Error('Format de clau de canal invàlid')
}

/**
 * Guardar una clau de canal versionada.
 */
async function storeChannelKeyVersion(
  channelId: string,
  keyVersion: number,
  keyBytes: Uint8Array,
  type: 'symmetric' | 'asymmetric',
  keyVersionId?: string | null
): Promise<void> {
  const normalizedVersion = normalizeKeyVersion(keyVersion)
  const encoded = await encodeChannelKeyForStorage(keyBytes)
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('channelKeysByVersion', 'readwrite')
    const store = tx.objectStore('channelKeysByVersion')
    store.put({
      compoundId: makeChannelVersionId(channelId, normalizedVersion),
      channelId,
      keyVersion: normalizedVersion,
      keyVersionId: keyVersionId ?? null,
      keyBytes: encoded.keyBytes,
      keyCiphertext: encoded.keyCiphertext,
      type,
      acquiredAt: Date.now(),
      expiresAt: null,
    })
    tx.oncomplete = () => {
      db.close()
      resolve()
    }
    tx.onerror = () => {
      db.close()
      reject(tx.error)
    }
  })
}

/**
 * Obtenir una clau de canal per versió exacta.
 */
export async function getChannelKeyVersion(channelId: string, keyVersion: number): Promise<Uint8Array | null> {
  const normalizedVersion = normalizeKeyVersion(keyVersion)
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('channelKeysByVersion', 'readonly')
    const store = tx.objectStore('channelKeysByVersion')
    const request = store.get(makeChannelVersionId(channelId, normalizedVersion))
    request.onsuccess = () => {
      const result = request.result
      if (!result) {
        db.close()
        resolve(null)
        return
      }
      void decodeChannelKeyFromStorage(result)
        .then((decoded) => {
          db.close()
          resolve(decoded)
        })
        .catch((err) => {
          db.close()
          reject(err)
        })
    }
    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

/**
 * Obtenir la darrera clau disponible d'un canal (major keyVersion).
 */
export async function getLatestChannelKey(channelId: string): Promise<{
  keyBytes: Uint8Array
  keyVersion: number
  keyVersionId?: string | null
} | null> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('channelKeysByVersion', 'readonly')
    const store = tx.objectStore('channelKeysByVersion')
    const index = store.index('channelId')
    const request = index.getAll()

    request.onsuccess = () => {
      const records = (request.result as Array<{ channelId: string; keyBytes?: string | null; keyCiphertext?: string | null; keyVersion: number; keyVersionId?: string | null }>)
        .filter((item) => item.channelId === channelId)
      if (!records || records.length === 0) {
        db.close()
        resolve(null)
        return
      }

      const latest = records.reduce((acc, cur) =>
        normalizeKeyVersion(cur.keyVersion) > normalizeKeyVersion(acc.keyVersion) ? cur : acc
      )

      void decodeChannelKeyFromStorage(latest)
        .then((decoded) => {
          db.close()
          resolve({
            keyBytes: decoded,
            keyVersion: normalizeKeyVersion(latest.keyVersion),
            keyVersionId: latest.keyVersionId ?? null,
          })
        })
        .catch((err) => {
          db.close()
          reject(err)
        })
    }

    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

/**
 * Llistar versions disponibles d'un canal.
 */
async function listChannelKeyVersions(channelId: string): Promise<number[]> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('channelKeysByVersion', 'readonly')
    const store = tx.objectStore('channelKeysByVersion')
    const index = store.index('channelId')
    const request = index.getAll()

    request.onsuccess = () => {
      const versions = Array.from(
        new Set(
          (request.result as Array<{ channelId: string; keyVersion?: number }>)
            .filter((r) => r.channelId === channelId)
            .map((r) => normalizeKeyVersion(r.keyVersion))
        )
      ).sort((a, b) => a - b)
      db.close()
      resolve(versions)
    }

    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

/**
 * Guardar una clau de canal (bytes) a IndexedDB.
 */
export async function storeChannelKey(
  channelId: string,
  keyBytes: Uint8Array,
  type: 'symmetric' | 'asymmetric',
  keyVersion = 1,
  keyVersionId?: string | null
): Promise<void> {
  await storeChannelKeyVersion(channelId, keyVersion, keyBytes, type, keyVersionId)
  const encoded = await encodeChannelKeyForStorage(keyBytes)

  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('channelKeysBytes', 'readwrite')
    const store = tx.objectStore('channelKeysBytes')
    store.put({
      channelId,
      keyBytes: encoded.keyBytes,
      keyCiphertext: encoded.keyCiphertext,
      type,
      acquiredAt: Date.now(),
      expiresAt: null,
    })
    tx.oncomplete = () => {
      db.close()
      resolve()
    }
    tx.onerror = () => {
      db.close()
      reject(tx.error)
    }
  })
}

/**
 * Obtenir una clau de canal des de IndexedDB.
 */
export async function getChannelKey(channelId: string): Promise<Uint8Array | null> {
  const latest = await getLatestChannelKey(channelId)
  if (latest) {
    return latest.keyBytes
  }

  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('channelKeysBytes', 'readonly')
    const store = tx.objectStore('channelKeysBytes')
    const request = store.get(channelId)
    request.onsuccess = () => {
      const result = request.result
      if (!result) {
        db.close()
        resolve(null)
        return
      }
      void decodeChannelKeyFromStorage(result)
        .then((decoded) => {
          db.close()
          resolve(decoded)
        })
        .catch((err) => {
          db.close()
          reject(err)
        })
    }
    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

export interface StoredChannelKeyRecord {
  channelId: string
  keyBytes: Uint8Array
  keyVersion: number
  keyVersionId?: string | null
  type: 'symmetric' | 'asymmetric'
  acquiredAt: number
  expiresAt: number | null
}

/**
 * Llistar metadata de totes les claus de canal guardades.
 */
export async function listChannelKeys(): Promise<
  Array<{
    channelId: string
    keyVersion: number
    keyVersionId: string | null
    type: 'symmetric' | 'asymmetric'
    acquiredAt: number
    expiresAt: number | null
  }>
> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('channelKeysByVersion', 'readonly')
    const store = tx.objectStore('channelKeysByVersion')
    const request = store.getAll()
    request.onsuccess = () => {
      const items = request.result.map((item: any) => ({
        channelId: item.channelId,
        keyVersion: normalizeKeyVersion(item.keyVersion),
        keyVersionId: item.keyVersionId ?? null,
        type: item.type as 'symmetric' | 'asymmetric',
        acquiredAt: item.acquiredAt,
        expiresAt: item.expiresAt ?? null,
      }))
      db.close()
      resolve(items)
    }
    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

/**
 * Llistar totes les claus de canal (inclou bytes) per exportació/importació.
 */
export async function getAllChannelKeys(): Promise<StoredChannelKeyRecord[]> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('channelKeysByVersion', 'readonly')
    const store = tx.objectStore('channelKeysByVersion')
    const request = store.getAll()
    request.onsuccess = () => {
      const rawItems = request.result as Array<any>
      void Promise.all(
        rawItems.map(async (item) => ({
          channelId: item.channelId,
          keyVersion: normalizeKeyVersion(item.keyVersion),
          keyVersionId: item.keyVersionId ?? null,
          keyBytes: await decodeChannelKeyFromStorage(item),
          type: item.type as 'symmetric' | 'asymmetric',
          acquiredAt: item.acquiredAt,
          expiresAt: item.expiresAt ?? null,
        }))
      )
        .then((items) => {
          db.close()
          resolve(items)
        })
        .catch((err) => {
          db.close()
          reject(err)
        })
    }
    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

/**
 * Eliminar una clau de canal.
 */
export async function deleteChannelKey(channelId: string): Promise<void> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(['channelKeysBytes', 'channelKeysByVersion'], 'readwrite')
    const legacyStore = tx.objectStore('channelKeysBytes')
    const versionedStore = tx.objectStore('channelKeysByVersion')
    const index = versionedStore.index('channelId')

    legacyStore.delete(channelId)

    const cursorRequest = index.openCursor()
    cursorRequest.onsuccess = () => {
      const cursor = cursorRequest.result
      if (cursor) {
        if ((cursor.value as { channelId?: string }).channelId === channelId) {
          versionedStore.delete(cursor.primaryKey)
        }
        cursor.continue()
      }
    }
    tx.oncomplete = () => {
      db.close()
      resolve()
    }
    tx.onerror = () => {
      db.close()
      reject(tx.error)
    }
  })
}

/**
 * Netejar claus expirades.
 */
async function cleanupExpiredKeys(): Promise<number> {
  const db = await getDB()
  const now = Date.now()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('channelKeysByVersion', 'readwrite')
    const store = tx.objectStore('channelKeysByVersion')
    const request = store.getAll()
    request.onsuccess = () => {
      const items = request.result
      let count = 0
      for (const item of items) {
        if (item.expiresAt && item.expiresAt < now) {
          store.delete(item.compoundId)
          count++
        }
      }
      db.close()
      resolve(count)
    }
    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

/**
 * Re-xifrar claus de canal antigues guardades en clar quan el vault local ja està actiu.
 */
export async function migrateChannelKeysToLocalVault(): Promise<number> {
  if (!hasLocalVault() || !isLocalVaultUnlocked()) {
    return 0
  }

  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(['channelKeysByVersion', 'channelKeysBytes'], 'readwrite')
    const versionedStore = tx.objectStore('channelKeysByVersion')
    const legacyStore = tx.objectStore('channelKeysBytes')
    let migrated = 0

    const versionedRequest = versionedStore.getAll()
    versionedRequest.onsuccess = () => {
      const rows = versionedRequest.result as Array<any>
      void Promise.all(
        rows.map(async (row) => {
          if (row.keyCiphertext || typeof row.keyBytes !== 'string' || row.keyBytes.length === 0) {
            return
          }
          const encrypted = await encryptBytesForLocalVault(base64ToUint8Array(row.keyBytes))
          versionedStore.put({
            ...row,
            keyBytes: null,
            keyCiphertext: encrypted,
          })
          migrated++
        })
      ).catch(() => {
        // tx.onerror gestiona el rebuig final
      })
    }

    const legacyRequest = legacyStore.getAll()
    legacyRequest.onsuccess = () => {
      const rows = legacyRequest.result as Array<any>
      void Promise.all(
        rows.map(async (row) => {
          if (row.keyCiphertext || typeof row.keyBytes !== 'string' || row.keyBytes.length === 0) {
            return
          }
          const encrypted = await encryptBytesForLocalVault(base64ToUint8Array(row.keyBytes))
          legacyStore.put({
            ...row,
            keyBytes: null,
            keyCiphertext: encrypted,
          })
          migrated++
        })
      ).catch(() => {
        // tx.onerror gestiona el rebuig final
      })
    }

    tx.oncomplete = () => {
      db.close()
      resolve(migrated)
    }
    tx.onerror = () => {
      db.close()
      reject(tx.error)
    }
  })
}

// ── Device Public Keys ────────────────────────────────────────

/**
 * Guardar la clau pública del dispositiu.
 */
export async function storeDevicePublicKey(
  deviceId: string,
  publicKey: Uint8Array,
  dsaPublicKey?: Uint8Array
): Promise<void> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('devicePublicKeys', 'readwrite')
    const store = tx.objectStore('devicePublicKeys')
    store.put({
      deviceId,
      publicKey: uint8ArrayToBase64(publicKey),
      kemPublicKey: uint8ArrayToBase64(publicKey),
      dsaPublicKey: dsaPublicKey ? uint8ArrayToBase64(dsaPublicKey) : undefined,
      algorithm: 'Kyber-1024',
    })
    tx.oncomplete = () => {
      db.close()
      resolve()
    }
    tx.onerror = () => {
      db.close()
      reject(tx.error)
    }
  })
}

/**
 * Obtenir la clau pública del dispositiu.
 */
export async function getDevicePublicKey(deviceId: string): Promise<Uint8Array | null> {
  const result = await getDevicePublicKeys(deviceId)
  return result?.kemPublicKey ?? null
}

export async function getDevicePublicKeys(deviceId: string): Promise<{
  kemPublicKey: Uint8Array
  dsaPublicKey: Uint8Array | null
} | null> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('devicePublicKeys', 'readonly')
    const store = tx.objectStore('devicePublicKeys')
    const request = store.get(deviceId)
    request.onsuccess = () => {
      const result = request.result
      if (!result) {
        db.close()
        resolve(null)
        return
      }
      const kemPublicKey = base64ToUint8Array(result.kemPublicKey ?? result.publicKey)
      const signingPublicKey = typeof result.dsaPublicKey === 'string'
        ? base64ToUint8Array(result.dsaPublicKey)
        : null
      db.close()
      resolve({
        kemPublicKey,
        dsaPublicKey: signingPublicKey,
      })
    }
    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

// ── LiveKit Session Keys ──────────────────────────────────────

/**
 * Guardar una clau de sessió LiveKit.
 */
export async function storeLiveKitSessionKey(
  sessionId: string,
  sessionKey: Uint8Array,
  channelId: string
): Promise<void> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('livekitSessionKeys', 'readwrite')
    const store = tx.objectStore('livekitSessionKeys')
    store.put({
      sessionId,
      sessionKey: uint8ArrayToBase64(sessionKey),
      channelId,
      createdAt: Date.now(),
    })
    tx.oncomplete = () => {
      db.close()
      resolve()
    }
    tx.onerror = () => {
      db.close()
      reject(tx.error)
    }
  })
}

/**
 * Obtenir una clau de sessió LiveKit.
 */
export async function getLiveKitSessionKey(sessionId: string): Promise<Uint8Array | null> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('livekitSessionKeys', 'readonly')
    const store = tx.objectStore('livekitSessionKeys')
    const request = store.get(sessionId)
    request.onsuccess = () => {
      const result = request.result
      if (!result) {
        db.close()
        resolve(null)
        return
      }
      const sessionKey = base64ToUint8Array(result.sessionKey)
      db.close()
      resolve(sessionKey)
    }
    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

// ── Neteja General ────────────────────────────────────────────

/**
 * Netejar tota la base de dades (per a logout o test).
 */
export async function clearAll(): Promise<void> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(
      ['keypairs', 'keypairsV2', 'channelKeysBytes', 'channelKeysByVersion', 'channelKeys', 'devicePublicKeys', 'livekitSessionKeys'],
      'readwrite'
    )
    const stores = [
      tx.objectStore('keypairs'),
      tx.objectStore('keypairsV2'),
      tx.objectStore('channelKeysBytes'),
      tx.objectStore('channelKeysByVersion'),
      tx.objectStore('channelKeys'),
      tx.objectStore('devicePublicKeys'),
      tx.objectStore('livekitSessionKeys'),
    ]
    stores.forEach((store) => store.clear())
    tx.oncomplete = () => {
      db.close()
      resolve()
    }
    tx.onerror = () => {
      db.close()
      reject(tx.error)
    }
  })
}

