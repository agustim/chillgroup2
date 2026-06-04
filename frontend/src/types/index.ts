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
  kemPublicKey?: string
  dsaPublicKey?: string
  hasPublicKey?: boolean
  hasKemPublicKey?: boolean
  hasDsaPublicKey?: boolean
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

export interface ServerLiveKitConfig {
  host: string
  apiKey: string
  isOverride: boolean
}

export interface ServerFullInfo extends Server {
  members: ServerMember[]
  livekitConfig?: ServerLiveKitConfig | null
}

export type ServerRole = 'owner' | 'admin' | 'member'

export interface ServerMember {
  userId: string
  username: string
  role: ServerRole
  joinedAt: string
}

export interface Friend {
  userId: string
  username: string
  status: PresenceStatus
}

export interface FriendPresence extends Friend {
  isOnline: boolean
}

export interface UserSearchResult extends Friend {
  isFriend: boolean
  isOnline?: boolean
}

export type PresenceStatus = 'online' | 'offline'

export type ChannelType = 'text' | 'voice'
export type EncryptionType = 'none' | 'symmetric' | 'asymmetric'

export interface Channel {
  channelId: string
  name: string
  type: ChannelType
  encryptionType: EncryptionType
  permissionLevel?: number | null
  scope?: 'server' | 'dm'
  dmPeerUserId?: string | null
  messageTTL: number | null
  isPrivate: boolean
  unreadCount?: number
  lastReadMessageId?: string | null
  keyVersionId?: string | null
  keyVersion?: number | null
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
  attachmentIds?: string[]
  keyVersion?: number | null
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
  /** Track de vídeo de LiveKit, si la càmera és activa */
  videoTrack?: any
}

export interface VoiceConnection {
  channelId: string
  channelName: string
  participants: VoiceParticipant[]
  isJoined: boolean
  isMuted: boolean
  isDeafened: boolean
  isCameraOn: boolean
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