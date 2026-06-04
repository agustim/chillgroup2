import { describe, it, expect, vi, beforeEach } from 'vitest'
import { encryptChannelMessage, decryptMessagesForChannel } from './channel-crypto'
import { generateSymmetricKey } from './crypto'
import * as storage from './storage'
import type { Message } from '../types'

// Mockejem les crides a API per aïllar la lògica criptogràfica
vi.mock('./api', () => ({
  channelGetKey: vi.fn(),
  channelUploadKeys: vi.fn(),
  channelGetMemberDevices: vi.fn().mockResolvedValue([]),
  deviceUpdatePublicKey: vi.fn(),
  channelGetAllKeyBundles: vi.fn().mockResolvedValue([]),
}))

describe('encryptChannelMessage - mode none', () => {
  it("retorna el text en clar si encryptionType='none'", async () => {
    const result = await encryptChannelMessage('ch-1', 'none', 'hola')
    expect(result.encryptedPayload).toBe('hola')
    expect(result.iv).toBe('')
    expect(result.keyVersion).toBeNull()
  })

  it('textos buits passen sense error', async () => {
    const result = await encryptChannelMessage('ch-2', 'none', '')
    expect(result.encryptedPayload).toBe('')
  })
})

describe('encryptChannelMessage - mode symmetric', () => {
  const channelId = 'ch-sym-1'

  beforeEach(async () => {
    // generateSymmetricKey retorna Uint8Array directament (no CryptoKey)
    const keyBytes = generateSymmetricKey()
    await storage.storeChannelKey(channelId, keyBytes, 'symmetric')
  })

  it('retorna payload xifrat diferent del text pla', async () => {
    const plaintext = 'Missatge secret'
    const result = await encryptChannelMessage(channelId, 'symmetric', plaintext)
    expect(result.encryptedPayload).not.toBe(plaintext)
    expect(result.iv.length).toBeGreaterThan(0)
  })

  it('dues encriptacions del mateix text donen IVs diferents', async () => {
    const r1 = await encryptChannelMessage(channelId, 'symmetric', 'text')
    const r2 = await encryptChannelMessage(channelId, 'symmetric', 'text')
    expect(r1.iv).not.toBe(r2.iv)
    expect(r1.encryptedPayload).not.toBe(r2.encryptedPayload)
  })

  it('llança error si no hi ha clau al canal', async () => {
    await expect(
      encryptChannelMessage('canal-sense-clau', 'symmetric', 'test')
    ).rejects.toThrow()
  })
})

describe('decryptMessagesForChannel - mode none', () => {
  it('retorna els missatges sense desxifrar si encryptionType=none', async () => {
    const messages: Message[] = [
      {
        messageId: '1',
        channelId: 'ch-none',
        senderUserId: 'u1',
        senderUsername: 'user1',
        senderDeviceId: 'd1',
        encryptedPayload: 'Missatge en clar',
        iv: '',
        keyVersion: null,
        timestamp: new Date().toISOString(),
        editedAt: null,
        deletedAt: null,
        attachmentIds: [],
        expiresAt: null,
      },
    ]

    const result = await decryptMessagesForChannel('ch-none', 'none', messages)
    // retorna Record<messageId, text>
    expect(result['1']).toBe('Missatge en clar')
  })
})

describe('encryptChannelMessage + decryptMessagesForChannel - round trip', () => {
  const channelId = 'ch-roundtrip'

  beforeEach(async () => {
    const keyBytes = generateSymmetricKey()
    await storage.storeChannelKey(channelId, keyBytes, 'symmetric')
  })

  it('xifra i desxifra correctament el missatge', async () => {
    const plaintext = 'Missatge E2E de prova'
    const { encryptedPayload, iv, keyVersion } = await encryptChannelMessage(
      channelId,
      'symmetric',
      plaintext
    )

    const messages: Message[] = [
      {
        messageId: 'rt-1',
        channelId,
        senderUserId: 'u1',
        senderUsername: 'user1',
        senderDeviceId: 'd1',
        encryptedPayload,
        iv,
        keyVersion: keyVersion ?? null,
        timestamp: new Date().toISOString(),
        editedAt: null,
        deletedAt: null,
        attachmentIds: [],
        expiresAt: null,
      },
    ]

    const decrypted = await decryptMessagesForChannel(channelId, 'symmetric', messages)
    expect(decrypted['rt-1']).toBe(plaintext)
  })
})
