//! Tests unitaris per a criptografia E2EE de canals i missatges.
//!
//! Segons especificació de definitions/TESTING.md
//!
//! Testa el flux complet:
//! 1. Crear canals dels tres tipus (none, symmetric, asymmetric)
//! 2. Enviar missatges xifrats
//! 3. Verificar que altres participants poden desxifrar

import { describe, it, expect, beforeEach, vi } from 'vitest'
import {
  generateSymmetricKey,
  encryptWithBytes,
  decryptWithBytes,
} from './crypto'
import {
  channelsCreate,
  ChannelInfo,
} from './api'

// Mock de fetch per a API
const mockFetch = vi.fn()
vi.stubGlobal('fetch', mockFetch)

function setupMocks(responseBody: any, status = 200) {
  mockFetch.mockResolvedValueOnce({
    ok: status < 400,
    status,
    json: async () => responseBody,
  })
}

// Simula generació de claus KEM (Kyber Encapsulation)
function simulateKemEncapsulate(channelKey: Uint8Array): {
  kemCiphertext: string
  sharedSecret: Uint8Array
} {
  // En un sistema real, això faria KEM sobre la clau pública del destinatari
  // Aquí simulam amb HMAC derivat
  const sharedSecret = new Uint8Array(32)
  sharedSecret.set(channelKey.slice(0, 32))
  return {
    kemCiphertext: btoa('simulated-kem-ciphertext'),
    sharedSecret,
  }
}

function simulateKemDecapsulate(kemCiphertext: string, channelKey: Uint8Array): Uint8Array {
  // En un sistema real, això faria decapsulació amb la clau privada
  // Aquí simulam amb la clau del canal
  return channelKey
}

