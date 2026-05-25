//! Tests unitaris per al client API.
//!
//! Segons especificació de definitions/TESTING.md

import { describe, it, expect, beforeEach, vi } from 'vitest'
import {
  authRegister,
  authLogin,
  authRefresh,
  authMe,
  serversList,
  serversCreate,
  serversGet,
  serversDelete,
  messagesList,
  messagesSend,
  messagesEdit,
  messagesDelete,
  messagesGet,
  messagesCheckNew,
  dmSend,
  dmChannelOpen,
  dmChannelsList,
  dmMessagesSend,
  dmChannelUpdateSettings,
  dmChannelRotateKey,
  channelsList,
  channelsCreate,
  channelsGetKeys,
  channelInvite,
  channelGetKey,
  channelGetMemberDevices,
} from './api'

// Mock de sessionStorage
const mockSessionStorage: Record<string, string> = {}
const mockSessionStorageImpl = {
  getItem: vi.fn((key: string) => mockSessionStorage[key] || null),
  setItem: vi.fn((key: string, value: string) => { mockSessionStorage[key] = value }),
  removeItem: vi.fn((key: string) => { delete mockSessionStorage[key] }),
  clear: vi.fn(() => { Object.keys(mockSessionStorage).forEach(k => delete mockSessionStorage[k]) }),
}
vi.stubGlobal('sessionStorage', mockSessionStorageImpl)

// Mock de fetch
const mockFetch = vi.fn()
vi.stubGlobal('fetch', mockFetch)

// Token de test
const TEST_TOKEN = 'test-jwt-token'

function setupMocks(responseBody: any, status = 200, token?: string) {
  if (token) {
    mockSessionStorage['chillgroup-token'] = token
  }
  mockFetch.mockResolvedValueOnce({
    ok: status < 400,
    status,
    json: async () => responseBody,
  })
}

