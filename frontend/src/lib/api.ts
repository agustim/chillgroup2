//! Client HTTP per a ChillGroup v2 API.
//!
//! Wrapper sobre fetch per a crides a l'API REST.

import type { Server, ServerFullInfo, ServerMember, ServerRole, Channel } from '../types'

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

function mapServer(server: any): Server {
  return {
    serverId: server.server_id ?? server.serverId,
    name: server.name,
    iconUrl: server.icon_url ?? server.iconUrl ?? null,
    ownerId: server.owner_id ?? server.ownerId,
    memberCount: server.member_count ?? server.memberCount,
    myRole: server.my_role ?? server.myRole ?? 'member',
    createdAt: server.created_at ?? server.createdAt,
  }
}

function mapServerMember(member: any) {
  return {
    userId: member.user_id ?? member.userId,
    username: member.username,
    role: member.role,
    joinedAt: member.joined_at ?? member.joinedAt,
  }
}

function mapServerFullInfo(server: any): ServerFullInfo {
  return {
    serverId: server.server_id ?? server.serverId,
    name: server.name,
    iconUrl: server.icon_url ?? server.iconUrl ?? null,
    ownerId: server.owner_id ?? server.ownerId,
    memberCount: server.member_count ?? server.memberCount ?? (server.members?.length ?? 0),
    myRole: server.my_role ?? server.myRole ?? 'member',
    members: (server.members ?? []).map(mapServerMember),
    createdAt: server.created_at ?? server.createdAt,
  }
}

function mapChannel(channel: any): ChannelInfo {
  return {
    channelId: channel.channel_id ?? channel.channelId,
    name: channel.name,
    type: channel.channel_type ?? channel.type,
    encryptionType: channel.encryption_type ?? channel.encryptionType,
    messageTTL: channel.message_ttl ?? channel.messageTTL ?? null,
    isPrivate: channel.is_private ?? channel.isPrivate ?? false,
    createdAt: channel.created_at ?? channel.createdAt,
  }
}

function mapInviteResponse(response: any) {
  return {
    invitedUser: response.invited_user,
  }
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

export async function serversList(): Promise<ApiResult<Server[]>> {
  const result = await apiRequest<any[]>('GET', '/api/servers')
  if (!result.success) return result
  return { success: true, data: result.data.map(mapServer) }
}

export async function serversCreate(name: string, iconUrl?: string | null): Promise<ApiResult<ServerFullInfo>> {
  const result = await apiRequest<any>('POST', '/api/servers', { name, iconUrl })
  if (!result.success) return result
  return { success: true, data: mapServerFullInfo(result.data) }
}

export async function serversGet(serverId: string): Promise<ApiResult<ServerFullInfo>> {
  const result = await apiRequest<any>('GET', `/api/servers/${serverId}`)
  if (!result.success) return result
  return { success: true, data: mapServerFullInfo(result.data) }
}

export async function serversDelete(serverId: string) {
  return apiRequest<{ deleted: boolean }>('DELETE', `/api/servers/${serverId}`)
}

export async function serverMembersList(serverId: string) {
  return apiRequest<ServerMember[]>('GET', `/api/servers/${serverId}/members`)
}

export async function serverInviteMember(serverId: string, username: string): Promise<ApiResult<{ invitedUser: string }>> {
  const result = await apiRequest<any>('POST', `/api/servers/${serverId}/members`, { username })
  if (!result.success) return result
  return { success: true, data: mapInviteResponse(result.data) }
}

export async function serverUpdateMemberRole(serverId: string, userId: string, role: ServerRole) {
  return apiRequest<ServerMember>('PUT', `/api/servers/${serverId}/members/${userId}/role`, { role })
}

export async function messagesList(channelId: string, limit = 50, before?: string) {
  const params = new URLSearchParams({ limit: String(limit) })
  if (before) params.set('before', before)
  const result = await apiRequest<PaginatedMessages>(
    'GET',
    `/api/channels/${channelId}/messages?${params}`
  )
  if (!result.success || !result.data) return result
  return {
    success: true as const,
    data: {
      data: result.data.data.map(mapMessageToTypes),
      pagination: result.data.pagination,
    },
  }
}

export async function messagesSend(
  channelId: string,
  encryptedPayload: string,
  iv: string,
  expiresAt?: string
) {
  const result = await apiRequest<any>('POST', `/api/channels/${channelId}/messages`, {
    encrypted_payload: encryptedPayload,
    iv,
    expires_at: expiresAt,
  })
  if (!result.success || !result.data) return result
  return {
    success: true as const,
    data: mapMessageToTypes(result.data),
  }
}

export async function messagesEdit(messageId: string, encryptedPayload: string, iv: string) {
  const result = await apiRequest<any>('PUT', `/api/messages/${messageId}`, {
    encrypted_payload: encryptedPayload,
    iv,
  })
  if (!result.success || !result.data) return result
  return {
    success: true as const,
    data: mapMessageToTypes(result.data),
  }
}

export async function messagesDelete(messageId: string) {
  return apiRequest<{ deletedAt: string }>('DELETE', `/api/messages/${messageId}`)
}

export async function messagesGet(messageId: string) {
  const result = await apiRequest<any>('GET', `/api/messages/${messageId}`)
  if (!result.success || !result.data) return result
  return {
    success: true as const,
    data: mapMessageToTypes(result.data),
  }
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
    encrypted_payload: encryptedPayload,
    iv,
    is_direct: true,
    recipient_user_id: recipientUserId,
  })
}

