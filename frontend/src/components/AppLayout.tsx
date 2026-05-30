import React, { useEffect, useMemo, useState } from 'react'
import { useAuth } from '../contexts/AuthContext'
import { ServerBar } from './sidebar/ServerBar'
import { ChannelList } from './sidebar/ChannelList'
import { MainContent } from './main/MainContent'
import { CreateServerModal } from './modals/CreateServerModal'
import { CreateTextChannelModal } from './modals/CreateTextChannelModal'
import { CreateVoiceChannelModal } from './modals/CreateVoiceChannelModal'
import { InviteMemberModal } from './modals/InviteMemberModal'
import { DeviceKeysModal } from './modals/DeviceKeysModal'
import { ChannelKeysModal } from './modals/ChannelKeysModal'
import { PermissionsPanel } from './modals/PermissionsModal'
import { ChangePasswordModal } from './modals/ChangePasswordModal'
import { FriendsModal } from './modals/FriendsModal'
import { AdminUsersPanel } from './main/AdminUsersPanel'
import { useLiveKit } from '../hooks/useLiveKit'
import { Channel, FriendPresence, Server, ServerFullInfo, VoiceParticipant } from '../types'
import { disconnectSocket, getSocket } from '../lib/socket'
import { hasLocalDeviceKeypair } from '../lib/device-keys'
import { ensureChannelKey, distributeChannelKey, syncChannelKeys } from '../lib/channel-crypto'
import { getChannelKey, getLatestChannelKey } from '../lib/storage'
import {
  friendsAdd,
  friendsList,
  friendsRemove,
  dmChannelOpen,
  dmChannelsList,
  serverInviteMember,
  serverUpdateMemberRole,
  serverRemoveMember,
  serversCreate,
  serversGet,
  serversList,
  channelsCreate,
  channelsList,
  channelsMarkRead,
  channelInvite,
  channelGetPermissions,
  channelGetExplicitPermissions,
  channelSetExplicitPermission,
  channelsUpdate,
  channelDelete,
  usersSearch,
  dmChannelRotateKey,
  userLimitsGet,
} from '../lib/api'
import { logger } from '../lib/logger'

interface AppLayoutProps {
  username: string
  onLogout?: () => void
}

type PanelType = 'none' | 'serverConfig' | 'channelConfig' | 'devices' | 'adminUsers' | 'permissions'
type ServerMenuAction = 'config' | 'invite' | 'createText' | 'createVoice' | null

function formatDmRepairFeedback(result: {
  discoveredDevices: string[]
  skippedSelfDevices: string[]
  skippedMissingKemDevices: string[]
  uploadedBundleDevices: string[]
  failedDevices: Array<{ deviceId: string; reason: string }>
}): string {
  const parts = [
    `Devices DM vistos: ${result.discoveredDevices.length}`,
    `bundles pujats: ${result.uploadedBundleDevices.length}`,
  ]

  if (result.skippedSelfDevices.length > 0) {
    parts.push(`omès actual: ${result.skippedSelfDevices.join(', ')}`)
  }

  if (result.skippedMissingKemDevices.length > 0) {
    parts.push(`sense kemPublicKey: ${result.skippedMissingKemDevices.join(', ')}`)
  }

  if (result.failedDevices.length > 0) {
    parts.push(`fallits: ${result.failedDevices.map((item) => `${item.deviceId} (${item.reason})`).join(', ')}`)
  }

  return parts.join(' | ')
}