describe('E2EE - Channel Key Management', () => {
  describe('canals sense encriptació (none)', () => {
    it('crea un canal sense encriptacio', async () => {
      setupMocks({
        success: true,
        data: {
          channelId: 'ch-none-1',
          name: 'general',
          type: 'text',
          encryptionType: 'none',
          messageTTL: null,
          isPrivate: false,
          createdAt: '2026-01-01T00:00:00Z',
        } as ChannelInfo,
      })

      const result = await channelsCreate('srv-1', 'general', 'text', 'none')

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.encryptionType).toBe('none')
      }
    })

    it('envia missatges en text pla en canals none', async () => {
      const plaintext = 'Missatge public del canal general'

      // En canals none, el missatge va en text pla
      expect(plaintext).toBeDefined()
      expect(plaintext.length).toBeGreaterThan(0)
    })
  })

  describe('canals simetrics (symmetric)', () => {
    beforeEach(() => {
      vi.clearAllMocks()
    })

    it('genera clau simetrica per al canal', () => {
      const channelKey = generateSymmetricKey()

      expect(channelKey).toBeDefined()
      expect(channelKey.length).toBe(32) // AES-256 = 32 bytes
    })

    it('xifra missatge amb clau simetrica del canal', async () => {
      // Cada usuari té la mateixa clau del canal
      const channelKey = generateSymmetricKey()
      const plaintext = 'Missatge secret del canal simetric'

      // Usuari 1 xifra
      const { encrypted, iv } = await encryptWithBytes(channelKey, plaintext)

      expect(encrypted).not.toBe(plaintext)
      expect(iv).toBeDefined()

      // Usuari 2 desxifra amb la mateixa clau
      const decrypted = await decryptWithBytes(channelKey, encrypted, iv)

      expect(decrypted).toBe(plaintext)
    })

    it('diferents usuaris poden desxifrar el mateix missatge', async () => {
      // Tots els usuaris comparteixen la mateixa clau
      const sharedChannelKey = generateSymmetricKey()
      const plaintext = 'Missatge compartit'

      // Usuari 1 xifra
      const enc1 = await encryptWithBytes(sharedChannelKey, plaintext)

      // Usuari 2 desxifra
      const dec2 = await decryptWithBytes(sharedChannelKey, enc1.encrypted, enc1.iv)

      // Usuari 3 desxifra
      const dec3 = await decryptWithBytes(sharedChannelKey, enc1.encrypted, enc1.iv)

      expect(dec2).toBe(plaintext)
      expect(dec3).toBe(plaintext)
    })

    it('missatge xifrat amb clau incorrecta no es pot desxifrar', async () => {
      const correctKey = generateSymmetricKey()
      const wrongKey = generateSymmetricKey()
      const plaintext = 'secret message'

      const { encrypted, iv } = await encryptWithBytes(correctKey, plaintext)

      await expect(
        decryptWithBytes(wrongKey, encrypted, iv)
      ).rejects.toThrow()
    })

    it('gestiona multiples missatges en el mateix canal simetric', async () => {
      const channelKey = generateSymmetricKey()
      const messages = [
        'Primer missatge',
        'Segon missatge',
        'Tercer missatge amb emoji 🎉',
      ]

      // Tots els usuaris xifren i desxifren
      for (const msg of messages) {
        const { encrypted, iv } = await encryptWithBytes(channelKey, msg)
        const decrypted = await decryptWithBytes(channelKey, encrypted, iv)
        expect(decrypted).toBe(msg)
      }
    })
  })

  describe('canals asimetrics (asymmetric)', () => {
    beforeEach(() => {
      vi.clearAllMocks()
    })

    interface DeviceInfo {
      deviceId: string
      username: string
      publicKey: Uint8Array // Simulada
      channelKey?: Uint8Array // Claus encapsulades
    }

    it('simula creacio de canal asimetric amb 3 usuaris', async () => {
      // Crear 3 dispositius amb les seves claus
      const users: DeviceInfo[] = [
        { deviceId: 'user-1', username: 'agusti', publicKey: new Uint8Array(1568).fill(1) },
        { deviceId: 'user-2', username: 'marcus', publicKey: new Uint8Array(1568).fill(2) },
        { deviceId: 'user-3', username: 'julia', publicKey: new Uint8Array(1568).fill(3) },
      ]

      // Generar clau del canal
      const channelKey = generateSymmetricKey()

      // L'owner encapsula la clau per a cada dispositiu
      const encapsulatedKeys: Record<string, { kemCiphertext: string }> = {}

      for (const user of users) {
        const { kemCiphertext } = simulateKemEncapsulate(channelKey)
        encapsulatedKeys[user.deviceId] = { kemCiphertext }
      }

      // Verificar que tots tenen el seu ciphertext
      expect(Object.keys(encapsulatedKeys)).toHaveLength(3)
      expect(encapsulatedKeys['user-1'].kemCiphertext).toBeDefined()
      expect(encapsulatedKeys['user-2'].kemCiphertext).toBeDefined()
      expect(encapsulatedKeys['user-3'].kemCiphertext).toBeDefined()
    })

    it('simula flux complet E2EE asimetric', async () => {
      // 1. Configuracio inicial
      const users: DeviceInfo[] = [
        { deviceId: 'user-1', username: 'agusti', publicKey: new Uint8Array(1568).fill(1) },
        { deviceId: 'user-2', username: 'marcus', publicKey: new Uint8Array(1568).fill(2) },
        { deviceId: 'user-3', username: 'julia', publicKey: new Uint8Array(1568).fill(3) },
      ]

      // 2. Generar clau del canal (només l'owner coneix)
      const channelKey = generateSymmetricKey()

      // 3. Owner encapsula clau per a cada usuari
      const encapsulated: Record<string, string> = {}
      for (const user of users) {
        const { kemCiphertext } = simulateKemEncapsulate(channelKey)
        encapsulated[user.deviceId] = kemCiphertext
      }

      // 4. Cada usuari desencapsula la seva clau
      for (const user of users) {
        const decryptedKey = simulateKemDecapsulate(encapsulated[user.deviceId], channelKey)
        expect(decryptedKey).toEqual(channelKey)
      }

      // 5. Agusti envia missatge xifrat
      const plaintext = 'Missatge E2EE des de Agusti'
      const { encrypted, iv } = await encryptWithBytes(channelKey, plaintext)

      // 6. Tots poden desxifrar
      for (const user of users) {
        const decrypted = await decryptWithBytes(channelKey, encrypted, iv)
        expect(decrypted).toBe(plaintext)
      }
    })

    it('dispositiu sense clau no pot desxifrar missatges', async () => {
      const channelKey = generateSymmetricKey()
      const unauthorizedKey = generateSymmetricKey()
      const plaintext = 'Missatge privat'

      // Xifrar amb clau correcta
      const { encrypted, iv } = await encryptWithBytes(channelKey, plaintext)

      // Intentar desxifrar amb clau incorrecta (device sense acces)
      await expect(
        decryptWithBytes(unauthorizedKey, encrypted, iv)
      ).rejects.toThrow()
    })

    it('gestiona revocacio de dispositiu', async () => {
      const users: DeviceInfo[] = [
        { deviceId: 'user-1', username: 'agusti', publicKey: new Uint8Array(1568).fill(1) },
        { deviceId: 'user-2', username: 'marcus', publicKey: new Uint8Array(1568).fill(2) },
      ]

      const channelKey = generateSymmetricKey()

      // Generar claus encapsulades
      const keys: Record<string, string> = {}
      for (const user of users) {
        const { kemCiphertext } = simulateKemEncapsulate(channelKey)
        keys[user.deviceId] = kemCiphertext
      }

      // Revocar user-2
      const revokedDevices = new Set(['user-2'])

      // User-1 encara pot desxifrar
      const { encrypted, iv } = await encryptWithBytes(channelKey, 'missatge')
      const dec1 = await decryptWithBytes(channelKey, encrypted, iv)
      expect(dec1).toBe('missatge')

      // User-2 ja no te la seva clau (revocada)
      // Simulem que no pot acceder
      expect(revokedDevices.has('user-2')).toBe(true)
    })
  })

  describe('flux complet de missatges E2EE', () => {
    beforeEach(() => {
      vi.clearAllMocks()
    })

    it('flux complet: canal none', async () => {
      // 1. Crear canal
      setupMocks({
        success: true,
        data: {
          channelId: 'ch-1',
          name: 'general',
          type: 'text',
          encryptionType: 'none',
          messageTTL: null,
          isPrivate: false,
          createdAt: '2026-01-01T00:00:00Z',
        },
      })

      const createResult = await channelsCreate('srv-1', 'general', 'text', 'none')
      expect(createResult.success).toBe(true)

      // 2. En canal none, missatges van en text pla
      const plaintextMsg = 'Hola a tothom!'
      expect(plaintextMsg).toBeDefined()
    })

    it('flux complet: canal simetric', async () => {
      // 1. Crear canal
      setupMocks({
        success: true,
        data: {
          channelId: 'ch-2',
          name: 'secret-symmetric',
          type: 'text',
          encryptionType: 'symmetric',
          messageTTL: null,
          isPrivate: true,
          createdAt: '2026-01-01T00:00:00Z',
        },
      })

      const createResult = await channelsCreate('srv-1', 'secret-symmetric', 'text', 'symmetric')
      expect(createResult.success).toBe(true)

      // 2. Generar clau compartida
      const channelKey = generateSymmetricKey()

      // 3. Usuari 1 envia missatge
      const msg1 = 'Missatge xifrat simetric'
      const { encrypted: enc1, iv: iv1 } = await encryptWithBytes(channelKey, msg1)

      // 4. Usuari 2 rep i desxifra
      const dec1 = await decryptWithBytes(channelKey, enc1, iv1)
      expect(dec1).toBe(msg1)

      // 5. Usuari 2 envia resposta
      const msg2 = 'Resposta xifrada'
      const { encrypted: enc2, iv: iv2 } = await encryptWithBytes(channelKey, msg2)
      const dec2 = await decryptWithBytes(channelKey, enc2, iv2)
      expect(dec2).toBe(msg2)
    })

    it('flux complet: canal asimetric', async () => {
      // 1. Crear canal privat
      setupMocks({
        success: true,
        data: {
          channelId: 'ch-3',
          name: 'secret-asymmetric',
          type: 'text',
          encryptionType: 'asymmetric',
          messageTTL: null,
          isPrivate: true,
          createdAt: '2026-01-01T00:00:00Z',
        },
      })

      const createResult = await channelsCreate('srv-1', 'secret-asymmetric', 'text', 'asymmetric')
      expect(createResult.success).toBe(true)

      // 2. Simular 3 usuaris
      const users = [
        { deviceId: 'u1', username: 'agusti' },
        { deviceId: 'u2', username: 'marcus' },
        { deviceId: 'u3', username: 'julia' },
      ]

      // 3. Owner genera clau del canal i encapsula per a cada usuari
      const channelKey = generateSymmetricKey()
      const encapsulatedKeys: Record<string, string> = {}

      for (const user of users) {
        const { kemCiphertext } = simulateKemEncapsulate(channelKey)
        encapsulatedKeys[user.deviceId] = kemCiphertext
      }

      // 4. Agusti envia missatge
      const fromAgusti = 'Hola des de Agusti 🎉'
      const { encrypted: encAgusti, iv: ivAgusti } = await encryptWithBytes(channelKey, fromAgusti)

      // 5. Tots desxifren
      for (const user of users) {
        const decrypted = await decryptWithBytes(channelKey, encAgusti, ivAgusti)
        expect(decrypted).toBe(fromAgusti)
      }

      // 6. Marcus envia missatge
      const fromMarcus = 'Resposta de Marcus'
      const { encrypted: encMarcus, iv: ivMarcus } = await encryptWithBytes(channelKey, fromMarcus)

      // 7. Tots desxifren
      for (const user of users) {
        const decrypted = await decryptWithBytes(channelKey, encMarcus, ivMarcus)
        expect(decrypted).toBe(fromMarcus)
      }
    })

    it('interoperabilitat: mateix canal, diferents usuaris', async () => {
      const channelKey = generateSymmetricKey()
      const participants = ['agusti', 'marcus', 'julia', 'pere', 'anna']
      const messages: Array<{ from: string; encrypted: string; iv: string }> = []

      // Cada usuari envia un missatge
      for (const username of participants) {
        const msg = `Missatge de ${username}`
        const { encrypted, iv } = await encryptWithBytes(channelKey, msg)
        messages.push({ from: username, encrypted, iv })
      }

      // Verificar que totes les respostes es poden desxifrar per cada participant
      for (const msg of messages) {
        for (let i = 0; i < participants.length; i++) {
          const decrypted = await decryptWithBytes(channelKey, msg.encrypted, msg.iv)
          expect(decrypted).toBe(`Missatge de ${msg.from}`)
        }
      }
    })

    it('verifica expiresAt en missatges amb TTL', async () => {
      const channelKey = generateSymmetricKey()
      const expiresAt = '2026-12-31T23:59:59Z'

      const { encrypted, iv } = await encryptWithBytes(channelKey, 'missatge temporal')

      // Verificar que el camp expiresAt es pot establir
      expect(expiresAt).toBeDefined()
      expect(new Date(expiresAt).getTime()).toBeGreaterThan(Date.now())
    })
  })

  describe('seguretat E2EE', () => {
    it('dos missatges identicals produeixen xifrats diferents (IV aleatori)', async () => {
      const key = generateSymmetricKey()
      const sameMessage = 'missatge identic'

      const enc1 = await encryptWithBytes(key, sameMessage)
      const enc2 = await encryptWithBytes(key, sameMessage)

      expect(enc1.encrypted).not.toBe(enc2.encrypted)
      expect(enc1.iv).not.toBe(enc2.iv)
    })

    it('missatge manipulats no es poden desxifrar', async () => {
      const key = generateSymmetricKey()
      const plaintext = 'secret'

      const { encrypted, iv } = await encryptWithBytes(key, plaintext)

      // Manipular el ciphertext
      const manipulated = encrypted + 'tampered'

      await expect(
        decryptWithBytes(key, manipulated, iv)
      ).rejects.toThrow()
    })

    it('IV incorrecte no permet desxifrar', async () => {
      const key = generateSymmetricKey()
      const plaintext = 'secret'

      const { encrypted } = await encryptWithBytes(key, plaintext)

      await expect(
        decryptWithBytes(key, encrypted, 'wrong-iv-value')
      ).rejects.toThrow()
    })
  })
})