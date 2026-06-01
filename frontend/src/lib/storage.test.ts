//! Tests unitaris per al mòdul de storage (IndexedDB).
//!
//! Segons especificació de definitions/TESTING.md

import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import {
  getDevicePublicKeys,
  getDeviceSecretKeys,
  storeKeypair,
  getKeypair,
  deleteKeypair,
  listKeypairs,
  storeChannelKey,
  getChannelKey,
  deleteChannelKey,
  storeDevicePublicKey,
  getDevicePublicKey,
  storeLiveKitSessionKey,
  getLiveKitSessionKey,
  clearAll,
} from './storage'
import { createLocalVault, lockLocalVault } from './local-vault'

const DB_NAME = 'chillgroup-store'

function cleanupDB(): Promise<void> {
  return new Promise((resolve) => {
    const req = indexedDB.deleteDatabase(DB_NAME)
    req.onsuccess = () => resolve()
    req.onerror = () => resolve()
  })
}

describe('IndexedDB Storage', () => {
  beforeEach(async () => {
    await cleanupDB()
    localStorage.clear()
    lockLocalVault()
  })

  afterEach(async () => {
    await cleanupDB()
    localStorage.clear()
    lockLocalVault()
  })

  describe('keypairs', () => {
    it('guarda i recupera un keypair', async () => {
      const secretKey = new Uint8Array(3168)
      secretKey.fill(42)

      await storeKeypair('device-1', secretKey)

      const retrieved = await getKeypair('device-1')
      expect(retrieved).toEqual(secretKey)
    })

    it('guarda i recupera també la signing key del dispositiu', async () => {
      const kemSecretKey = new Uint8Array([1, 2, 3])
      const dsaSecretKey = new Uint8Array([4, 5, 6])

      await storeKeypair('device-1', kemSecretKey, dsaSecretKey)

      const retrieved = await getDeviceSecretKeys('device-1')
      expect(retrieved?.kemSecretKey).toEqual(kemSecretKey)
      expect(retrieved?.dsaSecretKey).toEqual(dsaSecretKey)
    })

    it('retorna null si el keypair no existeix', async () => {
      const result = await getKeypair('nonexistent-device')
      expect(result).toBeNull()
    })

    it('sobre escriu un keypair existent', async () => {
      const key1 = new Uint8Array([1, 2, 3])
      const key2 = new Uint8Array([4, 5, 6])

      await storeKeypair('device-1', key1)
      await storeKeypair('device-1', key2)

      const retrieved = await getKeypair('device-1')
      expect(retrieved).toEqual(key2)
    })

    it('elimina un keypair', async () => {
      const secretKey = new Uint8Array(100)
      await storeKeypair('device-1', secretKey)

      await deleteKeypair('device-1')

      const retrieved = await getKeypair('device-1')
      expect(retrieved).toBeNull()
    })

    it('llista keypairs emmagatzemats', async () => {
      const key1 = new Uint8Array([1])
      const key2 = new Uint8Array([2])

      await storeKeypair('device-1', key1)
      await storeKeypair('device-2', key2)

      const keypairs = await listKeypairs()
      expect(keypairs).toHaveLength(2)
      expect(keypairs.map(k => k.deviceId)).toEqual(expect.arrayContaining(['device-1', 'device-2']))
    })
  })

  describe('channel keys', () => {
    it('guarda i recupera un channelKey', async () => {
      const channelKey = new Uint8Array(32)
      channelKey.fill(99)

      await storeChannelKey('channel-1', channelKey, 'asymmetric')

      const retrieved = await getChannelKey('channel-1')
      expect(retrieved).toEqual(channelKey)
    })

    it('retorna null si el canal no existeix', async () => {
      const result = await getChannelKey('nonexistent-channel')
      expect(result).toBeNull()
    })

    it('esborra un channelKey', async () => {
      const key = new Uint8Array(32)
      await storeChannelKey('channel-1', key, 'symmetric')
      await deleteChannelKey('channel-1')

      const retrieved = await getChannelKey('channel-1')
      expect(retrieved).toBeNull()
    })

    it('diferents tipus de clau', async () => {
      const key = new Uint8Array(32)
      await storeChannelKey('channel-1', key, 'symmetric')
      await storeChannelKey('channel-2', key, 'asymmetric')

      const retrieved1 = await getChannelKey('channel-1')
      const retrieved2 = await getChannelKey('channel-2')

      expect(retrieved1).toEqual(key)
      expect(retrieved2).toEqual(key)
    })

    it('desa la clau de canal xifrada en repòs quan hi ha vault local', async () => {
      await createLocalVault('passphrase-local')

      const channelKey = new Uint8Array(32)
      channelKey.fill(77)
      await storeChannelKey('channel-vault', channelKey, 'asymmetric', 1, 'kv-1')

      const db = await new Promise<IDBDatabase>((resolve, reject) => {
        const req = indexedDB.open(DB_NAME)
        req.onsuccess = () => resolve(req.result)
        req.onerror = () => reject(req.error)
      })

      const row = await new Promise<any>((resolve, reject) => {
        const tx = db.transaction('channelKeysByVersion', 'readonly')
        const store = tx.objectStore('channelKeysByVersion')
        const req = store.get('channel-vault::1')
        req.onsuccess = () => resolve(req.result)
        req.onerror = () => reject(req.error)
      })
      db.close()

      expect(row?.keyBytes ?? null).toBeNull()
      expect(typeof row?.keyCiphertext).toBe('string')

      const decrypted = await getChannelKey('channel-vault')
      expect(decrypted).toEqual(channelKey)
    })
  })

  describe('device public keys', () => {
    it('guarda i recupera una clau pública', async () => {
      const publicKey = new Uint8Array(1568)
      publicKey.fill(7)

      await storeDevicePublicKey('device-1', publicKey)

      const retrieved = await getDevicePublicKey('device-1')
      expect(retrieved).toEqual(publicKey)
    })

    it('guarda i recupera claus públiques KEM i DSA', async () => {
      const kemPublicKey = new Uint8Array([7, 8, 9])
      const dsaPublicKey = new Uint8Array([10, 11, 12])

      await storeDevicePublicKey('device-1', kemPublicKey, dsaPublicKey)

      const retrieved = await getDevicePublicKeys('device-1')
      expect(retrieved?.kemPublicKey).toEqual(kemPublicKey)
      expect(retrieved?.dsaPublicKey).toEqual(dsaPublicKey)
    })

    it('retorna null si no existeix', async () => {
      const result = await getDevicePublicKey('nonexistent')
      expect(result).toBeNull()
    })
  })

  describe('livekit session keys', () => {
    it('guarda i recupera una clau de sessió', async () => {
      const sessionKey = new Uint8Array(32)
      sessionKey.fill(55)

      await storeLiveKitSessionKey('session-1', sessionKey, 'channel-1')

      const retrieved = await getLiveKitSessionKey('session-1')
      expect(retrieved).toEqual(sessionKey)
    })

    it('retorna null si la sessió no existeix', async () => {
      const result = await getLiveKitSessionKey('nonexistent-session')
      expect(result).toBeNull()
    })
  })

  describe('clearAll', () => {
    it('esborra tota la base de dades', async () => {
      await storeKeypair('device-1', new Uint8Array([1]))
      await storeChannelKey('channel-1', new Uint8Array([2]), 'symmetric')
      await storeDevicePublicKey('device-1', new Uint8Array([3]))
      await storeLiveKitSessionKey('session-1', new Uint8Array([4]), 'channel-1')

      await clearAll()

      expect(await getKeypair('device-1')).toBeNull()
      expect(await getChannelKey('channel-1')).toBeNull()
      expect(await getDevicePublicKey('device-1')).toBeNull()
      expect(await getLiveKitSessionKey('session-1')).toBeNull()
    })
  })
})