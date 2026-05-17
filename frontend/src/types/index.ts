// Types globals per a ChillGroup v2

export interface User {
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

export interface Server {
  serverId: string
  name: string
  iconUrl: string | null
  ownerId: string
  memberCount: number
  myRole: ServerRole
  createdAt: string
}

export interface ServerFullInfo extends Server {
  members: ServerMember[]
}

export type ServerRole = 'owner' | 'admin' | 'member'

export interface ServerMember {
  userId: string
  username: string
  role: ServerRole
  joinedAt: string
}

export type ChannelType = 'text' | 'voice'
export type EncryptionType = 'none' | 'symmetric' | 'asymmetric'

export interface Channel {
  channelId: string
  name: string
  type: ChannelType
  encryptionType: EncryptionType
  messageTTL: number | null
  isPrivate: boolean
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

export interface VoiceParticipant {
  userId: string
  username: string
  avatar?: string
  joinedAt: string
  isDeafened: boolean
  isSuppressed: boolean
  isSpeaking: boolean
}

export interface PaginationMeta {
  has_more: boolean
  next_cursor: string | null
  prev_cursor: string | null
  total_new?: number
}

export interface PaginatedResult<T> {
  data: T[]
  pagination: PaginationMeta
}

export interface ApiError {
  success: false
  error: {
    code: number
    message: string
    details?: Record<string, string>
  }
}

export interface ApiSuccess<T> {
  success: true
  data: T
}

export type ApiResult<T> = ApiSuccess<T> | ApiError