export async function channelsList(serverId: string): Promise<ApiResult<Channel[]>> {
  const result = await apiRequest<any[]>('GET', `/api/servers/${serverId}/channels`)
  if (!result.success) return result
  return { success: true, data: result.data.map(mapChannelToTypes) }
}

function mapChannelToTypes(channel: any): Channel {
  return {
    channelId: channel.channel_id ?? channel.channelId,
    name: channel.name,
    type: channel.channel_type ?? channel.type,
    encryptionType: channel.encryption_type ?? channel.encryptionType,
    messageTTL: channel.message_ttl ?? channel.messageTTL ?? null,
    isPrivate: channel.is_private ?? channel.isPrivate ?? false,
    createdAt: channel.created_at ?? channel.createdAt,
  }
}

function mapMessageToTypes(msg: any): Message {
  return {
    messageId: msg.id ?? msg.messageId,
    channelId: msg.channel_id ?? msg.channelId,
    senderUserId: msg.sender_user_id ?? msg.senderUserId,
    senderUsername: msg.sender_username ?? msg.senderUsername,
    senderDeviceId: msg.sender_device_id ?? msg.senderDeviceId,
    encryptedPayload: msg.encrypted_payload ?? msg.encryptedPayload,
    iv: msg.iv,
    timestamp: msg.timestamp,
    expiresAt: msg.expires_at ?? msg.expiresAt ?? null,
    editedAt: msg.edited_at ?? msg.editedAt ?? null,
    deletedAt: msg.deleted_at ?? msg.deletedAt ?? null,
  }
}

export async function channelsCreate(
  serverId: string,
  name: string,
  type: 'text' | 'voice',
  encryptionType = 'none',
  messageTTL: number | null = null
): Promise<ApiResult<Channel>> {
  const result = await apiRequest<any>('POST', `/api/servers/${serverId}/channels`, {
    name,
    channel_type: type,
    encryption_type: encryptionType,
    message_ttl: messageTTL,
    is_private: false,
  })
  if (!result.success) return result
  return { success: true, data: mapChannelToTypes(result.data) }
}

export async function channelsUpdate(
  channelId: string,
  name?: string,
  messageTTL?: number | null,
  isPrivate?: boolean
): Promise<ApiResult<Channel>> {
  const body: Record<string, unknown> = {}
  if (name !== undefined) body.name = name
  if (messageTTL !== undefined) body.message_ttl = messageTTL
  if (isPrivate !== undefined) body.is_private = isPrivate
  const result = await apiRequest<any>('PUT', `/api/channels/${channelId}`, body)
  if (!result.success) return result
  return { success: true, data: mapChannelToTypes(result.data) }
}

export async function channelDelete(channelId: string): Promise<ApiResult<void>> {
  const result = await apiRequest<void>('DELETE', `/api/channels/${channelId}`)
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function channelInvite(channelId: string, username: string): Promise<ApiResult<{ invitedUser: string }>> {
  const result = await apiRequest<any>('POST', `/api/channels/${channelId}/invite`, {
    username,
  })
  if (!result.success) return result
  return { success: true, data: mapInviteResponse(result.data) }
}

export async function channelsGetKeys(channelId: string) {
  return apiRequest<any[]>('GET', `/api/channels/${channelId}/keys`)
}