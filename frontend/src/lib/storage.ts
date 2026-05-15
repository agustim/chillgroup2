//! IndexedDB wrapper per a ChillGroup v2.
//!
//! Gestiona l'emmagatzematge local de claus criptogràfiques i dades de sessió.

const DB_NAME = 'chillgroup-store'
const DB_VERSION = 1

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

      // Object store per a claus de canal (bytes)
      if (!db.objectStoreNames.contains('channelKeysBytes')) {
        const store = db.createObjectStore('channelKeysBytes', { keyPath: 'channelId' })
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

/**
 * Guardar el keypair del dispositiu a IndexedDB.
 */
export async function storeKeypair(deviceId: string, secretKey: Uint8Array): Promise<void> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('keypairs', 'readwrite')
    const store = tx.objectStore('keypairs')
    store.put({
      deviceId,
      kyberSecretKey: uint8ArrayToBase64(secretKey),
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
      db.close()
      resolve(keyBytes)
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

// ── Channel Keys (Bytes) ──────────────────────────────────────

/**
 * Guardar una clau de canal (bytes) a IndexedDB.
 */
export async function storeChannelKey(
  channelId: string,
  keyBytes: Uint8Array,
  type: 'symmetric' | 'asymmetric'
): Promise<void> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('channelKeysBytes', 'readwrite')
    const store = tx.objectStore('channelKeysBytes')
    store.put({
      channelId,
      keyBytes: uint8ArrayToBase64(keyBytes),
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
      const keyBytes = base64ToUint8Array(result.keyBytes)
      db.close()
      resolve(keyBytes)
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
    const tx = db.transaction('channelKeysBytes', 'readwrite')
    const store = tx.objectStore('channelKeysBytes')
    store.delete(channelId)
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
export async function cleanupExpiredKeys(): Promise<number> {
  const db = await getDB()
  const now = Date.now()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('channelKeysBytes', 'readwrite')
    const store = tx.objectStore('channelKeysBytes')
    const request = store.getAll()
    let count = 0
    request.onsuccess = () => {
      const items = request.result
      const deleteRequests: IDBRequest[] = []
      for (const item of items) {
        if (item.expiresAt && item.expiresAt < now) {
          deleteRequests.push(store.delete(item.channelId))
          count++
        }
      }
      if (deleteRequests.length > 0) {
        const tx2 = db.transaction('channelKeysBytes', 'readwrite')
        const store2 = tx2.objectStore('channelKeysBytes')
        deleteRequests.forEach((req) => {
          const idx = req.result
          store2.delete(idx)
        })
        tx2.oncomplete = () => {
          db.close()
          resolve(count)
        }
        tx2.onerror = () => {
          db.close()
          reject(tx2.error)
        }
      } else {
        db.close()
        resolve(0)
      }
    }
    request.onerror = () => {
      db.close()
      reject(request.error)
    }
  })
}

// ── Device Public Keys ────────────────────────────────────────

/**
 * Guardar la clau pública del dispositiu.
 */
export async function storeDevicePublicKey(
  deviceId: string,
  publicKey: Uint8Array
): Promise<void> {
  const db = await getDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction('devicePublicKeys', 'readwrite')
    const store = tx.objectStore('devicePublicKeys')
    store.put({
      deviceId,
      publicKey: uint8ArrayToBase64(publicKey),
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
      const publicKey = base64ToUint8Array(result.publicKey)
      db.close()
      resolve(publicKey)
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
      ['keypairs', 'channelKeysBytes', 'channelKeys', 'devicePublicKeys', 'livekitSessionKeys'],
      'readwrite'
    )
    const stores = [
      tx.objectStore('keypairs'),
      tx.objectStore('channelKeysBytes'),
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