//! Client HTTP per a ChillGroup v2 API.
//!
//! Wrapper sobre fetch per a crides a l'API REST.

import type { Channel, FriendPresence, PresenceStatus, Server, ServerFullInfo, ServerMember, ServerRole, UserSearchResult } from '../types'
import { getStoredDeviceId } from './device-identity'

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

  if (response.status === 204) {
    return { success: true, data: undefined as T } as ApiResponse<T>
  }

  let data: any = {}
  let responseText = ''
  if (typeof (response as Response & { text?: () => Promise<string> }).text === 'function') {
    responseText = await response.text()
    if (responseText.trim()) {
      try {
        data = JSON.parse(responseText)
      } catch {
        data = { success: false, error: { code: response.status, message: 'Network error' } }
      }
    }
  } else if (typeof (response as Response & { json?: () => Promise<unknown> }).json === 'function') {
    try {
      data = await response.json()
      responseText = JSON.stringify(data)
    } catch {
      data = { success: false, error: { code: response.status, message: 'Network error' } }
    }
  }

  if (response.ok) {
    if (data?.success === true) {
      return { success: true, data: data.data ?? data } as ApiResponse<T>
    }

    if (!responseText.trim()) {
      return { success: true, data: undefined as T } as ApiResponse<T>
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
  kemPublicKey: string
  dsaPublicKey: string
  hasPublicKey?: boolean
  hasKemPublicKey?: boolean
  hasDsaPublicKey?: boolean
  createdAt?: string
  lastSeen: string
  revoked: boolean
  isCurrent?: boolean
}

export interface Quotas {
  maxServers: number
  maxChannelsPerServer: number
  maxMessagesPerMinute: number
}

export type AdminUserRole = 'user' | 'admin'

export interface AdminUserItem {
  userId: string
  username: string
  role: AdminUserRole
  planId: string | null
}

export interface AdminServerItem {
  serverId: string
  name: string
  iconUrl: string | null
  ownerId: string
  memberCount: number
  createdAt: string
}

export interface AdminPlanItem {
  id: string
  name: string
  displayName: string
  description: string | null
  maxServers: number
  maxChannelsTextPerServer: number
  maxChannelsVoicePerServer: number
  maxMembersPerServer: number
  apiCallsPerMinute: number
  messagesPerDay: number
  isSystem: boolean
}

export interface AdminPlanInput {
  name: string
  displayName: string
  description?: string | null
  maxServers: number
  maxChannelsTextPerServer: number
  maxChannelsVoicePerServer: number
  maxMembersPerServer: number
  apiCallsPerMinute: number
  messagesPerDay: number
}

export interface InvitationCreateInfo {
  invitationId: string
  code: string
  serverId: string | null
  maxUses: number
  usesCount: number
  isActive: boolean
  createdBy: string
}

export interface InvitationListItem extends InvitationCreateInfo {
  remainingUses: number | null
}

export interface PlanTierInfo {
  id: string
  name: string
  displayName: string
  description: string | null
  maxServers: number
  maxChannelsTextPerServer: number
  maxChannelsVoicePerServer: number
  maxMembersPerServer: number
  apiCallsPerMinute: number
  messagesPerDay: number
}

export interface UserLimitsInfo {
  plan: {
    id: string
    name: string
    displayName: string
    description: string | null
    limits: {
      maxServers: number
      maxChannelsTextPerServer: number
      maxChannelsVoicePerServer: number
      maxMembersPerServer: number
      apiCallsPerMinute: number
      messagesPerDay: number
    }
  }
  usage: {
    totalServers: number
    totalTextChannels: number
    totalVoiceChannels: number
    totalMembersAcrossServers: number
    messagesToday: number
    apiCallsThisMinute: number
  }
  permissions: {
    canCreateServer: boolean
    canCreateTextChannel: boolean
    canCreateVoiceChannel: boolean
    canAddMembers: boolean
    canSendMessage: boolean
  }
  remaining: {
    servers: number
    textChannels: number
    voiceChannels: number
    members: number
    messagesToday: number
    apiCallsThisMinute: number
  }
}

export interface AdminUserLimitsInfo {
  userId: string
  plan: UserLimitsInfo['plan']
  usage: UserLimitsInfo['usage']
  remaining: UserLimitsInfo['remaining']
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
  keyVersion?: number | null
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

export interface DmChannelOpenInfo {
  dmChannelId: string
  peerUserId: string
  peerUsername: string
  encryptionType: 'asymmetric'
  messageTTL: number | null
  keyVersionId: string | null
  keyVersion: number | null
  created: boolean
}

export interface DmChannelListItem {
  dmChannelId: string
  peerUserId: string
  peerUsername: string
  messageTTL: number | null
  unreadCount: number
  lastMessageAt: string | null
}

export interface DmChannelRotateKeyInfo {
  dmChannelId: string
  keyVersionId: string
  keyVersion: number
}

export interface ChannelInfo {
  channelId: string
  name: string
  type: 'text' | 'voice'
  encryptionType: 'none' | 'symmetric' | 'asymmetric'
  scope?: 'server' | 'dm'
  dmPeerUserId?: string | null
  messageTTL: number | null
  isPrivate: boolean
  keyVersionId?: string | null
  keyVersion?: number | null
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
    channelId: channel.id,
    name: channel.name,
    type: channel.channel_type,
    encryptionType: channel.encryption_type,
    scope: channel.scope ?? 'server',
    dmPeerUserId: channel.dm_peer_user_id ?? channel.dmPeerUserId ?? null,
    messageTTL: channel.message_ttl,
    isPrivate: channel.is_private,
    createdAt: channel.created_at,
  }
}

function mapInviteResponse(response: any) {
  return {
    invitedUser: response.invited_user,
  }
}

function mapFriendPresence(friend: any): FriendPresence {
  const status = (friend.status ?? friend.presenceStatus ?? 'offline') as PresenceStatus
  return {
    userId: friend.user_id ?? friend.userId,
    username: friend.username,
    status,
    isOnline: status === 'online',
  }
}

function mapUserSearchResult(user: any): UserSearchResult {
  const status = (user.status ?? user.presenceStatus ?? 'offline') as PresenceStatus
  return {
    userId: user.user_id ?? user.userId,
    username: user.username,
    status,
    isOnline: status === 'online',
    isFriend: user.is_friend ?? user.isFriend ?? false,
  }
}

// ── API Functions ─────────────────────────────────────────────

export async function authRegister(username: string, password: string) {
  const deviceId = getStoredDeviceId()
  const payload = deviceId
    ? { username, password, device_id: deviceId }
    : { username, password }
  const result = await apiRequest<any>('POST', '/api/auth/register', payload)
  if (!result.success) return result
  return {
    success: true,
    data: {
      userId: result.data.user_id ?? result.data.userId,
      username: result.data.username,
      token: result.data.token,
      deviceId: result.data.device_id ?? result.data.deviceId,
      deviceLabel: result.data.device_label ?? result.data.deviceLabel,
    },
  } as ApiResponse<{
    userId: string
    username: string
    token: string
    deviceId: string
    deviceLabel: string
  }>
}

export async function authRegisterWithInvitation(code: string, username: string, password: string) {
  const deviceId = getStoredDeviceId()
  const payload = deviceId
    ? { code, username, password, device_id: deviceId }
    : { code, username, password }
  const result = await apiRequest<any>('POST', '/api/auth/register-with-invitation', payload)
  if (!result.success) return result
  return {
    success: true,
    data: {
      userId: result.data.user_id ?? result.data.userId,
      username: result.data.username,
      token: result.data.token,
      deviceId: result.data.device_id ?? result.data.deviceId,
      deviceLabel: result.data.device_label ?? result.data.deviceLabel,
    },
  } as ApiResponse<{
    userId: string
    username: string
    token: string
    deviceId: string
    deviceLabel: string
  }>
}

export async function authLogin(username: string, password: string) {
  const deviceId = getStoredDeviceId()
  const payload = deviceId
    ? { username, password, device_id: deviceId }
    : { username, password }
  const result = await apiRequest<any>('POST', '/api/auth/login', payload)
  if (!result.success) return result
  return {
    success: true,
    data: {
      userId: result.data.user_id ?? result.data.userId,
      username: result.data.username,
      token: result.data.token,
      deviceId: result.data.device_id ?? result.data.deviceId,
      deviceLabel: result.data.device_label ?? result.data.deviceLabel,
      isAdmin: result.data.is_admin ?? result.data.isAdmin ?? false,
    },
  } as ApiResponse<{
    userId: string
    username: string
    token: string
    deviceId: string
    deviceLabel: string
    isAdmin: boolean
  }>
}

export async function authRefresh() {
  return apiRequest<{ token: string }>('POST', '/api/auth/refresh')
}

export async function authMe() {
  return apiRequest<UserInfo>('GET', '/api/user/me')
}

export async function userChangePassword(oldPassword: string, newPassword: string): Promise<ApiResult<void>> {
  const result = await apiRequest<void>('PUT', '/api/user/me/password', {
    old_password: oldPassword,
    new_password: newPassword,
  })
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function userDevicesList(): Promise<ApiResult<DeviceInfo[]>> {
  const result = await apiRequest<any[]>('GET', '/api/user/me/devices')
  if (!result.success) return result
  const data = Array.isArray(result.data) ? result.data : []
  return {
    success: true,
    data: data.map((device: any) => ({
      deviceId: device.device_id ?? device.deviceId,
      label: device.label ?? 'Dispositiu',
      publicKey: device.kem_public_key ?? device.kemPublicKey ?? device.public_key ?? device.publicKey ?? '',
      kemPublicKey: device.kem_public_key ?? device.kemPublicKey ?? device.public_key ?? device.publicKey ?? '',
      dsaPublicKey: device.dsa_public_key ?? device.dsaPublicKey ?? '',
      hasPublicKey: device.has_kem_public_key ?? device.hasKemPublicKey ?? device.has_public_key ?? device.hasPublicKey ?? false,
      hasKemPublicKey: device.has_kem_public_key ?? device.hasKemPublicKey ?? device.has_public_key ?? device.hasPublicKey ?? false,
      hasDsaPublicKey: device.has_dsa_public_key ?? device.hasDsaPublicKey ?? false,
      createdAt: device.created_at ?? device.createdAt ?? null,
      lastSeen: device.last_seen ?? device.lastSeen ?? '',
      revoked: device.revoked ?? false,
      isCurrent: device.is_current ?? device.isCurrent ?? false,
    })),
  }
}

export async function userDeviceRevoke(deviceId: string): Promise<ApiResult<void>> {
  const result = await apiRequest<void>('DELETE', `/api/user/me/devices/${deviceId}`)
  if (!result.success) return result
  return { success: true, data: undefined }
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

export async function serversUpdate(serverId: string, name?: string, iconUrl?: string | null): Promise<ApiResult<ServerFullInfo>> {
  const payload: Record<string, unknown> = {}
  if (name !== undefined) payload.name = name
  if (iconUrl !== undefined) payload.icon_url = iconUrl

  const result = await apiRequest<any>('PUT', `/api/servers/${serverId}`, payload)
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
  const result = await apiRequest<any>('PUT', `/api/servers/${serverId}/members/${userId}/role`, { role })
  if (!result.success) return result
  return { success: true, data: mapServerMember(result.data) }
}

export async function serverRemoveMember(serverId: string, userId: string) {
  return apiRequest<{ userId: string; removed: boolean }>('DELETE', `/api/servers/${serverId}/members/${userId}`)
}

export async function messagesList(channelId: string, limit = 50, before?: string, scope?: 'server' | 'dm') {
  const params = new URLSearchParams({ limit: String(limit) })
  if (before) params.set('before', before)
  const path = scope === 'dm'
    ? `/api/dm/channels/${channelId}/messages?${params}`
    : `/api/channels/${channelId}/messages?${params}`
  const result = await apiRequest<PaginatedMessages>('GET', path)
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
  keyVersion?: number,
  expiresAt?: string,
  scope?: 'server' | 'dm'
) {
  const path = scope === 'dm' ? `/api/dm/channels/${channelId}/messages` : `/api/channels/${channelId}/messages`
  const result = await apiRequest<any>('POST', path, {
    encrypted_payload: encryptedPayload,
    iv,
    key_version: keyVersion,
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

export async function dmChannelOpen(targetUserId: string, messageTTL?: number | null): Promise<ApiResult<DmChannelOpenInfo>> {
  const result = await apiRequest<any>('POST', '/api/dm/channels/open', {
    target_user_id: targetUserId,
    message_ttl: messageTTL ?? null,
  })
  if (!result.success) return result
  const d = result.data
  return {
    success: true,
    data: {
      dmChannelId: d.dm_channel_id ?? d.dmChannelId,
      peerUserId: d.peer_user_id ?? d.peerUserId,
      peerUsername: d.peer_username ?? d.peerUsername,
      encryptionType: 'asymmetric',
      messageTTL: d.message_ttl ?? d.messageTTL ?? null,
      keyVersionId: d.key_version_id ?? d.keyVersionId ?? null,
      keyVersion: d.key_version ?? d.keyVersion ?? null,
      created: d.created ?? false,
    },
  }
}

export async function dmChannelsList(): Promise<ApiResult<DmChannelListItem[]>> {
  const result = await apiRequest<any[]>('GET', '/api/dm/channels')
  if (!result.success) return result
  const data = Array.isArray(result.data) ? result.data : []
  return {
    success: true,
    data: data.map((d: any) => ({
      dmChannelId: d.dm_channel_id ?? d.dmChannelId,
      peerUserId: d.peer_user_id ?? d.peerUserId,
      peerUsername: d.peer_username ?? d.peerUsername,
      messageTTL: d.message_ttl ?? d.messageTTL ?? null,
      unreadCount: d.unread_count ?? d.unreadCount ?? 0,
      lastMessageAt: d.last_message_at ?? d.lastMessageAt ?? null,
    })),
  }
}

export async function dmMessagesList(channelId: string, limit = 50, before?: string): Promise<ApiResult<PaginatedMessages>> {
  const params = new URLSearchParams({ limit: String(limit) })
  if (before) params.set('before', before)
  const result = await apiRequest<PaginatedMessages>('GET', `/api/dm/channels/${channelId}/messages?${params}`)
  if (!result.success || !result.data) return result
  return {
    success: true,
    data: {
      data: result.data.data.map(mapMessageToTypes),
      pagination: result.data.pagination,
    },
  }
}

export async function dmMessagesSend(
  channelId: string,
  encryptedPayload: string,
  iv: string,
  expiresAt?: string
): Promise<ApiResult<Message>> {
  const result = await apiRequest<any>('POST', `/api/dm/channels/${channelId}/messages`, {
    encrypted_payload: encryptedPayload,
    iv,
    expires_at: expiresAt,
  })
  if (!result.success || !result.data) return result
  return {
    success: true,
    data: mapMessageToTypes(result.data),
  }
}

export async function dmChannelUpdateSettings(channelId: string, messageTTL: number | null): Promise<ApiResult<{ dmChannelId: string; messageTTL: number | null }>> {
  const result = await apiRequest<any>('PUT', `/api/dm/channels/${channelId}/settings`, {
    message_ttl: messageTTL,
  })
  if (!result.success) return result
  return {
    success: true,
    data: {
      dmChannelId: result.data.dm_channel_id ?? result.data.dmChannelId,
      messageTTL: result.data.message_ttl ?? result.data.messageTTL ?? null,
    },
  }
}

export async function dmChannelRotateKey(channelId: string): Promise<ApiResult<DmChannelRotateKeyInfo>> {
  const result = await apiRequest<any>('POST', `/api/dm/channels/${channelId}/keys/rotate`)
  if (!result.success) return result
  return {
    success: true,
    data: {
      dmChannelId: result.data.dm_channel_id ?? result.data.dmChannelId,
      keyVersionId: result.data.key_version_id ?? result.data.keyVersionId,
      keyVersion: result.data.key_version ?? result.data.keyVersion,
    },
  }
}

export async function channelsList(serverId: string): Promise<ApiResult<Channel[]>> {
  const result = await apiRequest<any[]>('GET', `/api/servers/${serverId}/channels`)
  if (!result.success) return result
  return { success: true, data: result.data.map(mapChannelToTypes) }
}

function mapChannelToTypes(channel: any): Channel {
  return {
    channelId: channel.channel_id ?? channel.channelId ?? channel.id,
    name: channel.name,
    type: channel.channel_type ?? channel.type,
    encryptionType: channel.encryption_type ?? channel.encryptionType,
    permissionLevel: channel.permission_level ?? channel.permissionLevel ?? null,
    messageTTL: channel.message_ttl ?? channel.messageTTL ?? null,
    isPrivate: channel.is_private ?? channel.isPrivate ?? false,
    unreadCount: channel.unread_count ?? channel.unreadCount ?? 0,
    keyVersionId: channel.key_version_id ?? channel.keyVersionId ?? null,
    keyVersion: channel.key_version ?? channel.keyVersion ?? null,
    createdAt: channel.created_at ?? channel.createdAt,
  }
}

export async function channelsMarkRead(channelId: string, lastReadMessageId?: string) {
  return apiRequest<void>('POST', `/api/channels/${channelId}/read`, {
    last_read_message_id: lastReadMessageId ?? null,
  })
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
    keyVersion: msg.key_version ?? msg.keyVersion ?? null,
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
  messageTTL: number | null = null,
  isPrivate = false
): Promise<ApiResult<Channel>> {
  const result = await apiRequest<any>('POST', `/api/servers/${serverId}/channels`, {
    name,
    channel_type: type,
    encryption_type: encryptionType,
    message_ttl: messageTTL,
    is_private: isPrivate,
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

export async function channelGetAllKeyBundles(channelId: string): Promise<ApiResult<Array<{
  deviceId: string
  keyVersionId: string | null
  keyVersion: number | null
  encryptedKey: string
  kemCiphertext: string
  signature: string | null
  signedByDeviceId: string | null
}>>> {
  const result = await apiRequest<any[]>('GET', `/api/channels/${channelId}/keys/all`)
  if (!result.success) return result
  return {
    success: true,
    data: result.data.map((d: any) => ({
      deviceId: d.deviceId ?? d.device_id,
      keyVersionId: d.keyVersionId ?? d.key_version_id ?? null,
      keyVersion: d.keyVersion ?? d.key_version ?? null,
      encryptedKey: d.encryptedKey ?? d.encrypted_key,
      kemCiphertext: d.kemCiphertext ?? d.kem_ciphertext,
      signature: d.signature ?? null,
      signedByDeviceId: d.signedByDeviceId ?? d.signed_by_device_id ?? null,
    })),
  }
}

export async function channelGetKey(channelId: string): Promise<ApiResult<{
  deviceId: string
  keyVersionId?: string | null
  encryptedKey: string
  kemCiphertext: string
  signature?: string | null
  signedByDeviceId?: string | null
  keyVersion?: number | null
}>> {
  const result = await apiRequest<any>('GET', `/api/channels/${channelId}/keys`)
  if (!result.success) return result
  const d = result.data
  return {
    success: true,
    data: {
      deviceId: d.deviceId ?? d.device_id,
      keyVersionId: d.keyVersionId ?? d.key_version_id ?? null,
      encryptedKey: d.encryptedKey ?? d.encrypted_key,
      kemCiphertext: d.kemCiphertext ?? d.kem_ciphertext,
      signature: d.signature ?? null,
      signedByDeviceId: d.signedByDeviceId ?? d.signed_by_device_id ?? null,
      keyVersion: d.keyVersion ?? d.key_version ?? null,
    },
  }
}

export async function channelUploadKeys(
  channelId: string,
  bundles: Array<{
    deviceId: string
    encryptedKey: string
    kemCiphertext: string
    keyVersion?: number
    signature?: string
    signedByDeviceId?: string
  }>
): Promise<ApiResult<void>> {
  const body = bundles.map((b) => ({
    device_id: b.deviceId,
    encrypted_key: b.encryptedKey,
    kem_ciphertext: b.kemCiphertext,
    key_version: b.keyVersion,
    signature: b.signature,
    signed_by_device_id: b.signedByDeviceId,
  }))
  const result = await apiRequest<void>('POST', `/api/channels/${channelId}/keys`, body)
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function channelGetMemberDevices(channelId: string): Promise<ApiResult<Array<{
  deviceId: string
  publicKey: string
  kemPublicKey: string
  dsaPublicKey: string
  hasKemPublicKey: boolean
  hasDsaPublicKey: boolean
}>>> {
  const result = await apiRequest<any>('GET', `/api/channels/${channelId}/member-devices`)
  if (!result.success) return result
  const data = Array.isArray(result.data) ? result.data : (result.data?.data ?? [])
  return {
    success: true,
    data: data.map((d: any) => ({
      deviceId: d.deviceId ?? d.device_id,
      publicKey: d.kemPublicKey ?? d.kem_public_key ?? d.publicKey ?? d.public_key,
      kemPublicKey: d.kemPublicKey ?? d.kem_public_key ?? d.publicKey ?? d.public_key,
      dsaPublicKey: d.dsaPublicKey ?? d.dsa_public_key ?? '',
      hasKemPublicKey: d.hasKemPublicKey ?? d.has_kem_public_key ?? false,
      hasDsaPublicKey: d.hasDsaPublicKey ?? d.has_dsa_public_key ?? false,
    })),
  }
}

export async function channelGetPermissions(channelId: string): Promise<ApiResult<Array<{
  userId: string
  username: string
  permissionLevel: number
  permission: 'none' | 'read' | 'write' | 'manage'
}>>> {
  const result = await apiRequest<any>('GET', `/api/channels/${channelId}/permissions`)
  if (!result.success) return result
  const data = Array.isArray(result.data) ? result.data : (result.data?.data ?? [])
  return {
    success: true,
    data: data.map((item: any) => ({
      userId: item.userId ?? item.user_id,
      username: item.username,
      permissionLevel: Number(item.permissionLevel ?? item.permission_level ?? 0),
      permission: (item.permission ?? 'none') as 'none' | 'read' | 'write' | 'manage',
    })),
  }
}

export async function channelGetExplicitPermissions(channelId: string): Promise<ApiResult<Array<{
  userId: string
  username: string
  permissionLevel: number
  permission: 'none' | 'read' | 'write' | 'manage'
}>>> {
  const result = await apiRequest<any>('GET', `/api/channels/${channelId}/permissions/explicit`)
  if (!result.success) return result
  const data = Array.isArray(result.data) ? result.data : (result.data?.data ?? [])
  return {
    success: true,
    data: data.map((item: any) => ({
      userId: item.userId ?? item.user_id,
      username: item.username,
      permissionLevel: Number(item.permissionLevel ?? item.permission_level ?? 0),
      permission: (item.permission ?? 'none') as 'none' | 'read' | 'write' | 'manage',
    })),
  }
}

export async function channelSetExplicitPermission(
  channelId: string,
  userId: string,
  permissionLevel: number | null,
): Promise<ApiResult<void>> {
  const result = await apiRequest<void>(
    'PUT',
    `/api/channels/${channelId}/permissions/explicit/${userId}`,
    { permission_level: permissionLevel }
  )
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function deviceUpdatePublicKey(kemPublicKey: string, dsaPublicKey: string): Promise<ApiResult<void>> {
  const result = await apiRequest<void>('PUT', '/api/user/me/device/publickey', {
    kem_public_key: kemPublicKey,
    dsa_public_key: dsaPublicKey,
  })
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function friendsList(): Promise<ApiResult<FriendPresence[]>> {
  const result = await apiRequest<any>('GET', '/api/friends')
  if (!result.success) return result
  const data = Array.isArray(result.data) ? result.data : (result.data?.data ?? [])
  return { success: true, data: data.map(mapFriendPresence) }
}

export async function friendsAdd(username: string): Promise<ApiResult<void>> {
  const result = await apiRequest<void>('POST', '/api/friends', { username })
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function friendsRemove(friendUserId: string): Promise<ApiResult<void>> {
  const result = await apiRequest<void>('DELETE', `/api/friends/${friendUserId}`)
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function usersSearch(query: string): Promise<ApiResult<UserSearchResult[]>> {
  const params = new URLSearchParams({ q: query, limit: '20' })
  const result = await apiRequest<any>('GET', `/api/users/search?${params.toString()}`)
  if (!result.success) return result
  const data = Array.isArray(result.data) ? result.data : (result.data?.data ?? [])
  return { success: true, data: data.map(mapUserSearchResult) }
}

function mapAdminUserItem(raw: any): AdminUserItem {
  return {
    userId: raw.userId ?? raw.user_id,
    username: raw.username,
    role: (raw.role === 'admin' ? 'admin' : 'user') as AdminUserRole,
    planId: raw.planId ?? raw.plan_id ?? null,
  }
}

function mapAdminServerItem(raw: any): AdminServerItem {
  return {
    serverId: raw.serverId ?? raw.server_id,
    name: raw.name,
    iconUrl: raw.iconUrl ?? raw.icon_url ?? null,
    ownerId: raw.ownerId ?? raw.owner_id,
    memberCount: raw.memberCount ?? raw.member_count ?? 0,
    createdAt: raw.createdAt ?? raw.created_at ?? '',
  }
}

function mapAdminPlanItem(raw: any): AdminPlanItem {
  return {
    id: raw.id,
    name: raw.name,
    displayName: raw.displayName ?? raw.display_name,
    description: raw.description ?? null,
    maxServers: raw.maxServers ?? raw.max_servers,
    maxChannelsTextPerServer: raw.maxChannelsTextPerServer ?? raw.max_channels_text_per_server,
    maxChannelsVoicePerServer: raw.maxChannelsVoicePerServer ?? raw.max_channels_voice_per_server,
    maxMembersPerServer: raw.maxMembersPerServer ?? raw.max_members_per_server,
    apiCallsPerMinute: raw.apiCallsPerMinute ?? raw.api_calls_per_minute,
    messagesPerDay: raw.messagesPerDay ?? raw.messages_per_day,
    isSystem: Boolean(raw.isSystem ?? raw.is_system),
  }
}

export async function adminUsersList(): Promise<ApiResult<AdminUserItem[]>> {
  const result = await apiRequest<any>('GET', '/api/admin/users')
  if (!result.success) return result
  const data = Array.isArray(result.data) ? result.data : (result.data?.data ?? [])
  return { success: true, data: data.map(mapAdminUserItem) }
}

export async function adminUsersCreate(
  username: string,
  password: string,
  role: AdminUserRole,
  planId?: string | null,
): Promise<ApiResult<AdminUserItem>> {
  const result = await apiRequest<any>('POST', '/api/admin/users', {
    username,
    password,
    role,
    ...(planId ? { plan_id: planId } : {}),
  })
  if (!result.success) return result

  const userLike = {
    userId: result.data?.userId ?? result.data?.user_id,
    username: result.data?.username ?? username,
    role: result.data?.role ?? role,
    planId: result.data?.planId ?? result.data?.plan_id ?? planId ?? null,
  }

  return { success: true, data: mapAdminUserItem(userLike) }
}

export async function adminUsersUpdateRole(userId: string, role: AdminUserRole): Promise<ApiResult<void>> {
  const result = await apiRequest<void>('PUT', `/api/admin/users/${userId}/role/${role}`)
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function adminUsersUpdatePlan(userId: string, planId: string): Promise<ApiResult<void>> {
  const result = await apiRequest<void>('PUT', `/api/admin/users/${userId}/plan/${planId}`)
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function adminUsersDelete(userId: string): Promise<ApiResult<void>> {
  const result = await apiRequest<void>('DELETE', `/api/admin/users/${userId}`)
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function adminServersList(): Promise<ApiResult<AdminServerItem[]>> {
  const result = await apiRequest<any>('GET', '/api/admin/servers')
  if (!result.success) return result
  const data = Array.isArray(result.data) ? result.data : (result.data?.data ?? [])
  return { success: true, data: data.map(mapAdminServerItem) }
}

export async function adminServersCreate(name: string, iconUrl?: string | null): Promise<ApiResult<AdminServerItem>> {
  const result = await apiRequest<any>('POST', '/api/admin/servers', {
    name,
    icon_url: iconUrl ?? null,
  })
  if (!result.success) return result

  const serverLike = {
    serverId: result.data?.serverId ?? result.data?.server_id,
    name: result.data?.name ?? name,
    iconUrl: result.data?.iconUrl ?? result.data?.icon_url ?? iconUrl ?? null,
    ownerId: result.data?.ownerId ?? result.data?.owner_id ?? '',
    memberCount: result.data?.memberCount ?? result.data?.member_count ?? 0,
    createdAt: result.data?.createdAt ?? result.data?.created_at ?? '',
  }

  return { success: true, data: mapAdminServerItem(serverLike) }
}

export async function adminServersUpdate(serverId: string, name?: string, iconUrl?: string | null): Promise<ApiResult<void>> {
  const payload: Record<string, unknown> = {}
  if (name !== undefined) payload.name = name
  if (iconUrl !== undefined) payload.icon_url = iconUrl

  const result = await apiRequest<void>('PUT', `/api/admin/servers/${serverId}`, payload)
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function adminServersDelete(serverId: string): Promise<ApiResult<void>> {
  const result = await apiRequest<void>('DELETE', `/api/admin/servers/${serverId}`)
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function adminPlansList(): Promise<ApiResult<AdminPlanItem[]>> {
  const result = await apiRequest<any>('GET', '/api/admin/plans')
  if (!result.success) return result
  const data = Array.isArray(result.data) ? result.data : (result.data?.data ?? [])
  return { success: true, data: data.map(mapAdminPlanItem) }
}

export async function adminPlansCreate(input: AdminPlanInput): Promise<ApiResult<AdminPlanItem>> {
  const result = await apiRequest<any>('POST', '/api/admin/plans', {
    name: input.name,
    displayName: input.displayName,
    description: input.description ?? null,
    maxServers: input.maxServers,
    maxChannelsTextPerServer: input.maxChannelsTextPerServer,
    maxChannelsVoicePerServer: input.maxChannelsVoicePerServer,
    maxMembersPerServer: input.maxMembersPerServer,
    apiCallsPerMinute: input.apiCallsPerMinute,
    messagesPerDay: input.messagesPerDay,
  })
  if (!result.success) return result
  return { success: true, data: mapAdminPlanItem(result.data) }
}

export async function adminPlansUpdate(planId: string, input: Partial<AdminPlanInput>): Promise<ApiResult<void>> {
  const payload: Record<string, unknown> = {}
  if (input.name !== undefined) payload.name = input.name
  if (input.displayName !== undefined) payload.displayName = input.displayName
  if (input.description !== undefined) payload.description = input.description
  if (input.maxServers !== undefined) payload.maxServers = input.maxServers
  if (input.maxChannelsTextPerServer !== undefined) payload.maxChannelsTextPerServer = input.maxChannelsTextPerServer
  if (input.maxChannelsVoicePerServer !== undefined) payload.maxChannelsVoicePerServer = input.maxChannelsVoicePerServer
  if (input.maxMembersPerServer !== undefined) payload.maxMembersPerServer = input.maxMembersPerServer
  if (input.apiCallsPerMinute !== undefined) payload.apiCallsPerMinute = input.apiCallsPerMinute
  if (input.messagesPerDay !== undefined) payload.messagesPerDay = input.messagesPerDay

  const result = await apiRequest<void>('PUT', `/api/admin/plans/${planId}`, payload)
  if (!result.success) return result
  return { success: true, data: undefined }
}

export async function adminPlansDelete(planId: string): Promise<ApiResult<void>> {
  const result = await apiRequest<void>('DELETE', `/api/admin/plans/${planId}`)
  if (!result.success) return result
  return { success: true, data: undefined }
}

function mapInvitationCreateInfo(raw: any): InvitationCreateInfo {
  return {
    invitationId: raw.invitationId ?? raw.invitation_id,
    code: raw.code,
    serverId: raw.serverId ?? raw.server_id ?? null,
    maxUses: raw.maxUses ?? raw.max_uses ?? 1,
    usesCount: raw.usesCount ?? raw.uses_count ?? 0,
    isActive: raw.isActive ?? raw.is_active ?? true,
    createdBy: raw.createdBy ?? raw.created_by ?? '',
  }
}

function mapInvitationListItem(raw: any): InvitationListItem {
  const base = mapInvitationCreateInfo(raw)
  return {
    ...base,
    remainingUses: raw.remainingUses ?? raw.remaining_uses ?? null,
  }
}

function mapPlanTierInfo(raw: any): PlanTierInfo {
  return {
    id: raw.id,
    name: raw.name,
    displayName: raw.displayName ?? raw.display_name,
    description: raw.description ?? null,
    maxServers: raw.maxServers ?? raw.max_servers,
    maxChannelsTextPerServer: raw.maxChannelsTextPerServer ?? raw.max_channels_text_per_server,
    maxChannelsVoicePerServer: raw.maxChannelsVoicePerServer ?? raw.max_channels_voice_per_server,
    maxMembersPerServer: raw.maxMembersPerServer ?? raw.max_members_per_server,
    apiCallsPerMinute: raw.apiCallsPerMinute ?? raw.api_calls_per_minute,
    messagesPerDay: raw.messagesPerDay ?? raw.messages_per_day,
  }
}

function mapUserLimitsInfo(raw: any): UserLimitsInfo {
  return {
    plan: {
      id: raw.plan?.id,
      name: raw.plan?.name,
      displayName: raw.plan?.displayName ?? raw.plan?.display_name,
      description: raw.plan?.description ?? null,
      limits: {
        maxServers: raw.plan?.limits?.maxServers ?? raw.plan?.limits?.max_servers,
        maxChannelsTextPerServer: raw.plan?.limits?.maxChannelsTextPerServer ?? raw.plan?.limits?.max_channels_text_per_server,
        maxChannelsVoicePerServer: raw.plan?.limits?.maxChannelsVoicePerServer ?? raw.plan?.limits?.max_channels_voice_per_server,
        maxMembersPerServer: raw.plan?.limits?.maxMembersPerServer ?? raw.plan?.limits?.max_members_per_server,
        apiCallsPerMinute: raw.plan?.limits?.apiCallsPerMinute ?? raw.plan?.limits?.api_calls_per_minute,
        messagesPerDay: raw.plan?.limits?.messagesPerDay ?? raw.plan?.limits?.messages_per_day,
      },
    },
    usage: {
      totalServers: raw.usage?.totalServers ?? raw.usage?.total_servers ?? 0,
      totalTextChannels: raw.usage?.totalTextChannels ?? raw.usage?.total_text_channels ?? 0,
      totalVoiceChannels: raw.usage?.totalVoiceChannels ?? raw.usage?.total_voice_channels ?? 0,
      totalMembersAcrossServers: raw.usage?.totalMembersAcrossServers ?? raw.usage?.total_members_across_servers ?? 0,
      messagesToday: raw.usage?.messagesToday ?? raw.usage?.messages_today ?? 0,
      apiCallsThisMinute: raw.usage?.apiCallsThisMinute ?? raw.usage?.api_calls_this_minute ?? 0,
    },
    permissions: {
      canCreateServer: Boolean(raw.permissions?.canCreateServer ?? raw.permissions?.can_create_server),
      canCreateTextChannel: Boolean(raw.permissions?.canCreateTextChannel ?? raw.permissions?.can_create_text_channel),
      canCreateVoiceChannel: Boolean(raw.permissions?.canCreateVoiceChannel ?? raw.permissions?.can_create_voice_channel),
      canAddMembers: Boolean(raw.permissions?.canAddMembers ?? raw.permissions?.can_add_members),
      canSendMessage: Boolean(raw.permissions?.canSendMessage ?? raw.permissions?.can_send_message),
    },
    remaining: {
      servers: raw.remaining?.servers ?? 0,
      textChannels: raw.remaining?.textChannels ?? raw.remaining?.text_channels ?? 0,
      voiceChannels: raw.remaining?.voiceChannels ?? raw.remaining?.voice_channels ?? 0,
      members: raw.remaining?.members ?? 0,
      messagesToday: raw.remaining?.messagesToday ?? raw.remaining?.messages_today ?? 0,
      apiCallsThisMinute: raw.remaining?.apiCallsThisMinute ?? raw.remaining?.api_calls_this_minute ?? 0,
    },
  }
}

export async function invitationsCreate(maxUses: number, serverId?: string | null): Promise<ApiResult<InvitationCreateInfo>> {
  const result = await apiRequest<any>('POST', '/api/invitations', {
    max_uses: maxUses,
    ...(serverId ? { server_id: serverId } : {}),
  })
  if (!result.success) return result
  return { success: true, data: mapInvitationCreateInfo(result.data) }
}

export async function invitationsList(): Promise<ApiResult<InvitationListItem[]>> {
  const result = await apiRequest<any>('GET', '/api/invitations')
  if (!result.success) return result
  const data = Array.isArray(result.data) ? result.data : (result.data?.data ?? [])
  return { success: true, data: data.map(mapInvitationListItem) }
}

export async function plansList(): Promise<ApiResult<PlanTierInfo[]>> {
  const result = await apiRequest<any>('GET', '/api/plans')
  if (!result.success) return result
  const data = Array.isArray(result.data) ? result.data : (result.data?.data ?? [])
  return { success: true, data: data.map(mapPlanTierInfo) }
}

export async function userLimitsGet(): Promise<ApiResult<UserLimitsInfo>> {
  const result = await apiRequest<any>('GET', '/api/user/me/limits')
  if (!result.success) return result
  return { success: true, data: mapUserLimitsInfo(result.data?.data ?? result.data) }
}

export async function adminUserLimitsGet(userId: string): Promise<ApiResult<AdminUserLimitsInfo>> {
  const result = await apiRequest<any>('GET', `/api/admin/users/${userId}/limits`)
  if (!result.success) return result
  const data = result.data?.data ?? result.data
  return {
    success: true,
    data: {
      userId: data.userId ?? data.user_id ?? userId,
      plan: {
        id: data.plan?.id,
        name: data.plan?.name,
        displayName: data.plan?.displayName ?? data.plan?.display_name,
        description: data.plan?.description ?? null,
        limits: {
          maxServers: data.plan?.limits?.maxServers ?? data.plan?.limits?.max_servers,
          maxChannelsTextPerServer: data.plan?.limits?.maxChannelsTextPerServer ?? data.plan?.limits?.max_channels_text_per_server,
          maxChannelsVoicePerServer: data.plan?.limits?.maxChannelsVoicePerServer ?? data.plan?.limits?.max_channels_voice_per_server,
          maxMembersPerServer: data.plan?.limits?.maxMembersPerServer ?? data.plan?.limits?.max_members_per_server,
          apiCallsPerMinute: data.plan?.limits?.apiCallsPerMinute ?? data.plan?.limits?.api_calls_per_minute,
          messagesPerDay: data.plan?.limits?.messagesPerDay ?? data.plan?.limits?.messages_per_day,
        },
      },
      usage: {
        totalServers: data.usage?.totalServers ?? data.usage?.total_servers ?? 0,
        totalTextChannels: data.usage?.totalTextChannels ?? data.usage?.total_text_channels ?? 0,
        totalVoiceChannels: data.usage?.totalVoiceChannels ?? data.usage?.total_voice_channels ?? 0,
        totalMembersAcrossServers: data.usage?.totalMembersAcrossServers ?? data.usage?.total_members_across_servers ?? 0,
        messagesToday: data.usage?.messagesToday ?? data.usage?.messages_today ?? 0,
        apiCallsThisMinute: data.usage?.apiCallsThisMinute ?? data.usage?.api_calls_this_minute ?? 0,
      },
      remaining: {
        servers: data.remaining?.servers ?? 0,
        textChannels: data.remaining?.textChannels ?? data.remaining?.text_channels ?? 0,
        voiceChannels: data.remaining?.voiceChannels ?? data.remaining?.voice_channels ?? 0,
        members: data.remaining?.members ?? 0,
        messagesToday: data.remaining?.messagesToday ?? data.remaining?.messages_today ?? 0,
        apiCallsThisMinute: data.remaining?.apiCallsThisMinute ?? data.remaining?.api_calls_this_minute ?? 0,
      },
    },
  }
}

// ── LiveKit ─────────────────────────────────────────────────────

export interface LiveKitTokenResponse {
  token: string
}

export async function livekitGetToken(
  channelId: string,
  participantName: string,
): Promise<ApiResult<LiveKitTokenResponse>> {
  return apiRequest<LiveKitTokenResponse>('POST', '/api/livekit/token', {
    room: 'chillgroup-' + channelId,
    participant: participantName,
  })
}