describe('API Client', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockSessionStorage['chillgroup-token'] = TEST_TOKEN
  })

  describe('authRegister', () => {
    it('envia request de registre correctament', async () => {
      setupMocks({
        success: true,
        data: {
          userId: 'user-1',
          username: 'testuser',
          token: 'new-token',
          deviceId: 'device-1',
          deviceLabel: 'Test Browser',
        },
      })

      const result = await authRegister('testuser', 'password123')

      expect(mockFetch).toHaveBeenCalledWith('/api/auth/register', {
        method: 'POST',
        headers: expect.objectContaining({
          'Content-Type': 'application/json',
          'Authorization': 'Bearer test-jwt-token',
        }),
        body: JSON.stringify({ username: 'testuser', password: 'password123' }),
      })
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.username).toBe('testuser')
      }
    })

    it('retorna error quan el registre falla', async () => {
      setupMocks({
        success: false,
        error: {
          code: 409,
          message: 'El nom d\'usuari ja existeix',
        },
      }, 409)

      const result = await authRegister('existinguser', 'password123')
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.code).toBe(409)
      }
    })
  })

  describe('authLogin', () => {
    it('envia request de login correctament', async () => {
      setupMocks({
        success: true,
        data: {
          userId: 'user-1',
          username: 'testuser',
          token: 'new-token',
          deviceId: 'device-1',
          deviceLabel: 'Test Browser',
          isAdmin: false,
        },
      })

      const result = await authLogin('testuser', 'password123')

      expect(mockFetch).toHaveBeenCalledWith('/api/auth/login', {
        method: 'POST',
        headers: expect.objectContaining({
          'Content-Type': 'application/json',
        }),
        body: JSON.stringify({ username: 'testuser', password: 'password123' }),
      })
      expect(result.success).toBe(true)
    })

    it('retorna error 401 quan les credencials són incorrectes', async () => {
      setupMocks({
        success: false,
        error: { code: 401, message: 'Credencials incorrectes' },
      }, 401)

      const result = await authLogin('wronguser', 'wrongpassword')
      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.code).toBe(401)
      }
    })
  })

  describe('authRefresh', () => {
    it('renova el token correctament', async () => {
      setupMocks({
        success: true,
        data: { token: 'new-refreshed-token' },
      })

      const result = await authRefresh()
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.token).toBe('new-refreshed-token')
      }
    })
  })

  describe('authMe', () => {
    it('obté informació de l\'usuari actual', async () => {
      setupMocks({
        success: true,
        data: {
          userId: 'user-1',
          username: 'testuser',
          isAdmin: false,
          devices: [],
          quotas: { maxServers: 10, maxChannelsPerServer: 50, maxMessagesPerMinute: 30 },
        },
      })

      const result = await authMe()
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.username).toBe('testuser')
      }
    })
  })

  describe('servers', () => {
    it('llista servidors', async () => {
      setupMocks({
        success: true,
        data: [
          { serverId: 'srv-1', name: 'Server 1', iconUrl: null, ownerId: 'user-1', memberCount: 3, myRole: 'owner', createdAt: '2026-01-01T00:00:00Z' },
        ],
      })

      const result = await serversList()
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data).toHaveLength(1)
        expect(result.data[0].name).toBe('Server 1')
      }
    })

    it('crea un servidor', async () => {
      setupMocks({
        success: true,
        data: { serverId: 'srv-1', name: 'New Server', iconUrl: null, ownerId: 'user-1', memberCount: 1, myRole: 'owner', createdAt: '2026-01-01T00:00:00Z' },
      })

      const result = await serversCreate('New Server')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.name).toBe('New Server')
      }
    })

    it('obté info d\'un servidor', async () => {
      setupMocks({
        success: true,
        data: { serverId: 'srv-1', name: 'Test Server', iconUrl: null, ownerId: 'user-1', memberCount: 2, myRole: 'admin', createdAt: '2026-01-01T00:00:00Z' },
      })

      const result = await serversGet('srv-1')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.serverId).toBe('srv-1')
      }
    })

    it('elimina un servidor', async () => {
      setupMocks({
        success: true,
        data: { deleted: true },
      })

      const result = await serversDelete('srv-1')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.deleted).toBe(true)
      }
    })
  })

  describe('channel key metadata', () => {
    it('mapeja bundles signats amb keyVersionId', async () => {
      setupMocks({
        success: true,
        data: {
          deviceId: 'dev-1',
          keyVersionId: 'ver-1',
          keyVersion: 2,
          encryptedKey: 'encrypted-key',
          kemCiphertext: 'kem-ciphertext',
          signature: 'sig',
          signedByDeviceId: 'dev-signer',
        },
      })

      const result = await channelGetKey('ch-1')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.keyVersionId).toBe('ver-1')
        expect(result.data.signedByDeviceId).toBe('dev-signer')
        expect(result.data.signature).toBe('sig')
      }
    })

    it('mapeja dispositius membres amb claus KEM i DSA', async () => {
      setupMocks({
        success: true,
        data: [{
          deviceId: 'dev-1',
          kemPublicKey: 'kem-public',
          dsaPublicKey: 'dsa-public',
        }],
      })

      const result = await channelGetMemberDevices('ch-1')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data[0].publicKey).toBe('kem-public')
        expect(result.data[0].kemPublicKey).toBe('kem-public')
        expect(result.data[0].dsaPublicKey).toBe('dsa-public')
      }
    })
  })

  describe('channel invitations', () => {
    it('envia invitacions de canal amb només username', async () => {
      setupMocks({
        success: true,
        data: { invited_user: 'alice' },
      })

      const result = await channelInvite('ch-1', 'alice')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.invitedUser).toBe('alice')
      }

      const requestBody = JSON.parse(String(mockFetch.mock.calls[0]?.[1]?.body ?? '{}')) as { username?: string }
      expect(requestBody).toEqual({ username: 'alice' })
    })
  })

  describe('messages', () => {
    it('llista missatges amb paginació', async () => {
      setupMocks({
        success: true,
        data: {
          data: [
            {
              messageId: 'msg-1',
              channelId: 'ch-1',
              senderUserId: 'user-1',
              senderUsername: 'testuser',
              senderDeviceId: 'dev-1',
              encryptedPayload: 'encrypted-data',
              iv: 'iv-data',
              timestamp: '2026-01-01T00:00:00Z',
              expiresAt: null,
              editedAt: null,
              deletedAt: null,
            },
          ],
          pagination: { has_more: false, next_cursor: null, prev_cursor: null },
        },
      })

      const result = await messagesList('ch-1', 50)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.data).toHaveLength(1)
        expect(result.data.data[0].senderUsername).toBe('testuser')
      }
    })

    it('envia un missatge', async () => {
      setupMocks({
        success: true,
        data: {
          messageId: 'msg-1',
          channelId: 'ch-1',
          senderUserId: 'user-1',
          senderUsername: 'testuser',
          senderDeviceId: 'dev-1',
          encryptedPayload: 'encrypted-data',
          iv: 'iv-data',
          timestamp: '2026-01-01T00:00:00Z',
          expiresAt: null,
          editedAt: null,
          deletedAt: null,
        },
      })

      const result = await messagesSend('ch-1', 'encrypted-payload', 'iv-value')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.messageId).toBe('msg-1')
      }
    })

    it('edita un missatge', async () => {
      setupMocks({
        success: true,
        data: {
          messageId: 'msg-1',
          channelId: 'ch-1',
          senderUserId: 'user-1',
          senderUsername: 'testuser',
          senderDeviceId: 'dev-1',
          encryptedPayload: 'new-encrypted',
          iv: 'new-iv',
          timestamp: '2026-01-01T00:00:00Z',
          expiresAt: null,
          editedAt: '2026-01-01T01:00:00Z',
          deletedAt: null,
        },
      })

      const result = await messagesEdit('msg-1', 'new-encrypted', 'new-iv')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.editedAt).toBe('2026-01-01T01:00:00Z')
      }
    })

    it('elimina un missatge', async () => {
      setupMocks({
        success: true,
        data: { deletedAt: '2026-01-01T01:00:00Z' },
      })

      const result = await messagesDelete('msg-1')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.deletedAt).toBeDefined()
      }
    })

    it('obté un missatge concret', async () => {
      setupMocks({
        success: true,
        data: {
          messageId: 'msg-1',
          channelId: 'ch-1',
          senderUserId: 'user-1',
          senderUsername: 'testuser',
          senderDeviceId: 'dev-1',
          encryptedPayload: 'data',
          iv: 'iv',
          timestamp: '2026-01-01T00:00:00Z',
          expiresAt: null,
          editedAt: null,
          deletedAt: null,
        },
      })

      const result = await messagesGet('msg-1')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.messageId).toBe('msg-1')
      }
    })

    it('check missatges nous', async () => {
      setupMocks({
        success: true,
        data: {
          channelId: 'ch-1',
          hasNew: true,
          newCount: 5,
          firstNewMessageId: 'msg-5',
          lastSeen: '2026-01-01T00:00:00Z',
        },
      })

      const result = await messagesCheckNew('ch-1', '2026-01-01T00:00:00Z')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.hasNew).toBe(true)
        expect(result.data.newCount).toBe(5)
      }
    })
  })

  describe('direct messages', () => {
    it('envia un missatge directe', async () => {
      setupMocks({
        success: true,
        data: {
          messageId: 'dm-1',
          senderUserId: 'user-1',
          recipientUserId: 'user-2',
          encryptedPayload: 'encrypted',
          iv: 'iv',
          timestamp: '2026-01-01T00:00:00Z',
          isDirect: true,
          deletedAt: null,
        },
      })

      const result = await dmSend('user-2', 'encrypted', 'iv')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.recipientUserId).toBe('user-2')
        expect(result.data.isDirect).toBe(true)
      }
    })

    it('obre un canal DM v2', async () => {
      setupMocks({
        success: true,
        data: {
          dm_channel_id: 'dm-ch-1',
          peer_user_id: 'user-2',
          peer_username: 'marcus',
          message_ttl: 3600,
          key_version_id: 'kv-1',
          key_version: 1,
          created: true,
        },
      })

      const result = await dmChannelOpen('user-2', 3600)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.dmChannelId).toBe('dm-ch-1')
        expect(result.data.peerUsername).toBe('marcus')
        expect(result.data.created).toBe(true)
      }
    })

    it('llista canals DM v2', async () => {
      setupMocks({
        success: true,
        data: [
          {
            dm_channel_id: 'dm-ch-1',
            peer_user_id: 'user-2',
            peer_username: 'marcus',
            message_ttl: 3600,
            unread_count: 2,
            last_message_at: '2026-01-01T00:00:00Z',
          },
        ],
      })

      const result = await dmChannelsList()
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data).toHaveLength(1)
        expect(result.data[0].dmChannelId).toBe('dm-ch-1')
        expect(result.data[0].unreadCount).toBe(2)
      }
    })

    it('envia missatge per DM v2 channel', async () => {
      setupMocks({
        success: true,
        data: {
          id: 'msg-1',
          channel_id: 'dm-ch-1',
          sender_user_id: 'user-1',
          sender_username: 'agusti',
          sender_device_id: 'dev-1',
          encrypted_payload: 'encrypted',
          iv: 'iv',
          timestamp: '2026-01-01T00:00:00Z',
          expires_at: null,
          edited_at: null,
          deleted_at: null,
        },
      })

      const result = await dmMessagesSend('dm-ch-1', 'encrypted', 'iv')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.channelId).toBe('dm-ch-1')
      }
    })

    it('actualitza settings de DM v2', async () => {
      setupMocks({
        success: true,
        data: {
          dm_channel_id: 'dm-ch-1',
          message_ttl: 1800,
        },
      })

      const result = await dmChannelUpdateSettings('dm-ch-1', 1800)
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.dmChannelId).toBe('dm-ch-1')
        expect(result.data.messageTTL).toBe(1800)
      }
    })

    it('rota clau de DM v2', async () => {
      setupMocks({
        success: true,
        data: {
          dm_channel_id: 'dm-ch-1',
          key_version_id: 'kv-2',
          key_version: 2,
        },
      })

      const result = await dmChannelRotateKey('dm-ch-1')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.dmChannelId).toBe('dm-ch-1')
        expect(result.data.keyVersionId).toBe('kv-2')
        expect(result.data.keyVersion).toBe(2)
      }
    })
  })

  describe('channels', () => {
    it('llista canals d\'un servidor', async () => {
      setupMocks({
        success: true,
        data: [
          {
            channelId: 'ch-1',
            name: 'general',
            type: 'text',
            encryptionType: 'none',
            messageTTL: null,
            isPrivate: false,
            createdAt: '2026-01-01T00:00:00Z',
          },
        ],
      })

      const result = await channelsList('srv-1')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data).toHaveLength(1)
        expect(result.data[0].name).toBe('general')
      }
    })

    it('crea un canal', async () => {
      setupMocks({
        success: true,
        data: {
          channelId: 'ch-1',
          name: 'novo-canal',
          type: 'text',
          encryptionType: 'none',
          messageTTL: null,
          isPrivate: false,
          createdAt: '2026-01-01T00:00:00Z',
        },
      })

      const result = await channelsCreate('srv-1', 'novo-canal', 'text')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.name).toBe('novo-canal')
      }
    })

    it('obté claus de canal', async () => {
      setupMocks({
        success: true,
        data: [{ keyId: 'key-1', deviceId: 'dev-1', encryptedKey: 'enc-key', kemCiphertext: 'kem-ct', encryptionType: 'asymmetric', createdAt: '2026-01-01T00:00:00Z' }],
      })

      const result = await channelsGetKeys('ch-1')
      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data).toHaveLength(1)
      }
    })
  })

  describe('token management', () => {
    it('no inclou Authorization sense token', async () => {
      mockSessionStorage['chillgroup-token'] = ''
      setupMocks({ success: true, data: {} })

      await authMe()

      const headers = mockFetch.mock.calls?.[0]?.[1]?.headers as Record<string, string>
      expect(headers?.['Authorization']).toBeUndefined()
    })
  })
})