export function AppLayout({ username, onLogout }: AppLayoutProps) {
  const { user, logout, currentDeviceId } = useAuth()
  const [servers, setServers] = useState<Server[]>([])
  const [selectedServer, setSelectedServer] = useState<string | null>(null)
  const [serverDetails, setServerDetails] = useState<ServerFullInfo | null>(null)
  const [channels, setChannels] = useState<Channel[]>([])
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null)
  const [openTextChannelIds, setOpenTextChannelIds] = useState<string[]>([])
  const [voiceAsTextMode, setVoiceAsTextMode] = useState(false)
  const [voiceChannelId, setVoiceChannelId] = useState<string | null>(null)
  const [voiceChannelName, setVoiceChannelName] = useState<string>('')
  const [voicePresenceByChannel, setVoicePresenceByChannel] = useState<Record<string, VoiceParticipant[]>>({})
  const [friends, setFriends] = useState<FriendPresence[]>([])
  const [serverMemberPresenceById, setServerMemberPresenceById] = useState<Record<string, boolean>>({})
  const [panel, setPanel] = useState<PanelType>('none')
  const [feedback, setFeedback] = useState<string | null>(null)
  const [dmKeyActionBusy, setDmKeyActionBusy] = useState(false)
  const [canCreateServer, setCanCreateServer] = useState(true)
  const [canCreateTextChannel, setCanCreateTextChannel] = useState(true)
  const [canCreateVoiceChannel, setCanCreateVoiceChannel] = useState(true)
  
  // Modal states
  const [showCreateServer, setShowCreateServer] = useState(false)
  const [showCreateTextChannel, setShowCreateTextChannel] = useState(false)
  const [showCreateVoiceChannel, setShowCreateVoiceChannel] = useState(false)
  const [showInviteServer, setShowInviteServer] = useState(false)
  const [showDeviceKeysModal, setShowDeviceKeysModal] = useState(false)
  const [showChannelKeysModal, setShowChannelKeysModal] = useState(false)
  const [showChangePasswordModal, setShowChangePasswordModal] = useState(false)
  const [showFriendsModal, setShowFriendsModal] = useState(false)
  const [pendingServerConfigOpenId, setPendingServerConfigOpenId] = useState<string | null>(null)
  const [serverConfigInviteUsername, setServerConfigInviteUsername] = useState('')
  const [pendingMemberRemovalId, setPendingMemberRemovalId] = useState<string | null>(null)
  const [channelConfigName, setChannelConfigName] = useState('')
  const [channelConfigMessageTTL, setChannelConfigMessageTTL] = useState('')
  const [channelConfigIsPrivate, setChannelConfigIsPrivate] = useState(false)
  const [channelConfigInviteUsername, setChannelConfigInviteUsername] = useState('')
  const [channelExplicitPermissions, setChannelExplicitPermissions] = useState<Array<{
    userId: string
    username: string
    permissionLevel: number
    permission: 'none' | 'read' | 'write' | 'manage'
  }>>([])
  const [channelExplicitPermissionsLoading, setChannelExplicitPermissionsLoading] = useState(false)
  const [canViewChannelExplicitPermissions, setCanViewChannelExplicitPermissions] = useState(false)
  const [channelPermissionRows, setChannelPermissionRows] = useState<Array<{
    userId: string
    username: string
    effectiveLevel: number
    effectivePermission: 'none' | 'read' | 'write' | 'manage'
    explicitLevel: number | null
  }>>([])
  const [updatingChannelPermissionUserId, setUpdatingChannelPermissionUserId] = useState<string | null>(null)
  
  // LiveKit hook
  const {
    isConnected: liveKitConnected,
    isPublishing,
    isMuted: liveKitMuted,
    isDeafened: liveKitDeafened,
    isCameraOn: liveKitCameraOn,
    isScreenSharing: liveKitScreenSharing,
    localVideoTrack,
    localScreenTrack,
    remoteVideoTracks,
    participants: liveKitParticipants,
    connectToChannel: connectLiveKit,
    disconnect: disconnectLiveKit,
    toggleMute: toggleLiveKitMute,
    toggleDeafen: toggleLiveKitDeafen,
    toggleCamera: toggleLiveKitCamera,
    toggleScreenShare: toggleLiveKitScreenShare,
    error: liveKitError,
  } = useLiveKit()

  // Auto-dismiss feedback
  useEffect(() => {
    if (feedback) {
      const isError = feedback.includes('ha fallat') || feedback.startsWith('Error:')
      const timer = setTimeout(() => setFeedback(null), isError ? 12000 : 3000)
      return () => clearTimeout(timer)
    }
  }, [feedback])

  useEffect(() => {
    if (!user || !currentDeviceId) {
      return
    }

    let cancelled = false
    hasLocalDeviceKeypair(currentDeviceId)
      .then((hasKeypair) => {
        if (!cancelled && !hasKeypair) {
          setShowDeviceKeysModal(true)
        }
      })
      .catch(() => {
        if (!cancelled) {
          setShowDeviceKeysModal(true)
        }
      })

    return () => {
      cancelled = true
    }
  }, [user, currentDeviceId])

  useEffect(() => {
    if (!selectedChannel || selectedChannel.encryptionType === 'none' || !currentDeviceId) return
    syncChannelKeys(selectedChannel.channelId, selectedChannel.encryptionType as import('../types').EncryptionType, currentDeviceId).catch(() => {})
  }, [selectedChannel?.channelId, currentDeviceId])

  useEffect(() => {
    let cancelled = false

    const loadFriends = async () => {
      const result = await friendsList()
      if (!cancelled && result.success) {
        setFriends(result.data)
      }
    }

    void loadFriends()

    return () => {
      cancelled = true
    }
  }, [])

  useEffect(() => {
    let cancelled = false

    const loadUserLimits = async () => {
      if (!user) {
        setCanCreateServer(true)
        setCanCreateTextChannel(true)
        setCanCreateVoiceChannel(true)
        return
      }

      const result = await userLimitsGet()
      if (!cancelled && result.success) {
        setCanCreateServer(result.data.permissions.canCreateServer)
        setCanCreateTextChannel(result.data.permissions.canCreateTextChannel)
        setCanCreateVoiceChannel(result.data.permissions.canCreateVoiceChannel)
      }
    }

    void loadUserLimits()

    return () => {
      cancelled = true
    }
  }, [user?.userId])

  const selectedServerInfo = selectedServer ? servers.find((server) => server.serverId === selectedServer) : undefined
  const resolvedSelectedChannel = selectedChannel
    ? channels.find((channel) => channel.channelId === selectedChannel.channelId) ?? selectedChannel
    : null
  const canManageServer =
    selectedServerInfo?.myRole === 'owner' ||
    selectedServerInfo?.myRole === 'admin' ||
    serverDetails?.myRole === 'owner' ||
    serverDetails?.myRole === 'admin'

  useEffect(() => {
    if (panel !== 'channelConfig' || !resolvedSelectedChannel) {
      return
    }
    setChannelConfigName(resolvedSelectedChannel.name)
    setChannelConfigMessageTTL(
      resolvedSelectedChannel.messageTTL === null || resolvedSelectedChannel.messageTTL === undefined
        ? ''
        : String(resolvedSelectedChannel.messageTTL)
    )
    setChannelConfigIsPrivate(!!resolvedSelectedChannel.isPrivate)
    setChannelConfigInviteUsername('')
  }, [panel, resolvedSelectedChannel?.channelId, resolvedSelectedChannel?.name, resolvedSelectedChannel?.messageTTL, resolvedSelectedChannel?.isPrivate])

  useEffect(() => {
    if (panel !== 'channelConfig' || !resolvedSelectedChannel) {
      setChannelExplicitPermissions([])
      setCanViewChannelExplicitPermissions(false)
      setChannelExplicitPermissionsLoading(false)
      return
    }

    let cancelled = false
    const loadExplicitPermissions = async () => {
      setChannelExplicitPermissionsLoading(true)
      const result = await channelGetExplicitPermissions(resolvedSelectedChannel.channelId)
      if (cancelled) return

      if (result.success) {
        setCanViewChannelExplicitPermissions(true)
        setChannelExplicitPermissions(result.data)
      } else {
        setCanViewChannelExplicitPermissions(false)
        setChannelExplicitPermissions([])
      }

      setChannelExplicitPermissionsLoading(false)
    }

    void loadExplicitPermissions()

    return () => {
      cancelled = true
    }
  }, [panel, resolvedSelectedChannel?.channelId])

  useEffect(() => {
    if (panel !== 'channelConfig' || !resolvedSelectedChannel) {
      setChannelPermissionRows([])
      return
    }

    let cancelled = false
    const loadChannelPermissions = async () => {
      const [effectiveResult, explicitResult] = await Promise.all([
        channelGetPermissions(resolvedSelectedChannel.channelId),
        channelGetExplicitPermissions(resolvedSelectedChannel.channelId),
      ])

      if (cancelled) return

      if (!effectiveResult.success) {
        setChannelPermissionRows([])
        return
      }

      const explicitMap = new Map<string, number>()
      if (explicitResult.success) {
        for (const entry of explicitResult.data) {
          explicitMap.set(entry.userId, entry.permissionLevel)
        }
      }

      setChannelPermissionRows(
        effectiveResult.data.map((entry) => ({
          userId: entry.userId,
          username: entry.username,
          effectiveLevel: entry.permissionLevel,
          effectivePermission: entry.permission,
          explicitLevel: explicitMap.get(entry.userId) ?? null,
        }))
      )
    }

    void loadChannelPermissions()

    return () => {
      cancelled = true
    }
  }, [panel, resolvedSelectedChannel?.channelId, channelExplicitPermissions])

  const handleUpdateChannelExplicitPermission = async (userId: string, value: string) => {
    if (!resolvedSelectedChannel) return

    setUpdatingChannelPermissionUserId(userId)
    const nextLevel = value === 'inherited' ? null : Number(value)
    const result = await channelSetExplicitPermission(resolvedSelectedChannel.channelId, userId, nextLevel)
    setUpdatingChannelPermissionUserId(null)

    if (!result.success) {
      setFeedback(result.error.message)
      return
    }

    const explicitResult = await channelGetExplicitPermissions(resolvedSelectedChannel.channelId)
    if (!explicitResult.success) {
      setFeedback(explicitResult.error.message)
      return
    }
    setChannelExplicitPermissions(explicitResult.data)
    setFeedback('Permís del canal actualitzat')
  }

  useEffect(() => {
    const socket = getSocket()
    const handleFriendPresenceUpdated = (payload: { userId: string; username: string; status: string }) => {
      const status = payload.status === 'online' ? 'online' : 'offline'
      setFriends((current) => current.map((friend) => (
        friend.userId === payload.userId
          ? { ...friend, username: payload.username, status, isOnline: status === 'online' }
          : friend
      )))
    }

    socket.on('friend-presence-updated', handleFriendPresenceUpdated)
    return () => {
      socket.off('friend-presence-updated', handleFriendPresenceUpdated)
    }
  }, [])

  useEffect(() => {
    const socket = getSocket()
    let serversRefreshTimer: number | null = null
    let channelsRefreshTimer: number | null = null

    const handleUserServersUpdated = async () => {
      if (serversRefreshTimer !== null) {
        window.clearTimeout(serversRefreshTimer)
      }
      serversRefreshTimer = window.setTimeout(() => {
        void fetchServers()
      }, 250)
    }

    const handleServerChannelsUpdated = async (payload: { serverId?: string }) => {
      if (!selectedServer || payload.serverId !== selectedServer) {
        return
      }
      if (channelsRefreshTimer !== null) {
        window.clearTimeout(channelsRefreshTimer)
      }
      channelsRefreshTimer = window.setTimeout(() => {
        void fetchChannels(selectedServer)
      }, 250)
    }

    socket.on('user-servers-updated', handleUserServersUpdated)
    socket.on('server-channels-updated', handleServerChannelsUpdated)

    return () => {
      if (serversRefreshTimer !== null) {
        window.clearTimeout(serversRefreshTimer)
      }
      if (channelsRefreshTimer !== null) {
        window.clearTimeout(channelsRefreshTimer)
      }
      socket.off('user-servers-updated', handleUserServersUpdated)
      socket.off('server-channels-updated', handleServerChannelsUpdated)
    }
  }, [selectedServer])

  const fetchServers = async () => {
    const result = await serversList()
    if (result.success) {
      setServers(result.data)
      if (!selectedServer && result.data.length > 0) {
        setSelectedServer(result.data[0].serverId)
      }
    }
  }

  const fetchServerDetails = async (serverId: string) => {
    setFeedback(null)
    const result = await serversGet(serverId)
    if (result.success) {
      setServerDetails(result.data)
    }
  }

  const fetchChannels = async (serverId: string) => {
    const [serverChannelsResult, dmChannelsResult] = await Promise.all([
      channelsList(serverId),
      dmChannelsList(),
    ])

    if (serverChannelsResult.success) {
      const serverChannels = serverChannelsResult.data.map((channel) => ({
        ...channel,
        scope: channel.scope ?? 'server',
      }))
      const dmChannels: Channel[] = dmChannelsResult.success
        ? dmChannelsResult.data.map((dm) => ({
            channelId: dm.dmChannelId,
            name: dm.peerUsername,
            type: 'text',
            encryptionType: 'asymmetric',
            scope: 'dm',
            dmPeerUserId: dm.peerUserId,
            messageTTL: dm.messageTTL,
            isPrivate: true,
            unreadCount: dm.unreadCount,
            keyVersionId: null,
            keyVersion: null,
            createdAt: dm.lastMessageAt ?? new Date().toISOString(),
          }))
        : []

      setChannels((previous) => {
        const previousById = new Map(previous.map((channel) => [channel.channelId, channel]))
        return [...serverChannels, ...dmChannels].map((channel) => {
          const previousChannel = previousById.get(channel.channelId)
          if (!previousChannel) return channel
          return {
            ...channel,
            keyVersionId: channel.keyVersionId ?? previousChannel.keyVersionId ?? null,
            keyVersion: channel.keyVersion ?? previousChannel.keyVersion ?? null,
          }
        })
      })
    }
  }

  useEffect(() => {
    const checkServers = async () => {
      const result = await serversList()
      if (result.success && result.data.length > 0) {
        setServers(result.data)
        if (!selectedServer) {
          setSelectedServer(result.data[0].serverId)
        }
      }
    }
    checkServers()
  }, [])

  useEffect(() => {
    if (selectedServer) {
      setSelectedChannel(null)
      setOpenTextChannelIds([])
      if (pendingServerConfigOpenId === selectedServer) {
        setPanel('serverConfig')
        setPendingServerConfigOpenId(null)
      } else {
        setPanel('none')
      }
      setPendingMemberRemovalId(null)
      fetchServerDetails(selectedServer)
      fetchChannels(selectedServer)
    }
  }, [selectedServer, pendingServerConfigOpenId])

  useEffect(() => {
    const socket = getSocket()

    const handleVoicePresenceUpdated = (data: { channelId: string; users: VoiceParticipant[] }) => {
      setVoicePresenceByChannel((prev) => ({
        ...prev,
        [data.channelId]: data.users ?? [],
      }))
    }

    const handleVoicePresenceSnapshot = (data: {
      serverId: string
      channels: Array<{ channelId: string; users: VoiceParticipant[] }>
    }) => {
      if (!selectedServer || data.serverId !== selectedServer) {
        return
      }
      const next: Record<string, VoiceParticipant[]> = {}
      for (const channel of data.channels ?? []) {
        next[channel.channelId] = channel.users ?? []
      }
      setVoicePresenceByChannel(next)
    }

    socket.on('voice-presence-updated', handleVoicePresenceUpdated)
    socket.on('voice-presence-snapshot', handleVoicePresenceSnapshot)

    return () => {
      socket.off('voice-presence-updated', handleVoicePresenceUpdated)
      socket.off('voice-presence-snapshot', handleVoicePresenceSnapshot)
    }
  }, [selectedServer])

  useEffect(() => {
    const socket = getSocket()

    const handleServerMemberPresenceUpdated = (payload: { serverId: string; userId: string; status: string }) => {
      if (!selectedServer || payload.serverId !== selectedServer) {
        return
      }
      const isOnline = payload.status === 'online'
      setServerMemberPresenceById((current) => ({
        ...current,
        [payload.userId]: isOnline,
      }))
    }

    const handleServerMemberPresenceSnapshot = (payload: {
      serverId: string
      members: Array<{ userId: string; status: string }>
    }) => {
      if (!selectedServer || payload.serverId !== selectedServer) {
        return
      }
      const next: Record<string, boolean> = {}
      for (const member of payload.members ?? []) {
        next[member.userId] = member.status === 'online'
      }
      setServerMemberPresenceById(next)
    }

    socket.on('server-member-presence-updated', handleServerMemberPresenceUpdated)
    socket.on('server-member-presence-snapshot', handleServerMemberPresenceSnapshot)

    return () => {
      socket.off('server-member-presence-updated', handleServerMemberPresenceUpdated)
      socket.off('server-member-presence-snapshot', handleServerMemberPresenceSnapshot)
    }
  }, [selectedServer])

  useEffect(() => {
    if (!selectedServer) {
      setVoicePresenceByChannel({})
      setServerMemberPresenceById({})
      return
    }
    const socket = getSocket()
    setVoicePresenceByChannel({})
    setServerMemberPresenceById({})
    socket.emit('join-server-presence', { serverId: selectedServer })
    socket.emit('get-server-member-presence', { serverId: selectedServer })
    socket.emit('get-voice-presence', { serverId: selectedServer })
  }, [selectedServer])

  useEffect(() => {
    if (!voiceChannelId) {
      return
    }

    const socket = getSocket()
    const localIsSpeaking = liveKitParticipants[0]?.isSpeaking ?? false

    socket.emit('voice-state-updated', {
      channelId: voiceChannelId,
      isSuppressed: !!liveKitMuted,
      isDeafened: !!liveKitDeafened,
      isSpeaking: localIsSpeaking,
    })
  }, [voiceChannelId, liveKitMuted, liveKitDeafened, liveKitParticipants])

  useEffect(() => {
    if (!resolvedSelectedChannel || resolvedSelectedChannel.type !== 'text') {
      return
    }

    channelsMarkRead(resolvedSelectedChannel.channelId).catch(() => {
      // Best effort: el socket reconcilia unread igualment.
    })

    setChannels((prev) =>
      prev.map((c) =>
        c.channelId === resolvedSelectedChannel.channelId ? { ...c, unreadCount: 0 } : c
      )
    )
  }, [resolvedSelectedChannel?.channelId, resolvedSelectedChannel?.type])

  const handleUnreadUpdated = (channelId: string, unreadCount: number) => {
    setChannels((prev) =>
      prev.map((channel) =>
        channel.channelId === channelId
          ? { ...channel, unreadCount }
          : channel
      )
    )
  }

  const handleSelectServer = (serverId: string) => {
    setSelectedServer(serverId)
  }

  const handleOpenTextChannel = (channel: Channel) => {
    setSelectedChannel(channel)
    if (channel.type === 'text') {
      setOpenTextChannelIds((current) =>
        current.includes(channel.channelId) ? current : [...current, channel.channelId]
      )
    }
  }

  const handleStartDirectMessage = async (targetUserId: string, targetUsername: string) => {
    if (!currentDeviceId) {
      setFeedback('Falta el dispositiu actual per obrir el DM')
      return
    }

    const result = await dmChannelOpen(targetUserId, 86400)
    if (!result.success) {
      setFeedback(result.error.message)
      return
    }

    const dmChannel: Channel = {
      channelId: result.data.dmChannelId,
      name: targetUsername,
      type: 'text',
      encryptionType: 'asymmetric',
      scope: 'dm',
      dmPeerUserId: targetUserId,
      messageTTL: result.data.messageTTL,
      isPrivate: true,
      unreadCount: 0,
      keyVersionId: result.data.keyVersionId,
      keyVersion: result.data.keyVersion,
      createdAt: new Date().toISOString(),
    }

    setSelectedChannel(dmChannel)
    setChannels((current) => {
      const existingIndex = current.findIndex((channel) => channel.channelId === dmChannel.channelId)
      if (existingIndex >= 0) {
        const next = [...current]
        next[existingIndex] = { ...next[existingIndex], ...dmChannel }
        return next
      }
      return [...current, dmChannel]
    })
    setOpenTextChannelIds((current) => (
      current.includes(dmChannel.channelId) ? current : [...current, dmChannel.channelId]
    ))

    if (result.data.created) {
      try {
        const { generateSymmetricKey } = await import('../lib/crypto')
        const { storeChannelKey } = await import('../lib/storage')
        const channelKey = generateSymmetricKey()
        const keyVersion = result.data.keyVersion ?? 1
        await storeChannelKey(dmChannel.channelId, channelKey, 'asymmetric', keyVersion, result.data.keyVersionId)
        await distributeChannelKey(
          dmChannel.channelId,
          channelKey,
          keyVersion,
          result.data.keyVersionId,
          currentDeviceId,
        )
        setFeedback(`DM obert amb ${targetUsername}`)
      } catch (error) {
        const message = error instanceof Error ? error.message : 'No s\'ha pogut preparar la clau del DM'
        setFeedback(message)
      }
      return
    }

    setFeedback(`DM obert amb ${targetUsername}`)
  }

  const handleCloseTextTab = (channelId: string) => {
    setOpenTextChannelIds((current) => {
      const next = current.filter((id) => id !== channelId)
      if (selectedChannel?.channelId === channelId) {
        const fallbackId = next[next.length - 1] ?? null
        const fallbackChannel = fallbackId ? channels.find((channel) => channel.channelId === fallbackId) ?? null : null
        setSelectedChannel(fallbackChannel)
      }
      return next
    })
  }

  const handleRepairDmKey = async (channel: Channel) => {
    if (channel.scope !== 'dm') return
    if (!currentDeviceId) {
      setFeedback('Falta el dispositiu actual per arreglar claus del DM')
      return
    }

    setDmKeyActionBusy(true)
    try {
      let latest = await getLatestChannelKey(channel.channelId)
      let channelKey = latest?.keyBytes ?? await getChannelKey(channel.channelId)

      if (!channelKey) {
        channelKey = await ensureChannelKey(channel.channelId, channel.encryptionType, currentDeviceId)
        latest = await getLatestChannelKey(channel.channelId)
      }

      const keyVersionId = latest?.keyVersionId ?? channel.keyVersionId ?? null
      const keyVersion = latest?.keyVersion ?? channel.keyVersion ?? 1

      if (!channelKey) {
        throw new Error('No tens cap clau local per poder arreglar el DM')
      }

      if (!keyVersionId) {
        throw new Error('Falta keyVersionId; no es pot signar la redistribució de claus')
      }

      const distribution = await distributeChannelKey(
        channel.channelId,
        channelKey,
        keyVersion,
        keyVersionId,
        currentDeviceId,
      )

      setFeedback(formatDmRepairFeedback(distribution))
    } catch (error) {
      const message = error instanceof Error ? error.message : 'No s\'ha pogut arreglar la clau del DM'
      setFeedback(message)
    } finally {
      setDmKeyActionBusy(false)
    }
  }

  const handleRotateDmKey = async (channel: Channel) => {
    if (channel.scope !== 'dm') return
    if (!currentDeviceId) {
      setFeedback('Falta el dispositiu actual per rotar la clau del DM')
      return
    }

    setDmKeyActionBusy(true)
    try {
      const rotateResult = await dmChannelRotateKey(channel.channelId)
      if (!rotateResult.success) {
        setFeedback(rotateResult.error.message)
        return
      }

      const { generateSymmetricKey } = await import('../lib/crypto')
      const { storeChannelKey } = await import('../lib/storage')

      const channelKey = generateSymmetricKey()
      await storeChannelKey(
        channel.channelId,
        channelKey,
        'asymmetric',
        rotateResult.data.keyVersion,
        rotateResult.data.keyVersionId,
      )

      await distributeChannelKey(
        channel.channelId,
        channelKey,
        rotateResult.data.keyVersion,
        rotateResult.data.keyVersionId,
        currentDeviceId,
      )

      setSelectedChannel((current) => (
        current && current.channelId === channel.channelId
          ? {
              ...current,
              keyVersion: rotateResult.data.keyVersion,
              keyVersionId: rotateResult.data.keyVersionId,
            }
          : current
      ))

      setFeedback(`Clau del DM rotada a la versió ${rotateResult.data.keyVersion}`)
    } catch (error) {
      const message = error instanceof Error ? error.message : 'No s\'ha pogut rotar la clau del DM'
      setFeedback(message)
    } finally {
      setDmKeyActionBusy(false)
    }
  }

  // ── Voice connection logic (LiveKit real) ─────────────────────────────
  const handleVoiceChannelClick = async (channel: Channel) => {
    if (channel.type !== 'voice') return

    // If clicking the same channel we're in → leave
    if (voiceChannelId === channel.channelId) {
      disconnectLiveKit()
      getSocket().emit('leave-voice-channel', { channelId: channel.channelId })
      setVoiceChannelId(null)
      setVoiceChannelName('')
      setFeedback(`Has sortit del canal "${channel.name}"`)
      return
    }

    // If already in a different voice channel → leave first
    if (voiceChannelId) {
      disconnectLiveKit()
      getSocket().emit('leave-voice-channel', { channelId: voiceChannelId })
      setFeedback(`Has sortit del canal "${voiceChannelName}"`)
    }

    // Join the new voice channel
    setVoiceChannelId(channel.channelId)
    setVoiceChannelName(channel.name)

    try {
      let voiceChannelKey: Uint8Array | null = null
      if (channel.encryptionType !== 'none') {
        if (!currentDeviceId) {
          throw new Error('Falta el dispositiu actual per obtenir la clau del canal')
        }

        voiceChannelKey = await ensureChannelKey(
          channel.channelId,
          channel.encryptionType,
          currentDeviceId
        )

        if (!voiceChannelKey) {
          throw new Error('No s\'ha pogut obtenir la clau del canal de veu')
        }

        if (channel.encryptionType === 'asymmetric') {
          distributeChannelKey(channel.channelId, voiceChannelKey).catch(() => {})
        }
      }

      await connectLiveKit(channel.channelId, channel.name, {
        encryptionType: channel.encryptionType,
        channelKey: voiceChannelKey,
      })
      getSocket().emit('join-voice-channel', { channelId: channel.channelId })
      setFeedback(`T'has unit al canal de veu "${channel.name}"`)
    } catch (e: any) {
      setFeedback(`Error: ${e.message}`)
      setVoiceChannelId(null)
      setVoiceChannelName('')
    }
  }

  const handleLeaveVoiceChannel = () => {
    if (voiceChannelId) {
      getSocket().emit('leave-voice-channel', { channelId: voiceChannelId })
      disconnectLiveKit()
      setVoiceChannelId(null)
      setVoiceChannelName('')
      setFeedback(`Has sortit del canal "${voiceChannelName}"`)
    }
  }

  useEffect(() => {
    const handlePageClose = () => {
      if (voiceChannelId) {
        getSocket().emit('leave-voice-channel', { channelId: voiceChannelId })
      }
    }

    window.addEventListener('beforeunload', handlePageClose)
    window.addEventListener('pagehide', handlePageClose)

    return () => {
      window.removeEventListener('beforeunload', handlePageClose)
      window.removeEventListener('pagehide', handlePageClose)
    }
  }, [voiceChannelId])

  const handleToggleMute = () => {
    toggleLiveKitMute()
  }

  const handleToggleDeafen = () => {
    toggleLiveKitDeafen()
  }

  const handleToggleCamera = async () => {
    await toggleLiveKitCamera()
  }

  const handleToggleScreenShare = async () => {
    await toggleLiveKitScreenShare()
  }

  const handleCreateServer = async () => {
    if (!canCreateServer) {
      setFeedback('Ja has arribat al límit de servidors del teu tier')
      return
    }

    setShowCreateServer(true)
  }

  const handleCreateServerSubmit = async (name: string, iconUrl: string | null) => {
    const result = await serversCreate(name, iconUrl)
    if (result.success) {
      await fetchServers()
      setSelectedServer(result.data.serverId)
      setFeedback(`Servidor "${result.data.name}" creat`)
    } else {
      setFeedback(result.error.message)
    }
  }

  // Crear canal de text
  const handleCreateTextChannel = async (
    name: string,
    encryptionType: string,
    messageTTL: number | null,
    isPrivate: boolean
  ) => {
    if (!selectedServer) return
    const result = await channelsCreate(selectedServer, name, 'text', encryptionType, messageTTL, isPrivate)
    if (result.success) {
      if (result.data.encryptionType === 'asymmetric') {
        // Nivell 2: generar clau localment i distribuir-la a tots els membres.
        // Nivell 1 (symmetric) es genera al servidor.
        const { generateSymmetricKey } = await import('../lib/crypto')
        const { storeChannelKey } = await import('../lib/storage')
        const channelKey = generateSymmetricKey()
        await storeChannelKey(
          result.data.channelId,
          channelKey,
          result.data.encryptionType,
          result.data.keyVersion ?? 1,
          result.data.keyVersionId ?? null,
        )
        distributeChannelKey(
          result.data.channelId,
          channelKey,
          result.data.keyVersion ?? 1,
          result.data.keyVersionId ?? null,
          currentDeviceId ?? undefined,
        ).catch(() => {})
      }
      await fetchChannels(selectedServer)
      setSelectedChannel(result.data)
      setFeedback(`Canal "${result.data.name}" creat`)
    } else {
      setFeedback(result.error.message)
    }
  }

  // Crear canal de veu
  const handleCreateVoiceChannel = async (name: string, encryptionType: string, isPrivate: boolean) => {
    if (!selectedServer) return
    const result = await channelsCreate(selectedServer, name, 'voice', encryptionType, null, isPrivate)
    if (result.success) {
      if (result.data.encryptionType === 'asymmetric') {
        const { generateSymmetricKey } = await import('../lib/crypto')
        const { storeChannelKey } = await import('../lib/storage')
        const channelKey = generateSymmetricKey()
        await storeChannelKey(
          result.data.channelId,
          channelKey,
          result.data.encryptionType,
          result.data.keyVersion ?? 1,
          result.data.keyVersionId ?? null,
        )
        distributeChannelKey(
          result.data.channelId,
          channelKey,
          result.data.keyVersion ?? 1,
          result.data.keyVersionId ?? null,
          currentDeviceId ?? undefined,
        ).catch(() => {})
      }
      await fetchChannels(selectedServer)
      setSelectedChannel(result.data)
      setFeedback(`Canal "${result.data.name}" creat`)
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleInviteServerMember = async () => {
    if (!selectedServer) return
    setShowInviteServer(true)
  }

  const handleInviteServerSubmit = async (username: string) => {
    if (!selectedServer) return
    const result = await serverInviteMember(selectedServer, username)
    if (result.success) {
      setFeedback(`Invitació enviada a ${username}`)
      await fetchServerDetails(selectedServer)

      // Redistribuir claus de canals asimètrics on tenim clau local.
      // Evita deixar el nou membre sense bundle si no estàvem en el canal concret.
      if (channels.some((channel) => channel.encryptionType === 'asymmetric')) {
        const { getLatestChannelKey, getChannelKey } = await import('../lib/storage')
        await Promise.allSettled(
          channels
            .filter((channel) => channel.encryptionType === 'asymmetric')
            .map(async (channel) => {
              const latestKey = await getLatestChannelKey(channel.channelId)
              const channelKey = latestKey?.keyBytes ?? await getChannelKey(channel.channelId)
              if (!channelKey) return
              await distributeChannelKey(
                channel.channelId,
                channelKey,
                latestKey?.keyVersion ?? channel.keyVersion ?? 1,
                latestKey?.keyVersionId ?? channel.keyVersionId ?? null,
                currentDeviceId ?? undefined,
              )
            })
        )
      }
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleServerConfigInviteSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    const usernameToInvite = serverConfigInviteUsername.trim()
    if (!usernameToInvite) {
      setFeedback("El nom d'usuari és obligatori")
      return
    }
    await handleInviteServerSubmit(usernameToInvite)
    setServerConfigInviteUsername('')
  }

  const handleUpdateServerMemberRole = async (userId: string, role: 'admin' | 'member') => {
    if (!selectedServer) return
    const result = await serverUpdateMemberRole(selectedServer, userId, role)
    if ('error' in result) {
      setFeedback(result.error.message)
      return
    }
    await fetchServerDetails(selectedServer)
    setFeedback(`Rol actualitzat: ${result.data.username} → ${result.data.role}`)
  }

  const handleRemoveServerMember = async (userId: string) => {
    if (!selectedServer) return
    setPendingMemberRemovalId(null)
    const result = await serverRemoveMember(selectedServer, userId)
    if (!result.success) {
      setFeedback(result.error.message)
      return
    }
    await fetchServerDetails(selectedServer)
    await fetchServers()
    setFeedback('Membre eliminat del servidor')
  }

  const handleInviteChannelSubmit = async (username: string) => {
    const channel = resolvedSelectedChannel
    if (!channel) return
    const result = await channelInvite(channel.channelId, username)
    if (result.success) {
      setFeedback(`Invitació al canal enviada a ${username}`)
      if (channel.encryptionType === 'asymmetric') {
        try {
          let latestKey = await getLatestChannelKey(channel.channelId)
          let channelKey = latestKey?.keyBytes ?? await getChannelKey(channel.channelId)

          if (!channelKey && currentDeviceId) {
            channelKey = await ensureChannelKey(channel.channelId, channel.encryptionType, currentDeviceId)
            latestKey = await getLatestChannelKey(channel.channelId)
          }

          if (!channelKey) {
            throw new Error('No tens la clau local del canal per redistribuir-la')
          }

          await distributeChannelKey(
            channel.channelId,
            channelKey,
            latestKey?.keyVersion ?? channel.keyVersion ?? 1,
            latestKey?.keyVersionId ?? channel.keyVersionId ?? null,
            currentDeviceId ?? undefined,
          )
        } catch (err) {
          const msg = err instanceof Error ? err.message : 'No s\'ha pogut redistribuir la clau del canal'
          logger.error('[E2EE] Ha fallat la redistribució després de convidar al canal', {
            channelId: channel.channelId,
            username,
            currentDeviceId,
            error: msg,
          })
          setFeedback(`Invitació enviada, però ha fallat la redistribució de clau: ${msg}`)
        }
      }
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleManageDevices = () => {
    setShowDeviceKeysModal(true)
  }

  const handleManageChannelKeys = () => {
    setShowChannelKeysModal(true)
  }

  const handleManageFriends = () => {
    setShowFriendsModal(true)
  }

  const handleChangePassword = () => {
    setShowChangePasswordModal(true)
  }

  const handleManagePermissions = () => {
    setSelectedChannel(null)
    setPanel('permissions')
  }

  const handleManageAdminUsers = () => {
    if (!user?.isAdmin) {
      setFeedback('No tens permisos d\'administració global')
      return
    }
    setSelectedChannel(null)
    setPanel('adminUsers')
  }

  const handleOpenAdminServerConfig = (serverId: string) => {
    setSelectedChannel(null)
    if (selectedServer === serverId) {
      setPanel('serverConfig')
      return
    }

    setPendingServerConfigOpenId(serverId)
    setSelectedServer(serverId)
  }

  const refreshFriends = async () => {
    const result = await friendsList()
    if (result.success) {
      setFriends(result.data)
    }
  }

  const handleAddFriend = async (usernameToAdd: string) => {
    const result = await friendsAdd(usernameToAdd)
    if (result.success) {
      await refreshFriends()
      setFeedback(`Amic afegit: ${usernameToAdd}`)
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleRemoveFriend = async (userId: string) => {
    const result = await friendsRemove(userId)
    if (result.success) {
      await refreshFriends()
      setFeedback('Amic eliminat')
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleSearchUsers = async (query: string) => {
    const result = await usersSearch(query)
    return result.success ? result.data : []
  }

  const handleLogout = () => {
    disconnectLiveKit()
    disconnectSocket()
    logout()
  }

  // Obrir pestanya integrada de configuració de canal
  const handleConfigureChannel = (channel?: Channel) => {
    if (channel && (channel.permissionLevel ?? 0) < 3) {
      setFeedback('No tens permisos per configurar aquest canal')
      return
    }

    if (channel) {
      setSelectedChannel(channel)
    }
    setPanel('channelConfig')
  }

  // Desar canvis del canal
  const handleConfigureChannelSubmit = async (name: string, messageTTL: number | null, isPrivate: boolean) => {
    if (!selectedChannel) return
    const result = await channelsUpdate(selectedChannel.channelId, name, messageTTL, isPrivate)
    if (result.success) {
      await fetchChannels(selectedServer!)
      setSelectedChannel({
        ...selectedChannel,
        name,
        messageTTL,
        isPrivate,
      })
      setFeedback(`Canal "${name}" actualitzat`)
    } else {
      setFeedback(result.error.message)
    }
  }

  // Esborrar canal amb confirmació visual
  const handleDeleteChannel = async (channelId: string) => {
    const result = await channelDelete(channelId)
    if (result.success) {
      await fetchChannels(selectedServer!)
      setSelectedChannel(null)
      setPanel('serverConfig')
      setFeedback('Canal esborrat')
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleChannelConfigSave = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!resolvedSelectedChannel) return

    const trimmedName = channelConfigName.trim()
    if (!trimmedName) {
      setFeedback('El nom del canal és obligatori')
      return
    }

    const ttlRaw = channelConfigMessageTTL.trim()
    let parsedTtl: number | null = null
    if (ttlRaw) {
      const value = Number(ttlRaw)
      if (Number.isNaN(value) || value < 0) {
        setFeedback('TTL ha de ser un número positiu o buit')
        return
      }
      parsedTtl = value
    }

    await handleConfigureChannelSubmit(trimmedName, parsedTtl, channelConfigIsPrivate)
  }

  const handleChannelConfigInvite = async (event: React.FormEvent) => {
    event.preventDefault()
    const usernameToInvite = channelConfigInviteUsername.trim()
    if (!usernameToInvite) {
      setFeedback("El nom d'usuari és obligatori")
      return
    }
    await handleInviteChannelSubmit(usernameToInvite)
    setChannelConfigInviteUsername('')
  }

  // Gestiona les accions del menú del servidor
  const handleServerMenuAction = (action: ServerMenuAction) => {
    switch (action) {
      case 'config':
        setPanel('serverConfig')
        break
      case 'invite':
        setShowInviteServer(true)
        break
      case 'createText':
        setShowCreateTextChannel(true)
        break
      case 'createVoice':
        setShowCreateVoiceChannel(true)
        break
    }
  }

  const openTextTabs = channels.filter((channel) => channel.type === 'text' && openTextChannelIds.includes(channel.channelId))
  const activeVoiceChannel = voiceChannelId ? channels.find((channel) => channel.channelId === voiceChannelId) ?? null : null
  const textTabNodes = openTextTabs.map((channel) => (
    <div
      key={channel.channelId}
      className={`main-content-tab ${resolvedSelectedChannel?.channelId === channel.channelId ? 'active' : ''}`}
      onClick={() => {
        setPanel('none')
        setSelectedChannel(channel)
      }}
    >
      <span>#</span>
      <span>{channel.name}</span>
      {(channel.unreadCount ?? 0) > 0 && (
        <span className="channel-unread-badge">{channel.unreadCount}</span>
      )}
      <button
        type="button"
        className="main-content-tab-close"
        onClick={(event) => {
          event.stopPropagation()
          handleCloseTextTab(channel.channelId)
        }}
        title="Tancar pestanya"
      >
        ✕
      </button>
    </div>
  ))
  const voiceTabNode = activeVoiceChannel ? (
    <div
      className={`main-content-tab ${resolvedSelectedChannel?.channelId === activeVoiceChannel.channelId ? 'active' : ''}`}
      onClick={() => {
        setPanel('none')
        setSelectedChannel(activeVoiceChannel)
      }}
    >
      <span>🔊</span>
      <span>{activeVoiceChannel.name}</span>
      <button
        type="button"
        className="main-content-tab-close"
        onClick={(event) => {
          event.stopPropagation()
          handleLeaveVoiceChannel()
        }}
        title="Surt del canal de veu"
      >
        ✕
      </button>
    </div>
  ) : null

  const adminUsersTabNode = panel === 'adminUsers' ? (
    <div
      className="main-content-tab active"
      onClick={() => setPanel('adminUsers')}
    >
      <span>🛠️</span>
      <span>Usuaris</span>
      <button
        type="button"
        className="main-content-tab-close"
        onClick={(event) => {
          event.stopPropagation()
          setPanel('none')
        }}
        title="Tancar pestanya"
      >
        ✕
      </button>
    </div>
  ) : null

  const permissionsTabNode = panel === 'permissions' ? (
    <div
      className="main-content-tab active"
      onClick={() => setPanel('permissions')}
    >
      <span>🛡️</span>
      <span>Permisos</span>
      <button
        type="button"
        className="main-content-tab-close"
        onClick={(event) => {
          event.stopPropagation()
          setPanel('none')
        }}
        title="Tancar pestanya"
      >
        ✕
      </button>
    </div>
  ) : null

  const serverConfigTabNode = panel === 'serverConfig' ? (
    <div
      className="main-content-tab active"
      onClick={() => setPanel('serverConfig')}
    >
      <span>⚙️</span>
      <span>Servidor</span>
      <button
        type="button"
        className="main-content-tab-close"
        onClick={(event) => {
          event.stopPropagation()
          setPanel('none')
        }}
        title="Tancar pestanya"
      >
        ✕
      </button>
    </div>
  ) : null

  const channelConfigTabNode = panel === 'channelConfig' && resolvedSelectedChannel ? (
    <div
      className="main-content-tab active"
      onClick={() => setPanel('channelConfig')}
    >
      <span>#</span>
      <span>{resolvedSelectedChannel.name}</span>
      <button
        type="button"
        className="main-content-tab-close"
        onClick={(event) => {
          event.stopPropagation()
          setPanel('none')
        }}
        title="Tancar pestanya"
      >
        ✕
      </button>
    </div>
  ) : null

  const mergedVoiceParticipants = voiceChannelId
    ? liveKitParticipants.map((participant) => {
        const socketPresence = (voicePresenceByChannel[voiceChannelId] ?? []).find(
          (presence) => presence.userId === participant.userId
        )

        if (!socketPresence) {
          return participant
        }

        return {
          ...participant,
          isSuppressed: socketPresence.isSuppressed,
          isDeafened: socketPresence.isDeafened,
          isSpeaking: participant.isSpeaking || socketPresence.isSpeaking,
        }
      })
    : []

  // Build voice connection object from LiveKit state
  const voiceConnection = voiceChannelId
    ? {
        channelId: voiceChannelId,
        channelName: voiceChannelName,
        participants: mergedVoiceParticipants,
        isJoined: liveKitConnected,
        isMuted: liveKitMuted,
        isDeafened: liveKitDeafened,
        isCameraOn: liveKitCameraOn,
      }
    : null

  return (
    <div className="app-layout">
      <ServerBar
        servers={servers}
        selectedServer={selectedServer}
        onSelectServer={handleSelectServer}
        onCreateServer={handleCreateServer}
        canCreateServer={canCreateServer}
        onServerAction={handleServerMenuAction}
      />

      {selectedServer && (
        <ChannelList
          channels={channels}
          selectedChannel={selectedChannel}
          voiceConnection={voiceConnection}
          voicePresenceByChannel={voicePresenceByChannel}
          isMuted={liveKitMuted}
          isDeafened={liveKitDeafened}
          isCameraOn={liveKitCameraOn}
          isScreenSharing={liveKitScreenSharing}
          onToggleMute={handleToggleMute}
          onToggleDeafen={handleToggleDeafen}
          onToggleCamera={() => { void handleToggleCamera() }}
          onToggleScreenShare={() => { void handleToggleScreenShare() }}
          onSelectChannel={(channel) => {
            if (channel.type === 'voice') {
              handleVoiceChannelClick(channel)
            } else {
              handleOpenTextChannel(channel)
            }
          }}
          onStartDirectMessage={handleStartDirectMessage}
          onConfigureChannel={handleConfigureChannel}
          username={username}
          onLogout={handleLogout}
          onManageDevices={handleManageDevices}
          onManageChannelKeys={handleManageChannelKeys}
          onManageFriends={handleManageFriends}
          onChangePassword={handleChangePassword}
          onManagePermissions={handleManagePermissions}
          onManageAdminUsers={handleManageAdminUsers}
          onCreateTextChannel={canManageServer && canCreateTextChannel ? () => setShowCreateTextChannel(true) : undefined}
          onCreateVoiceChannel={canManageServer && canCreateVoiceChannel ? () => setShowCreateVoiceChannel(true) : undefined}
          canCreateTextChannel={canManageServer && canCreateTextChannel}
          canCreateVoiceChannel={canManageServer && canCreateVoiceChannel}
          canManageAdminUsers={user?.isAdmin ?? false}
          friends={friends}
          serverMembers={serverDetails?.members ?? []}
          serverMemberPresenceById={serverMemberPresenceById}
        />
      )}

      <div className="main-content-area">
        {selectedServer && (openTextTabs.length > 0 || activeVoiceChannel || panel === 'adminUsers' || panel === 'serverConfig' || panel === 'channelConfig' || panel === 'permissions') && (
          <div className="main-content-tabs">
            {textTabNodes}
            {voiceTabNode}
            {serverConfigTabNode}
            {channelConfigTabNode}
            {permissionsTabNode}
            {adminUsersTabNode}
          </div>
        )}

        {feedback && <div className="feedback-banner">{feedback}</div>}
        {liveKitError && <div className="feedback-banner" style={{ backgroundColor: '#ff4444' }}>{liveKitError}</div>}

        {panel === 'adminUsers' ? (
          <AdminUsersPanel
            isOpen={true}
            onClose={() => setPanel('none')}
            onFeedback={setFeedback}
            selectedServerId={selectedServer}
            availableServers={servers.map((server) => ({
              serverId: server.serverId,
              name: server.name,
              ownerId: server.ownerId,
              myRole: server.myRole,
              memberCount: server.memberCount,
            }))}
            onOpenServerConfig={handleOpenAdminServerConfig}
            onServerListRefresh={fetchServers}
          />
        ) : panel === 'permissions' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Permisos i accessos</h3>
              <button className="admin-panel-tab" onClick={() => setPanel('serverConfig')}>
                Tornar a servidor
              </button>
            </div>

            <PermissionsPanel
              server={serverDetails}
              channels={channels}
              currentDeviceId={currentDeviceId}
            />
          </div>
        ) : panel === 'serverConfig' && serverDetails ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Configuració del servidor</h3>
              <div style={{ display: 'flex', gap: '8px' }}>
                <button className="admin-panel-tab" onClick={() => setPanel('permissions')}>
                  Permisos usuaris/canals
                </button>
                <button className="admin-panel-tab" onClick={() => setPanel('none')}>
                  Tancar
                </button>
              </div>
            </div>

            <p>
              <strong>{serverDetails.name}</strong> · Rol: {serverDetails.myRole}
            </p>

            {canManageServer && (
              <form onSubmit={handleServerConfigInviteSubmit} className="modal-form" style={{ marginTop: '12px', marginBottom: '12px' }}>
                <div className="form-group">
                  <label htmlFor="integrated-server-invite">Convidar membre al servidor</label>
                  <input
                    id="integrated-server-invite"
                    type="text"
                    value={serverConfigInviteUsername}
                    onChange={(e) => setServerConfigInviteUsername(e.target.value)}
                    placeholder="Nom d'usuari"
                  />
                </div>
                <div className="modal-form-actions" style={{ justifyContent: 'flex-end' }}>
                  <button type="submit" className="admin-panel-tab">
                    Convidar
                  </button>
                </div>
              </form>
            )}

            <div className="server-members" style={{ marginTop: '12px' }}>
              <h4>Canals del servidor</h4>
              <ul>
                {channels.filter((channel) => channel.scope !== 'dm').map((channel) => (
                  <li key={channel.channelId} style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
                    <span>{channel.type === 'voice' ? '🔊' : '#'} {channel.name}</span>
                    <button
                      type="button"
                      className="admin-panel-tab"
                      onClick={() => handleConfigureChannel(channel)}
                    >
                      Configurar
                    </button>
                  </li>
                ))}
              </ul>
            </div>

            <div className="server-members" style={{ marginTop: '12px' }}>
              <h4>Membres</h4>
              <ul>
                {serverDetails.members.map((member) => {
                  const isCurrentUser = member.userId === user?.userId
                  const canModify = canManageServer && member.role !== 'owner' && !isCurrentUser
                  return (
                    <li key={member.userId} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '8px' }}>
                      <span>{member.username} — {member.role}</span>
                      {canManageServer && (
                        <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                          {member.role !== 'owner' && (
                            <select
                              aria-label={`Rol de ${member.username}`}
                              value={member.role}
                              onChange={(e) => {
                                const nextRole = e.target.value as 'admin' | 'member'
                                void handleUpdateServerMemberRole(member.userId, nextRole)
                              }}
                              className="device-keys-input"
                              style={{ width: '120px', padding: '4px 8px' }}
                            >
                              <option value="member">member</option>
                              <option value="admin">admin</option>
                            </select>
                          )}
                          <button
                            type="button"
                            className="admin-panel-tab"
                            style={{ borderColor: '#ff6b6b', color: '#ff6b6b' }}
                            disabled={!canModify}
                            onClick={() => {
                              setPendingMemberRemovalId(member.userId)
                            }}
                          >
                            Eliminar
                          </button>
                          {pendingMemberRemovalId === member.userId && (
                            <>
                              <button
                                type="button"
                                className="admin-panel-tab"
                                onClick={() => {
                                  void handleRemoveServerMember(member.userId)
                                }}
                              >
                                Confirmar
                              </button>
                              <button
                                type="button"
                                className="admin-panel-tab"
                                onClick={() => setPendingMemberRemovalId(null)}
                              >
                                Cancel·lar
                              </button>
                            </>
                          )}
                        </div>
                      )}
                    </li>
                  )
                })}
              </ul>
            </div>
          </div>
        ) : panel === 'channelConfig' && resolvedSelectedChannel ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Configuració integrada del canal</h3>
              <button className="admin-panel-tab" onClick={() => setPanel('serverConfig')}>
                Tornar a servidor
              </button>
            </div>

            <form onSubmit={handleChannelConfigSave} className="modal-form" style={{ marginBottom: '12px' }}>
              <div className="form-group">
                <label htmlFor="integrated-channel-name">Nom del canal</label>
                <input
                  id="integrated-channel-name"
                  type="text"
                  value={channelConfigName}
                  onChange={(e) => setChannelConfigName(e.target.value)}
                  maxLength={30}
                />
              </div>
              <div className="form-group">
                <label htmlFor="integrated-channel-ttl">TTL (segons)</label>
                <input
                  id="integrated-channel-ttl"
                  type="number"
                  value={channelConfigMessageTTL}
                  onChange={(e) => setChannelConfigMessageTTL(e.target.value)}
                  placeholder="Sense límit"
                  min="0"
                />
              </div>
              <label style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '12px' }}>
                <input
                  type="checkbox"
                  checked={channelConfigIsPrivate}
                  onChange={(e) => setChannelConfigIsPrivate(e.target.checked)}
                />
                Canal privat
              </label>
              <div className="modal-form-actions" style={{ justifyContent: 'space-between' }}>
                <button type="button" className="admin-panel-tab" onClick={() => setPanel('none')}>
                  Tancar
                </button>
                <button type="submit" className="admin-panel-tab active">
                  Desar canvis
                </button>
              </div>
            </form>

            <form onSubmit={handleChannelConfigInvite} className="modal-form" style={{ marginBottom: '12px' }}>
              <div className="form-group">
                <label htmlFor="integrated-channel-invite">Convidar usuari</label>
                <input
                  id="integrated-channel-invite"
                  type="text"
                  value={channelConfigInviteUsername}
                  onChange={(e) => setChannelConfigInviteUsername(e.target.value)}
                  placeholder="Nom d'usuari"
                />
              </div>
              <div className="modal-form-actions" style={{ justifyContent: 'flex-end' }}>
                <button type="submit" className="admin-panel-tab">
                  Convidar
                </button>
              </div>
            </form>

            <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
              <button
                type="button"
                className="admin-panel-tab"
                style={{ borderColor: '#ff6b6b', color: '#ff6b6b' }}
                onClick={() => handleDeleteChannel(resolvedSelectedChannel.channelId)}
              >
                Esborrar canal
              </button>
            </div>

            {canViewChannelExplicitPermissions && (
              <div className="server-members" style={{ marginTop: '12px' }}>
                <h4>Permisos del canal (efectius + override explícit)</h4>
                {channelExplicitPermissionsLoading ? (
                  <p>Carregant permisos explícits...</p>
                ) : channelPermissionRows.length > 0 ? (
                  <div style={{ overflowX: 'auto' }}>
                    <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
                      <thead>
                        <tr>
                          <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>Usuari</th>
                          <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>Permís efectiu</th>
                          <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>Origen</th>
                          <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>Override explícit</th>
                        </tr>
                      </thead>
                      <tbody>
                        {channelPermissionRows.map((entry) => (
                          <tr key={entry.userId}>
                            <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>{entry.username}</td>
                            <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>
                              {entry.effectivePermission} ({entry.effectiveLevel})
                            </td>
                            <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>
                              <span
                                style={{
                                  display: 'inline-block',
                                  padding: '2px 8px',
                                  borderRadius: '999px',
                                  fontSize: '11px',
                                  border: '1px solid var(--bg-active)',
                                  background: entry.explicitLevel === null ? 'transparent' : 'var(--bg-active)',
                                }}
                              >
                                {entry.explicitLevel === null ? 'heretat' : 'explícit'}
                              </span>
                            </td>
                            <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>
                              <select
                                value={entry.explicitLevel === null ? 'inherited' : String(entry.explicitLevel)}
                                onChange={(event) => {
                                  void handleUpdateChannelExplicitPermission(entry.userId, event.target.value)
                                }}
                                disabled={updatingChannelPermissionUserId === entry.userId}
                                className="device-keys-input"
                                style={{ width: '180px', padding: '4px 8px' }}
                              >
                                <option value="inherited">heretat (rol servidor)</option>
                                <option value="1">read (1)</option>
                                <option value="2">write (2)</option>
                                <option value="3">manage (3)</option>
                              </select>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                ) : (
                  <p>No hi ha dades de permisos visibles en aquest canal.</p>
                )}
              </div>
            )}
          </div>
        ) : resolvedSelectedChannel ? (
          <>
            <MainContent
              channel={resolvedSelectedChannel}
              voiceConnection={voiceConnection}
              currentDeviceId={currentDeviceId}
              onLeaveVoice={handleLeaveVoiceChannel}
              onUnreadUpdated={handleUnreadUpdated}
              localVideoTrack={localVideoTrack}
              localScreenTrack={localScreenTrack}
              remoteVideoTracks={remoteVideoTracks}
              voiceAsTextMode={voiceAsTextMode}
              onToggleVoiceAsTextMode={() => setVoiceAsTextMode((prev) => !prev)}
              onDmRepairKey={handleRepairDmKey}
              onDmRotateKey={handleRotateDmKey}
              dmKeyActionBusy={dmKeyActionBusy}
            />
          </>
        ) : voiceConnection ? (
          <MainContent
            channel={null}
            voiceConnection={voiceConnection}
            currentDeviceId={currentDeviceId}
            onLeaveVoice={handleLeaveVoiceChannel}
            localVideoTrack={localVideoTrack}
            localScreenTrack={localScreenTrack}
            remoteVideoTracks={remoteVideoTracks}
            voiceAsTextMode={voiceAsTextMode}
            onToggleVoiceAsTextMode={() => setVoiceAsTextMode((prev) => !prev)}
          />
        ) : (
          <div className="welcome-screen">
            <h1>Benvingut/da, {username}!</h1>
            <p>Selecciona un servidor i un canal per començar.</p>
          </div>
        )}

        {/* ── Modals ─────────────────────────────────── */}
        <CreateServerModal
          isOpen={showCreateServer}
          onClose={() => setShowCreateServer(false)}
          onCreate={handleCreateServerSubmit}
        />

        <CreateTextChannelModal
          isOpen={showCreateTextChannel}
          onClose={() => setShowCreateTextChannel(false)}
          onCreate={handleCreateTextChannel}
        />

        <CreateVoiceChannelModal
          isOpen={showCreateVoiceChannel}
          onClose={() => setShowCreateVoiceChannel(false)}
          onCreate={handleCreateVoiceChannel}
        />

        {selectedServer && (
          <InviteMemberModal
            isOpen={showInviteServer}
            onClose={() => setShowInviteServer(false)}
            onInvite={handleInviteServerSubmit}
            inviteType="server"
            targetName={selectedServerInfo?.name ?? selectedServer}
          />
        )}

        {user && (
          <DeviceKeysModal
            isOpen={showDeviceKeysModal}
            onClose={() => setShowDeviceKeysModal(false)}
            currentDeviceId={currentDeviceId}
            channels={channels}
            devices={user.devices ?? []}
          />
        )}

        {user && (
          <ChannelKeysModal
            isOpen={showChannelKeysModal}
            onClose={() => setShowChannelKeysModal(false)}
            channels={channels}
          />
        )}

        <FriendsModal
          isOpen={showFriendsModal}
          onClose={() => setShowFriendsModal(false)}
          friends={friends}
          onAddFriend={handleAddFriend}
          onRemoveFriend={handleRemoveFriend}
          onSearchUsers={handleSearchUsers}
        />

        <ChangePasswordModal
          isOpen={showChangePasswordModal}
          onClose={() => setShowChangePasswordModal(false)}
        />
      </div>
    </div>
  )
}
