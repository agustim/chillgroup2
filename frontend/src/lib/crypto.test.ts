//! Tests unitaris per al mòdul de criptografia.
//!
//! Segons especificació de definitions/TESTING.md

import { describe, it, expect, beforeEach } from 'vitest'
import {
  generateKey,
  encryptMessage,
  decryptMessage,
  generateSymmetricKey,
  encryptWithBytes,
  decryptWithBytes,
  encryptPlainText,
  verifyUniqueEncryption,
} from './crypto'

describe('Crypto Module - AES-GCM-256', () => {
  let key: Awaited<ReturnType<typeof generateKey>>

  beforeEach(async () => {
    key = await generateKey()
  })

  describe('generateKey', () => {
    it('genera una clau AES-256 vàlida', async () => {
      expect(key).toBeDefined()
      expect(key.type).toBe('secret')
      expect(key.algorithm.name).toBe('AES-GCM')
      // length pot ser accessible via (key.algorithm as any).length
    })

    it('genera claus diferents cada vegada', async () => {
      const key2 = await generateKey()
      const raw1 = await crypto.subtle.exportKey('raw', key)
      const raw2 = await crypto.subtle.exportKey('raw', key2)
      const arr1 = new Uint8Array(raw1)
      const arr2 = new Uint8Array(raw2)
      // Les claus han de ser diferents
      expect(arr1).not.toEqual(arr2)
    })
  })

  describe('encryptMessage', () => {
    it('xifra i desxifra un missatge', async () => {
      const plaintext = 'Missatge de prova E2EE'
      const encrypted = await encryptMessage(key, plaintext)

      expect(encrypted.encrypted).not.toBe(plaintext)
      expect(encrypted.iv.length).toBeGreaterThan(0)

      const decrypted = await decryptMessage(key, encrypted.encrypted, encrypted.iv)
      expect(decrypted).toBe(plaintext)
    })

    it('xifra missatges buits', async () => {
      const plaintext = ''
      const encrypted = await encryptMessage(key, plaintext)
      const decrypted = await decryptMessage(key, encrypted.encrypted, encrypted.iv)
      expect(decrypted).toBe(plaintext)
    })

    it('xifra missatges amb caràcters especials', async () => {
      const plaintext = 'HOLA QUÈ TAL! Ñ / * & ( ) @#$%'
      const encrypted = await encryptMessage(key, plaintext)
      const decrypted = await decryptMessage(key, encrypted.encrypted, encrypted.iv)
      expect(decrypted).toBe(plaintext)
    })

    it('xifra missatges llargs', async () => {
      const plaintext = 'a'.repeat(10000)
      const encrypted = await encryptMessage(key, plaintext)
      const decrypted = await decryptMessage(key, encrypted.encrypted, encrypted.iv)
      expect(decrypted).toBe(plaintext)
    })
  })

  describe('KEM Encapsulate/Decapsulate simulation', () => {
    it('xifra i desencapsula una clau channelKey', async () => {
      const channelKey = generateSymmetricKey()
      const plaintext = 'channel-secret-data'

      const encrypted = await encryptWithBytes(channelKey, plaintext)
      const decrypted = await decryptWithBytes(channelKey, encrypted.encrypted, encrypted.iv)

      expect(decrypted).toBe(plaintext)
    })

    it('clés diferents produeixen resultats diferents', async () => {
      const channelKey1 = generateSymmetricKey()
      const channelKey2 = generateSymmetricKey()
      const plaintext = 'test-message'

      const enc1 = await encryptWithBytes(channelKey1, plaintext)
      const enc2 = await encryptWithBytes(channelKey2, plaintext)

      expect(enc1.encrypted).not.toBe(enc2.encrypted)
    })
  })

  describe('AES-GCM Encrypt/Decrypt', () => {
    it('xifra i desxifra un missatge amb clau genèrica', async () => {
      const key2 = await generateKey()
      const plaintext = 'Missatge de prova amb clau diferent'
      const encrypted = await encryptMessage(key2, plaintext)

      expect(encrypted.encrypted).not.toBe(plaintext)
      expect(encrypted.iv.length).toBeGreaterThan(0)

      const decrypted = await decryptMessage(key2, encrypted.encrypted, encrypted.iv)
      expect(decrypted).toBe(plaintext)
    })

    it('dos encriptacions del mateix text tenen IV diferent', async () => {
      const msg = 'Missatge idèntic'
      const enc1 = await encryptMessage(key, msg)
      const enc2 = await encryptMessage(key, msg)

      // Més encriptats però IV diferent
      expect(enc1.encrypted).not.toBe(enc2.encrypted)
      expect(enc1.iv).not.toBe(enc2.iv)
    })

    it('no pot desxifrar amb IV incorrecte', async () => {
      const plaintext = 'secret message'
      const encrypted = await encryptMessage(key, plaintext)

      // Modificar l'IV
      const wrongIv = 'AAAAAA'
      await expect(
        decryptMessage(key, encrypted.encrypted, wrongIv)
      ).rejects.toThrow()
    })
  })

  describe('encryptPlainText', () => {
    it('encripta i retorna la clau generada', async () => {
      const plaintext = 'missatge secret'
      const result = await encryptPlainText(plaintext)

      expect(result.encrypted).not.toBe(plaintext)
      expect(result.iv.length).toBeGreaterThan(0)
      expect(result.keyBytes.length).toBe(32) // AES-256 = 32 bytes

      // Desencriptar amb la clau retornada
      const decrypted = await decryptWithBytes(result.keyBytes, result.encrypted, result.iv)
      expect(decrypted).toBe(plaintext)
    })

    it('encripta missatges amb emojis', async () => {
      const plaintext = 'Missatge amb emoji 🎉🔒✨'
      const result = await encryptPlainText(plaintext)
      const decrypted = await decryptWithBytes(result.keyBytes, result.encrypted, result.iv)
      expect(decrypted).toBe(plaintext)
    })
  })

  describe('verifyUniqueEncryption', () => {
    it('verifica que cada encriptació és única amb mateixa clau', async () => {
      // Generar una sola clau compartida
      const sharedKey = generateSymmetricKey()
      const plaintext = 'test unique encryption'

      // Encriptar dues vegades amb la mateixa clau
      const enc1 = await encryptWithBytes(sharedKey, plaintext)
      const enc2 = await encryptWithBytes(sharedKey, plaintext)

      // Amb la mateixa clau però IV diferent, els ciphertexts han de ser diferents
      expect(enc1.encrypted).not.toBe(enc2.encrypted)
      expect(enc1.iv).not.toBe(enc2.iv)

      // Però ambdós han de desencriptar al mateix text
      const decrypted1 = await decryptWithBytes(sharedKey, enc1.encrypted, enc1.iv)
      const decrypted2 = await decryptWithBytes(sharedKey, enc2.encrypted, enc2.iv)
      expect(decrypted1).toBe(plaintext)
      expect(decrypted2).toBe(plaintext)
    })

    it('verifica amb missatge buit', async () => {
      const result = await verifyUniqueEncryption('')
      expect(result.areDifferent).toBe(true)
    })
  })

  describe('edge cases', () => {
    it('manipulació de ciphertext detecta canvis', async () => {
      const plaintext = 'secret message'
      const encrypted = await encryptMessage(key, plaintext)

      // Modificar el ciphertext
      const manipulated = encrypted.encrypted + 'X'
      await expect(
        decryptMessage(key, manipulated, encrypted.iv)
      ).rejects.toThrow()
    })

    it('reutilitzar IV amb la mateixa clau és perillós però funciona', async () => {
      const plaintext1 = 'message 1'
      const plaintext2 = 'message 2'

      // Encriptar dos missatges amb el mateix IV
      const iv = crypto.getRandomValues(new Uint8Array(12))
      const encoder = new TextEncoder()

      const enc1 = await crypto.subtle.encrypt(
        { name: 'AES-GCM', iv },
        key,
        encoder.encode(plaintext1)
      )

      const enc2 = await crypto.subtle.encrypt(
        { name: 'AES-GCM', iv },
        key,
        encoder.encode(plaintext2)
      )

      // Amb el mateix IV, es poden desxifrar correctament
      const dec1 = await crypto.subtle.decrypt(
        { name: 'AES-GCM', iv },
        key,
        enc1
      )
      const dec2 = await crypto.subtle.decrypt(
        { name: 'AES-GCM', iv },
        key,
        enc2
      )

      expect(new TextDecoder().decode(dec1)).toBe(plaintext1)
      expect(new TextDecoder().decode(dec2)).toBe(plaintext2)
    })
  })
})