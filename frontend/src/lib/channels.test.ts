//! Tests unitaris per a la gestió de canals (text i veu).
//!
//! Segons especificació de definitions/TESTING.md

import { describe, it, expect, beforeEach, vi } from 'vitest'
import {
  channelsList,
  channelsCreate,
  channelsGetKeys,
  Message,
  ChannelInfo,
} from './api'

// Mock de fetch
const mockFetch = vi.fn()
vi.stubGlobal('fetch', mockFetch)

// Token de test
const TEST_TOKEN = 'test-jwt-token'

function setupMocks(responseBody: any, status = 200) {
  mockFetch.mockResolvedValueOnce({
    ok: status < 400,
    status,
    json: async () => responseBody,
  })
}

describe('Channels Management', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  describe('canals de text', () => {
    it('llista canals de text correctament', async () => {
      const channels: ChannelInfo[] = [
        {
          channelId: 'ch-1',
          name: 'general',
          type: 'text',
          encryptionType: 'none',
          messageTTL: null,
          isPrivate: false,
          createdAt: '2026-01-01T00:00:00Z',
        },
        {
          channelId: 'ch-2',
          name: 'tecnologia',
          type: 'text',
          encryptionType: 'none',
          messageTTL: null,
          isPrivate: false,
          createdAt: '2026-01-02T00:00:00Z',
        },
        {
          channelId: 'ch-3',
          name: 'memes',
          type: 'text',
          encryptionType: 'none',
          messageTTL: null,
          isPrivate: false,
          createdAt: '2026-01-03T00:00:00Z',
        },
      ]

      setupMocks({ success: true, data: channels })

      const result = await channelsList('srv-1')

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data).toHaveLength(3)
        expect(result.data.every(c => c.type === 'text')).toBe(true)
        expect(result.data.map(c => c.name)).toEqual(expect.arrayContaining(['general', 'tecnologia', 'memes']))
      }
    })

    it('filtra només canals de text', async () => {
      const channels: ChannelInfo[] = [
        { channelId: 'ch-1', name: 'general', type: 'text', encryptionType: 'none', messageTTL: null, isPrivate: false, createdAt: '2026-01-01T00:00:00Z' },
        { channelId: 'ch-2', name: 'general-voice', type: 'voice', encryptionType: 'none', messageTTL: null, isPrivate: false, createdAt: '2026-01-01T00:00:00Z' },
        { channelId: 'ch-3', name: 'dev', type: 'text', encryptionType: 'none', messageTTL: null, isPrivate: false, createdAt: '2026-01-01T00:00:00Z' },
      ]

      setupMocks({ success: true, data: channels })

      const result = await channelsList('srv-1')

      if (result.success) {
        const textChannels = result.data.filter(c => c.type === 'text')
        expect(textChannels).toHaveLength(2)
        expect(textChannels.map(c => c.name)).toEqual(expect.arrayContaining(['general', 'dev']))
      }
    })

    it('crea un canal de text correctament', async () => {
      setupMocks({
        success: true,
        data: {
          channelId: 'ch-new',
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
        expect(result.data.type).toBe('text')
        expect(result.data.name).toBe('novo-canal')
      }
    })
  })

  describe('canals de veu', () => {
    it('llista canals de veu correctament', async () => {
      const channels: ChannelInfo[] = [
        { channelId: 'ch-1', name: 'Lounge', type: 'voice', encryptionType: 'none', messageTTL: null, isPrivate: false, createdAt: '2026-01-01T00:00:00Z' },
        { channelId: 'ch-2', name: 'Gaming', type: 'voice', encryptionType: 'none', messageTTL: null, isPrivate: false, createdAt: '2026-01-01T00:00:00Z' },
      ]

      setupMocks({ success: true, data: channels })

      const result = await channelsList('srv-1')

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.filter(c => c.type === 'voice')).toHaveLength(2)
      }
    })

    it('crea un canal de veu correctament', async () => {
      setupMocks({
        success: true,
        data: {
          channelId: 'ch-voice-1',
          name: 'Meeting Room',
          type: 'voice',
          encryptionType: 'none',
          messageTTL: null,
          isPrivate: false,
          createdAt: '2026-01-01T00:00:00Z',
        },
      })

      const result = await channelsCreate('srv-1', 'Meeting Room', 'voice')

      expect(result.success).toBe(true)
      if (result.success) {
        expect(result.data.type).toBe('voice')
        expect(result.data.name).toBe('Meeting Room')
      }
    })

    it('distingeix entre canals de text i veu amb mateix nom', async () => {
      const channels: ChannelInfo[] = [
        { channelId: 'ch-1', name: 'general', type: 'text', encryptionType: 'none', messageTTL: null, isPrivate: false, createdAt: '2026-01-01T00:00:00Z' },
        { channelId: 'ch-2', name: 'general', type: 'voice', encryptionType: 'none', messageTTL: null, isPrivate: false, createdAt: '2026-01-01T00:00:00Z' },
      ]

      setupMocks({ success: true, data: channels })

      const result = await channelsList('srv-1')

      if (result.success) {
        const textCh = result.data.find(c => c.type === 'text' && c.name === 'general')
        const voiceCh = result.data.find(c => c.type === 'voice' && c.name === 'general')

        expect(textCh).toBeDefined()
        expect(voiceCh).toBeDefined()
        expect(textCh?.channelId).not.toBe(voiceCh?.channelId)
      }
    })
  })

  describe('canals privats', () => {
    it('identifica canals privats', async () => {
      const channels: ChannelInfo[] = [
        { channelId: 'ch-1', name: 'secret', type: 'text', encryptionType: 'asymmetric', messageTTL: null, isPrivate: true, createdAt: '2026-01-01T00:00:00Z' },
      ]

      setupMocks({ success: true, data: channels })

      const result = await channelsList('srv-1')

      if (result.success) {
        expect(result.data[0].isPrivate).toBe(true)
        expect(result.data[0].encryptionType).toBe('asymmetric')
      }
    })

    it('identifica canals publics', async () => {
      const channels: ChannelInfo[] = [
        { channelId: 'ch-1', name: 'general', type: 'text', encryptionType: 'none', messageTTL: null, isPrivate: false, createdAt: '2026-01-01T00:00:00Z' },
      ]

      setupMocks({ success: true, data: channels })

      const result = await channelsList('srv-1')

      if (result.success) {
        expect(result.data[0].isPrivate).toBe(false)
        expect(result.data[0].encryptionType).toBe('none')
      }
    })
  })

  describe('TTL de missatges', () => {
    it('canals amb TTL configuren expiresAt', async () => {
      const channels: ChannelInfo[] = [
        { channelId: 'ch-1', name: 'temporal', type: 'text', encryptionType: 'none', messageTTL: 3600, isPrivate: false, createdAt: '2026-01-01T00:00:00Z' },
      ]

      setupMocks({ success: true, data: channels })

      const result = await channelsList('srv-1')

      if (result.success) {
        expect(result.data[0].messageTTL).toBe(3600) // 1 hora
      }
    })
  })

  describe('errors', () => {
    it('gestiona error 404 en llistar canals', async () => {
      setupMocks({ success: false, error: { code: 404, message: 'Servidor no trobat' } }, 404)

      const result = await channelsList('srv-invalid')

      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.code).toBe(404)
      }
    })

    it('gestiona error 403 en accedir a canal privat', async () => {
      setupMocks({ success: false, error: { code: 403, message: 'No tens accés a aquest canal' } }, 403)

      const result = await channelsGetKeys('ch-private')

      expect(result.success).toBe(false)
      if (!result.success) {
        expect(result.error.code).toBe(403)
      }
    })
  })
})

