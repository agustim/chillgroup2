//! Client HTTP per a ChillGroup v2 API.
//!
//! Wrapper sobre fetch per a crides a l'API REST.

const API_BASE = import.meta.env.VITE_API_BASE ?? ''

/**
 * Interfície per a errors de l'API.
 */
export interface ApiError {
  success: false
  error: {
    code: number
    message: string
    details?: Record<string, string>
  }
}

/**
 * Resposta exitosa de l'API.
 */
export interface ApiResponse<T = unknown> {
  success: true
  data: T
}

export type ApiResult<T> = ApiResponse<T> | ApiError

/**
 * Obtenir el token JWT des de sessionStorage.
 */
function getToken(): string | null {
  try {
    return sessionStorage.getItem('chillgroup-token')
  } catch {
    return null
  }
}

/**
 * Fer una crida HTTP a l'API.
 */
async function apiRequest<T>(
  method: string,
  path: string,
  body?: unknown
): Promise<ApiResult<T>> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  }

  const token = getToken()
  if (token) {
    headers['Authorization'] = `Bearer ${token}`
  }

  const options: RequestInit = {
    method,
    headers,
  }

  if (body !== undefined) {
    options.body = JSON.stringify(body)
  }

  const response = await fetch(`${API_BASE}${path}`, options)

  let data: any = {}
  try {
    data = await response.json()
  } catch {
    // If JSON parsing fails, create a generic error
    data = { success: false, error: { code: response.status, message: 'Network error' } }
  }

  if (response.ok) {
    if (data?.success === true) {
      return { success: true, data: data.data ?? data } as ApiResponse<T>
    }

    if (data?.success === false) {
      return {
        success: false,
        error: {
          code: data?.error?.code || response.status,
          message: data?.error?.message || `HTTP ${response.status}`,
        },
      } as ApiError
    }

    return { success: true, data: data as T } as ApiResponse<T>
  }

  return {
    success: false,
    error: {
      code: data?.error?.code || response.status,
      message: data?.error?.message || `HTTP ${response.status}`,
    },
  } as ApiError
}

// ── Types ─────────────────────────────────────────────────────

export interface UserInfo {
  userId: string
  username: string
  isAdmin: boolean
  devices: DeviceInfo[]
  quotas: Quotas
}

export interface DeviceInfo {
  deviceId: string
  label: string
  publicKey: string
  lastSeen: string
  revoked: boolean
}

export interface Quotas {
  maxServers: number
  maxChannelsPerServer: number
  maxMessagesPerMinute: number
}

export interface ServerInfo {
  serverId: string
  name: string
  iconUrl: string | null
  ownerId: string
  memberCount: number
  myRole: string
  createdAt: string
}

export interface Message {
  messageId: string
  channelId: string
  senderUserId: string
  senderUsername: string
  senderDeviceId: string
  encryptedPayload: string
  iv: string
  timestamp: string
  expiresAt: string | null
  editedAt: string | null
  deletedAt: string | null
}

export interface PaginatedMessages {
  data: Message[]
  pagination: {
    has_more: boolean
    next_cursor: string | null
    prev_cursor: string | null
    total_new?: number
  }
}

export interface DirectMessage {
  messageId: string
  senderUserId: string
  recipientUserId: string
  encryptedPayload: string
  iv: string
  timestamp: string
  isDirect: boolean
  deletedAt: string | null
}

export interface ChannelInfo {
  channelId: string
  name: string
  type: 'text' | 'voice'
  encryptionType: 'none' | 'symmetric' | 'asymmetric'
  messageTTL: number | null
  isPrivate: boolean
  createdAt: string
}

// ── API Functions ─────────────────────────────────────────────

export async function authRegister(username: string, password: string) {
  return apiRequest<{
    userId: string
    username: string
    token: string
    deviceId: string
    deviceLabel: string
  }>('POST', '/api/auth/register', { username, password })
}

export async function authLogin(username: string, password: string) {
  return apiRequest<{
    userId: string
    username: string
    token: string
    deviceId: string
    deviceLabel: string
    isAdmin: boolean
  }>('POST', '/api/auth/login', { username, password })
}

export async function authRefresh() {
  return apiRequest<{ token: string }>('POST', '/api/auth/refresh')
}

export async function authMe() {
  return apiRequest<UserInfo>('GET', '/api/user/me')
}

export async function serversList() {
  return apiRequest<ServerInfo[]>('GET', '/api/servers')
}

export async function serversCreate(name: string, iconUrl?: string | null) {
  return apiRequest<ServerInfo>('POST', '/api/servers', { name, iconUrl })
}

export async function serversGet(serverId: string) {
  return apiRequest<ServerInfo>('GET', `/api/servers/${serverId}`)
}

export async function serversDelete(serverId: string) {
  return apiRequest<{ deleted: boolean }>('DELETE', `/api/servers/${serverId}`)
}

export async function messagesList(channelId: string, limit = 50, before?: string) {
  const params = new URLSearchParams({ limit: String(limit) })
  if (before) params.set('before', before)
  return apiRequest<PaginatedMessages>(
    'GET',
    `/api/channels/${channelId}/messages?${params}`
  )
}

export async function messagesSend(
  channelId: string,
  encryptedPayload: string,
  iv: string,
  expiresAt?: string
) {
  return apiRequest<Message>('POST', `/api/channels/${channelId}/messages`, {
    encryptedPayload,
    iv,
    expiresAt,
  })
}

export async function messagesEdit(messageId: string, encryptedPayload: string, iv: string) {
  return apiRequest<Message>('PUT', `/api/messages/${messageId}`, {
    encryptedPayload,
    iv,
  })
}

export async function messagesDelete(messageId: string) {
  return apiRequest<{ deletedAt: string }>('DELETE', `/api/messages/${messageId}`)
}

export async function messagesGet(messageId: string) {
  return apiRequest<Message>('GET', `/api/messages/${messageId}`)
}

export async function messagesCheckNew(channelId: string, lastSeen: string) {
  return apiRequest<{
    channelId: string
    hasNew: boolean
    newCount: number
    firstNewMessageId: string | null
    lastSeen: string
  }>('GET', `/api/channels/${channelId}/messages/check-new?last_seen=${lastSeen}`)
}

export async function dmSend(recipientUserId: string, encryptedPayload: string, iv: string) {
  return apiRequest<DirectMessage>('POST', '/api/direct-messages', {
    encryptedPayload,
    iv,
    isDirect: true,
    recipientUserId,
  })
}

export async function channelsList(serverId: string) {
  return apiRequest<ChannelInfo[]>('GET', `/api/servers/${serverId}/channels`)
}

export async function channelsCreate(
  serverId: string,
  name: string,
  type: 'text' | 'voice',
  encryptionType = 'none'
) {
  return apiRequest<ChannelInfo>('POST', `/api/servers/${serverId}/channels`, {
    name,
    type,
    encryptionType,
    isPrivate: false,
  })
}

export async function channelsGetKeys(channelId: string) {
  return apiRequest<any[]>('GET', `/api/channels/${channelId}/keys`)
}