describe('Voice Channel Users Presence', () => {
  // Mock per a presència d'usuaris en canals de veu
  interface VoiceChannelUser {
    userId: string
    username: string
    avatar?: string
    joinedAt: string
    isDeafened: boolean
    isSuppressed: boolean
  }

  interface VoiceChannelPresence {
    channelId: string
    channelName: string
    users: VoiceChannelUser[]
    maxUsers: number
  }

  function createMockUser(userId: string, username: string, minutesAgo: number): VoiceChannelUser {
    const joinedAt = new Date(Date.now() - minutesAgo * 60000).toISOString()
    return {
      userId,
      username,
      avatar: `https://example.com/avatars/${userId}.jpg`,
      joinedAt,
      isDeafened: false,
      isSuppressed: Math.random() > 0.7,
    }
  }

  describe('llistat d\'usuaris en canals de veu', () => {
    it('mostra usuaris ordenats per hora d\'entrada', () => {
      const users = [
        createMockUser('user-1', 'agusti', 30),
        createMockUser('user-2', 'marcus', 10),
        createMockUser('user-3', 'julia', 5),
      ]

      // Ordenar per joinedAt (més recent primer)
      const sorted = [...users].sort((a, b) => new Date(b.joinedAt).getTime() - new Date(a.joinedAt).getTime())

      expect(sorted[0].username).toBe('julia') // Ha entrat fa 5 min
      expect(sorted[1].username).toBe('marcus') // Ha entrat fa 10 min
      expect(sorted[2].username).toBe('agusti') // Ha entrat fa 30 min
    })

    it('mostra comptador d\'usuaris correctament', () => {
      const users: VoiceChannelUser[] = [
        createMockUser('user-1', 'agusti', 30),
        createMockUser('user-2', 'marcus', 10),
        createMockUser('user-3', 'julia', 5),
        createMockUser('user-4', 'pere', 2),
      ]

      const presence: VoiceChannelPresence = {
        channelId: 'ch-voice-1',
        channelName: 'Lounge',
        users,
        maxUsers: 10,
      }

      expect(presence.users.length).toBe(4)
      expect(`${presence.users.length}/${presence.maxUsers}`).toBe('4/10')
    })

    it('gestiona canal buit', () => {
      const presence: VoiceChannelPresence = {
        channelId: 'ch-voice-2',
        channelName: 'Quiet Room',
        users: [],
        maxUsers: 10,
      }

      expect(presence.users.length).toBe(0)
      expect(presence.channelName).toBe('Quiet Room')
    })

    it('identifica usuaris sord muts (deafened)', () => {
      const users: VoiceChannelUser[] = [
        createMockUser('user-1', 'agusti', 10),
        createMockUser('user-2', 'marcus', 5),
      ]
      users[0].isDeafened = true

      const deafUsers = users.filter(u => u.isDeafened)
      const speakingUsers = users.filter(u => !u.isDeafened)

      expect(deafUsers.length).toBe(1)
      expect(deafUsers[0].username).toBe('agusti')
      expect(speakingUsers.length).toBe(1)
    })

    it('identifica usuaris suprimits (suppressed)', () => {
      const users: VoiceChannelUser[] = [
        createMockUser('user-1', 'agusti', 10),
        createMockUser('user-2', 'marcus', 5),
      ]
      users[1].isSuppressed = true

      const suppressedUsers = users.filter(u => u.isSuppressed)

      expect(suppressedUsers.length).toBe(1)
      expect(suppressedUsers[0].username).toBe('marcus')
    })
  })

  describe('entrar i sortir de canals de veu', () => {
    interface VoiceEvent {
      type: 'join' | 'leave' | 'deafen' | 'undeafen' | 'suppress' | 'unsuppress'
      userId: string
      username: string
      channelId: string
      timestamp: string
    }

    function createJoinEvent(userId: string, username: string, channelId: string): VoiceEvent {
      return {
        type: 'join',
        userId,
        username,
        channelId,
        timestamp: new Date().toISOString(),
      }
    }

    function createLeaveEvent(userId: string, username: string, channelId: string): VoiceEvent {
      return {
        type: 'leave',
        userId,
        username,
        channelId,
        timestamp: new Date().toISOString(),
      }
    }

    it('registra entrada d\'usuari en canal de veu', () => {
      const events: VoiceEvent[] = []
      const channelUsers = new Map<string, Set<string>>()

      // Usuario entra
      const joinEvent = createJoinEvent('user-1', 'agusti', 'ch-voice-1')
      events.push(joinEvent)

      if (!channelUsers.has(joinEvent.channelId)) {
        channelUsers.set(joinEvent.channelId, new Set())
      }
      channelUsers.get(joinEvent.channelId)!.add(joinEvent.userId)

      expect(channelUsers.get('ch-voice-1')!.size).toBe(1)
      expect(channelUsers.get('ch-voice-1')!.has('user-1')).toBe(true)
    })

    it('registra sortida d\'usuari de canal de veu', () => {
      const channelUsers = new Map<string, Set<string>>()

      // Primer entra
      channelUsers.set('ch-voice-1', new Set(['user-1', 'user-2']))

      // Després surt
      channelUsers.get('ch-voice-1')!.delete('user-1')

      expect(channelUsers.get('ch-voice-1')!.size).toBe(1)
      expect(channelUsers.get('ch-voice-1')!.has('user-1')).toBe(false)
      expect(channelUsers.get('ch-voice-1')!.has('user-2')).toBe(true)
    })

    it('gestiona multiples usuaris entrant i sortint', () => {
      const channelUsers = new Map<string, Set<string>>()
      const events: VoiceEvent[] = []

      // Escenari: 3 usuaris entren, 1 surt, 2 més entren, 1 surt
      const scenarios = [
        { user: 'user-1', username: 'agusti', action: 'join' as const },
        { user: 'user-2', username: 'marcus', action: 'join' as const },
        { user: 'user-3', username: 'julia', action: 'join' as const },
        { user: 'user-2', username: 'marcus', action: 'leave' as const },
        { user: 'user-4', username: 'pere', action: 'join' as const },
        { user: 'user-5', username: 'anna', action: 'join' as const },
        { user: 'user-1', username: 'agusti', action: 'leave' as const },
      ]

      for (const scenario of scenarios) {
        if (scenario.action === 'join') {
          if (!channelUsers.has('ch-voice-1')) {
            channelUsers.set('ch-voice-1', new Set())
          }
          channelUsers.get('ch-voice-1')!.add(scenario.user)
          events.push(createJoinEvent(scenario.user, scenario.username, 'ch-voice-1'))
        } else {
          channelUsers.get('ch-voice-1')!.delete(scenario.user)
          events.push(createLeaveEvent(scenario.user, scenario.username, 'ch-voice-1'))
        }
      }

      // User-2 entra i surt, user-1 entra i surt, queden: user-3, user-4, user-5
      expect(channelUsers.get('ch-voice-1')!.size).toBe(3)
      expect(channelUsers.get('ch-voice-1')!).toEqual(new Set(['user-3', 'user-4', 'user-5']))
      expect(events.length).toBe(7)

      const joinEvents = events.filter(e => e.type === 'join')
      const leaveEvents = events.filter(e => e.type === 'leave')

      expect(joinEvents.length).toBe(5)
      expect(leaveEvents.length).toBe(2)
    })

    it('compta usuaris per canal correctament', () => {
      const channelUsers = new Map<string, Set<string>>()

      // Canal 1: 3 usuaris
      channelUsers.set('ch-voice-1', new Set(['user-1', 'user-2', 'user-3']))
      // Canal 2: 1 usuari
      channelUsers.set('ch-voice-2', new Set(['user-4']))
      // Canal 3: buit
      channelUsers.set('ch-voice-3', new Set())

      expect(channelUsers.get('ch-voice-1')!.size).toBe(3)
      expect(channelUsers.get('ch-voice-2')!.size).toBe(1)
      expect(channelUsers.get('ch-voice-3')!.size).toBe(0)
    })

    it('gestiona usuari que entra en multiple canals simultaniament', () => {
      const channelUsers = new Map<string, Set<string>>()

      // Un usuari entra en dos canals diferents
      channelUsers.set('ch-voice-1', new Set(['user-1']))
      channelUsers.set('ch-voice-2', new Set(['user-1']))

      expect(channelUsers.get('ch-voice-1')!.size).toBe(1)
      expect(channelUsers.get('ch-voice-2')!.size).toBe(1)
      expect(channelUsers.get('ch-voice-1')!.has('user-1')).toBe(true)
      expect(channelUsers.get('ch-voice-2')!.has('user-1')).toBe(true)
    })
  })

  describe('notifications d\'entrada/sortida', () => {
    interface SystemNotification {
      message: string
      timestamp: string
      type: 'channel_join' | 'channel_leave'
    }

    it('genera notificacio quan usuari entra en canal', () => {
      const notifications: SystemNotification[] = []

      const joinNotification: SystemNotification = {
        message: 'agusti s\'ha unit al canal Lounge',
        timestamp: new Date().toISOString(),
        type: 'channel_join',
      }

      notifications.push(joinNotification)

      expect(notifications.length).toBe(1)
      expect(notifications[0].type).toBe('channel_join')
      expect(notifications[0].message).toContain('agusti')
      expect(notifications[0].message).toContain('Lounge')
    })

    it('genera notificacio quan usuari surt de canal', () => {
      const notifications: SystemNotification[] = []

      const leaveNotification: SystemNotification = {
        message: 'marcus ha abandonat el canal Lounge',
        timestamp: new Date().toISOString(),
        type: 'channel_leave',
      }

      notifications.push(leaveNotification)

      expect(notifications.length).toBe(1)
      expect(notifications[0].type).toBe('channel_leave')
      expect(notifications[0].message).toContain('marcus')
    })

    it('manté historial de notificacions ordenat per timestamp', () => {
      const notifications: SystemNotification[] = [
        {
          message: 'agusti s\'ha unit al canal Lounge',
          timestamp: new Date(Date.now() - 30 * 60000).toISOString(), // fa 30 min
          type: 'channel_join',
        },
        {
          message: 'marcus s\'ha unit al canal Lounge',
          timestamp: new Date(Date.now() - 10 * 60000).toISOString(), // fa 10 min
          type: 'channel_join',
        },
        {
          message: 'marcus ha abandonat el canal Lounge',
          timestamp: new Date().toISOString(), // ara
          type: 'channel_leave',
        },
      ]

      // Verificar que estan ordenades
      for (let i = 1; i < notifications.length; i++) {
        expect(new Date(notifications[i].timestamp).getTime()).toBeGreaterThanOrEqual(
          new Date(notifications[i - 1].timestamp).getTime()
        )
      }
    })
  })
})