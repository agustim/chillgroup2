import React, { useEffect, useState } from 'react'
import { useAuth } from '../contexts/AuthContext'
import { ServerBar } from './sidebar/ServerBar'
import { ChannelList } from './sidebar/ChannelList'
import { MainContent } from './main/MainContent'
import { CreateServerPanel } from './modals/CreateServerModal'
import { CreateTextChannelPanel } from './modals/CreateTextChannelModal'
import { CreateVoiceChannelPanel } from './modals/CreateVoiceChannelModal'
import { InviteMemberModal } from './modals/InviteMemberModal'
import { DeviceKeysPanel } from './modals/DeviceKeysModal'
import { ChannelKeysPanel } from './modals/ChannelKeysModal'
import { PermissionsPanel } from './modals/PermissionsModal'
import { ChangePasswordPanel } from './modals/ChangePasswordModal'
import { FriendsPanel } from './modals/FriendsModal'
import { AdminUsersPanel } from './main/AdminUsersPanel'
import { LogoutBackupModal } from './modals/LogoutBackupModal'
import { ServerInvitationsModal } from './modals/ServerInvitationsModal'
import { PanelTab } from './shared/PanelTab'
import { ServerConfigPanel } from './main/ServerConfigPanel'
import { ChannelConfigPanel } from './main/ChannelConfigPanel'
import { LeaveServerModal } from './modals/LeaveServerModal'
import { usePresence } from '../hooks/usePresence'
import { useChannelConfig } from '../hooks/useChannelConfig'
import { useLiveKit } from '../hooks/useLiveKit'
import { Channel, FriendPresence, Server, ServerFullInfo, VoiceParticipant } from '../types'
import { disconnectSocket, getSocket } from '../lib/socket'
import { hasLocalDeviceKeypair } from '../lib/device-keys'
import { ensureChannelKey, distributeChannelKey, syncChannelKeys, forceRefreshChannelKey } from '../lib/channel-crypto'
import { getChannelKey, getLatestChannelKey } from '../lib/storage'
import {
  friendsAdd,
  friendsList,
  friendsRemove,
  dmChannelOpen,
  dmChannelsList,
  serverCreateInvitation,
  serverLeave,
  serverUpdateMemberRole,
  serverRemoveMember,
  serversCreate,
  serversGet,
  serversList,
  channelsCreate,
  channelsList,
  channelInvite,
  channelDelete,
  dmChannelRotateKey,
  channelRotateKey,
  userLimitsGet,
} from '../lib/api'
import { logger } from '../lib/logger'

interface AppLayoutProps {
  username: string
  onLogout?: () => void
}

export type PanelType = 'none' | 'serverConfig' | 'channelConfig' | 'devices' | 'adminUsers' | 'permissions' | 'friends' | 'createServer' | 'changePassword' | 'channelKeys' | 'createTextChannel' | 'createVoiceChannel'
type ServerMenuAction = 'config' | 'invite' | 'createText' | 'createVoice' | 'leave' | null

function formatRepairFeedback(result: {
  discoveredDevices: string[]
  skippedSelfDevices: string[]
  skippedMissingKemDevices: string[]
  uploadedBundleDevices: string[]
  failedDevices: Array<{ deviceId: string; reason: string }>
}): string {
  const parts = [
    `Devices vistos: ${result.discoveredDevices.length}`,
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
  const [isChannelListCollapsed, setIsChannelListCollapsed] = useState(false)
  const [openTextChannelIds, setOpenTextChannelIds] = useState<string[]>([])
  const [voiceAsTextMode, setVoiceAsTextMode] = useState(false)
  const [voiceChannelId, setVoiceChannelId] = useState<string | null>(null)
  const [voiceChannelName, setVoiceChannelName] = useState<string>('')
  const [friends, setFriends] = useState<FriendPresence[]>([])
  const [panel, setPanel] = useState<PanelType>('none')
  const [feedback, setFeedback] = useState<string | null>(null)
  const [quotaWarning, setQuotaWarning] = useState<string | null>(null)
  const [dmKeyActionBusy, setDmKeyActionBusy] = useState(false)
  const [canCreateServer, setCanCreateServer] = useState(true)
  const [canCreateTextChannel, setCanCreateTextChannel] = useState(true)
  const [canCreateVoiceChannel, setCanCreateVoiceChannel] = useState(true)

  // Modal states
  const [showInviteServer, setShowInviteServer] = useState(false)
  const [leaveServerConfirm, setLeaveServerConfirm] = useState<{ serverId: string; serverName: string; isLastAdmin: boolean } | null>(null)
  const [leaveServerBusy, setLeaveServerBusy] = useState(false)
  const [showServerInvitations, setShowServerInvitations] = useState(false)
  const [pendingInvitationCount, setPendingInvitationCount] = useState(0)
  const [pendingServerConfigOpenId, setPendingServerConfigOpenId] = useState<string | null>(null)
  const [pendingMemberRemovalId, setPendingMemberRemovalId] = useState<string | null>(null)
  const [showLogoutModal, setShowLogoutModal] = useState(false)

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

  // Presence hook
  const { voicePresenceByChannel, serverMemberPresenceById } = usePresence(selectedServer)

  // Computed values (used by useChannelConfig below)
  const selectedServerInfo = selectedServer ? servers.find((server) => server.serverId === selectedServer) : undefined
  const resolvedSelectedChannel = selectedChannel
    ? channels.find((channel) => channel.channelId === selectedChannel.channelId) ?? selectedChannel
    : null
  const canManageServer =
    selectedServerInfo?.myRole === 'owner' ||
    selectedServerInfo?.myRole === 'admin' ||
    serverDetails?.myRole === 'owner' ||
    serverDetails?.myRole === 'admin'

  // Channel config hook
  const {
    channelConfigName,
    setChannelConfigName,
    channelConfigMessageTTL,
    setChannelConfigMessageTTL,
    channelConfigIsPrivate,
    setChannelConfigIsPrivate,
    channelExplicitPermissions,
    channelExplicitPermissionsLoading,
    canViewChannelExplicitPermissions,
    channelPermissionRows,
    updatingChannelPermissionUserId,
    handleUpdateChannelExplicitPermission,
    handleChannelConfigSave,
  } = useChannelConfig({
    isActive: panel === 'channelConfig',
    channel: resolvedSelectedChannel,
    selectedServer,
    fetchChannels,
    setSelectedChannel,
    setFeedback,
  })

  // Auto-dismiss feedback
  useEffect(() => {
    if (feedback) {
      const isError = feedback.includes('ha fallat') || feedback.startsWith('Error:')
      const timer = setTimeout(() => setFeedback(null), isError ? 12000 : 3000)
      return () => clearTimeout(timer)
    }
  }, [feedback])

  useEffect(() => {
    if (!user || !currentDeviceId) return

    let cancelled = false
    hasLocalDeviceKeypair(currentDeviceId)
      .then((hasKeypair) => {
        if (!cancelled && !hasKeypair) setPanel('devices')
      })
      .catch(() => {
        if (!cancelled) setPanel('devices')
      })

    return () => { cancelled = true }
  }, [user, currentDeviceId])

  useEffect(() => {
    if (!selectedChannel || selectedChannel.encryptionType === 'none' || !currentDeviceId) return
    syncChannelKeys(selectedChannel.channelId, selectedChannel.encryptionType as import('../types').EncryptionType, currentDeviceId).catch(() => {})
  }, [selectedChannel?.channelId, currentDeviceId])

  useEffect(() => {
    let cancelled = false
    const loadFriends = async () => {
      const result = await friendsList()
      if (!cancelled && result.success) setFriends(result.data)
    }
    void loadFriends()
    return () => { cancelled = true }
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
    return () => { cancelled = true }
  }, [user?.userId])

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
    return () => { socket.off('friend-presence-updated', handleFriendPresenceUpdated) }
  }, [])

  useEffect(() => {
    const socket = getSocket()
    let serversRefreshTimer: number | null = null
    let channelsRefreshTimer: number | null = null

    const handleUserServersUpdated = async () => {
      if (serversRefreshTimer !== null) window.clearTimeout(serversRefreshTimer)
      serversRefreshTimer = window.setTimeout(() => { void fetchServers() }, 250)
    }

    const handleServerChannelsUpdated = async (payload: { serverId?: string }) => {
      if (!selectedServer || payload.serverId !== selectedServer) return
      if (channelsRefreshTimer !== null) window.clearTimeout(channelsRefreshTimer)
      channelsRefreshTimer = window.setTimeout(() => { void fetchChannels(selectedServer) }, 250)
    }

    const handleServerInvitation = () => { setPendingInvitationCount((n) => n + 1) }

    const handleQuotaWarning = (data: { type: string; threshold: number; usedBytes: number; maxBytes: number }) => {
      const pct = Math.round((data.usedBytes / data.maxBytes) * 100)
      const typeLabel = data.type === 'storage' ? 'emmagatzematge' : data.type
      setQuotaWarning(`Avís: has usat el ${pct}% de la quota de ${typeLabel} del teu pla.`)
    }

    socket.on('user-servers-updated', handleUserServersUpdated)
    socket.on('server-channels-updated', handleServerChannelsUpdated)
    socket.on('server-invitation', handleServerInvitation)
    socket.on('quota_warning', handleQuotaWarning)

    return () => {
      if (serversRefreshTimer !== null) window.clearTimeout(serversRefreshTimer)
      if (channelsRefreshTimer !== null) window.clearTimeout(channelsRefreshTimer)
      socket.off('user-servers-updated', handleUserServersUpdated)
      socket.off('server-channels-updated', handleServerChannelsUpdated)
      socket.off('server-invitation', handleServerInvitation)
      socket.off('quota_warning', handleQuotaWarning)
    }
  }, [selectedServer])

  useEffect(() => {
    if (!voiceChannelId) return
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
    if (!resolvedSelectedChannel || resolvedSelectedChannel.type !== 'text') return
    setChannels((prev) =>
      prev.map((c) =>
        c.channelId === resolvedSelectedChannel.channelId ? { ...c, unreadCount: 0 } : c
      )
    )
  }, [resolvedSelectedChannel?.channelId, resolvedSelectedChannel?.type])

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
    if (result.success) setServerDetails(result.data)
  }

  async function fetchChannels(serverId: string) {
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
        if (!selectedServer) setSelectedServer(result.data[0].serverId)
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
    const handlePageClose = () => {
      if (voiceChannelId) getSocket().emit('leave-voice-channel', { channelId: voiceChannelId })
    }
    window.addEventListener('beforeunload', handlePageClose)
    window.addEventListener('pagehide', handlePageClose)
    return () => {
      window.removeEventListener('beforeunload', handlePageClose)
      window.removeEventListener('pagehide', handlePageClose)
    }
  }, [voiceChannelId])

  const handleUnreadUpdated = (channelId: string, unreadCount: number) => {
    setChannels((prev) =>
      prev.map((channel) =>
        channel.channelId === channelId ? { ...channel, unreadCount } : channel
      )
    )
  }

  const handleSelectServer = (serverId: string) => { setSelectedServer(serverId) }

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

  const handleRepairKey = async (channel: Channel) => {
    if (channel.encryptionType !== 'asymmetric') return
    if (!currentDeviceId) {
      setFeedback('Falta el dispositiu actual per arreglar les claus')
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

      if (!channelKey) throw new Error('No tens cap clau local per poder arreglar el canal')
      if (!keyVersionId) throw new Error('Falta keyVersionId; no es pot signar la redistribució de claus')

      const distribution = await distributeChannelKey(
        channel.channelId,
        channelKey,
        keyVersion,
        keyVersionId,
        currentDeviceId,
      )
      setFeedback(formatRepairFeedback(distribution))
    } catch (error) {
      const message = error instanceof Error ? error.message : 'No s\'ha pogut arreglar les claus del canal'
      setFeedback(message)
    } finally {
      setDmKeyActionBusy(false)
    }
  }

  const handleRotateKey = async (channel: Channel) => {
    if (!currentDeviceId && channel.encryptionType === 'asymmetric') {
      setFeedback('Falta el dispositiu actual per rotar la clau')
      return
    }

    setDmKeyActionBusy(true)
    try {
      let keyVersion: number
      let keyVersionId: string

      if (channel.scope === 'dm') {
        const rotateResult = await dmChannelRotateKey(channel.channelId)
        if (!rotateResult.success) { setFeedback(rotateResult.error.message); return }
        keyVersion = rotateResult.data.keyVersion
        keyVersionId = rotateResult.data.keyVersionId
      } else {
        const rotateResult = await channelRotateKey(channel.channelId)
        if (!rotateResult.success) { setFeedback(rotateResult.error.message); return }
        keyVersion = rotateResult.data.keyVersion
        keyVersionId = rotateResult.data.keyVersionId
      }

      if (channel.encryptionType === 'asymmetric' && currentDeviceId) {
        const { generateSymmetricKey } = await import('../lib/crypto')
        const { storeChannelKey } = await import('../lib/storage')

        const channelKey = generateSymmetricKey()
        await storeChannelKey(channel.channelId, channelKey, 'asymmetric', keyVersion, keyVersionId)
        await distributeChannelKey(channel.channelId, channelKey, keyVersion, keyVersionId, currentDeviceId)
      } else if (channel.encryptionType === 'symmetric' && currentDeviceId) {
        await forceRefreshChannelKey(channel.channelId, 'symmetric', currentDeviceId)
      }

      const updateChannel = (c: Channel) =>
        c.channelId === channel.channelId ? { ...c, keyVersion, keyVersionId } : c
      setSelectedChannel((current) => (current ? updateChannel(current) : current))
      setChannels((prev) => prev.map(updateChannel))
      setFeedback(`Clau rotada a la versió ${keyVersion}`)
    } catch (error) {
      const message = error instanceof Error ? error.message : 'No s\'ha pogut rotar la clau'
      setFeedback(message)
    } finally {
      setDmKeyActionBusy(false)
    }
  }

  const handleVoiceChannelClick = async (channel: Channel) => {
    if (channel.type !== 'voice') return

    if (voiceChannelId === channel.channelId) {
      disconnectLiveKit()
      getSocket().emit('leave-voice-channel', { channelId: channel.channelId })
      setVoiceChannelId(null)
      setVoiceChannelName('')
      setFeedback(`Has sortit del canal "${channel.name}"`)
      return
    }

    if (voiceChannelId) {
      disconnectLiveKit()
      getSocket().emit('leave-voice-channel', { channelId: voiceChannelId })
      setFeedback(`Has sortit del canal "${voiceChannelName}"`)
    }

    setVoiceChannelId(channel.channelId)
    setVoiceChannelName(channel.name)

    try {
      let voiceChannelKey: Uint8Array | null = null
      if (channel.encryptionType !== 'none') {
        if (!currentDeviceId) throw new Error('Falta el dispositiu actual per obtenir la clau del canal')

        voiceChannelKey = await ensureChannelKey(channel.channelId, channel.encryptionType, currentDeviceId)
        if (!voiceChannelKey) throw new Error('No s\'ha pogut obtenir la clau del canal de veu')

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

  const handleToggleMute = () => { toggleLiveKitMute() }
  const handleToggleDeafen = () => { toggleLiveKitDeafen() }
  const handleToggleCamera = async () => { await toggleLiveKitCamera() }
  const handleToggleScreenShare = async () => { await toggleLiveKitScreenShare() }

  const handleCreateServer = async () => {
    if (!canCreateServer) {
      setFeedback('Ja has arribat al límit de servidors del teu tier')
      return
    }
    setPanel('createServer')
  }

  const handleCreateServerSubmit = async (name: string, iconUrl: string | null) => {
    const result = await serversCreate(name, iconUrl)
    if (result.success) {
      await fetchServers()
      setSelectedServer(result.data.serverId)
      setPanel('none')
      setFeedback(`Servidor "${result.data.name}" creat`)
    } else {
      setFeedback(result.error.message)
    }
  }

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
        const { generateSymmetricKey } = await import('../lib/crypto')
        const { storeChannelKey } = await import('../lib/storage')
        const channelKey = generateSymmetricKey()
        await storeChannelKey(result.data.channelId, channelKey, result.data.encryptionType, result.data.keyVersion ?? 1, result.data.keyVersionId ?? null)
        distributeChannelKey(result.data.channelId, channelKey, result.data.keyVersion ?? 1, result.data.keyVersionId ?? null, currentDeviceId ?? undefined).catch(() => {})
      }
      await fetchChannels(selectedServer)
      setSelectedChannel(result.data)
      setFeedback(`Canal "${result.data.name}" creat`)
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleCreateVoiceChannel = async (name: string, encryptionType: string, isPrivate: boolean) => {
    if (!selectedServer) return
    const result = await channelsCreate(selectedServer, name, 'voice', encryptionType, null, isPrivate)
    if (result.success) {
      if (result.data.encryptionType === 'asymmetric') {
        const { generateSymmetricKey } = await import('../lib/crypto')
        const { storeChannelKey } = await import('../lib/storage')
        const channelKey = generateSymmetricKey()
        await storeChannelKey(result.data.channelId, channelKey, result.data.encryptionType, result.data.keyVersion ?? 1, result.data.keyVersionId ?? null)
        distributeChannelKey(result.data.channelId, channelKey, result.data.keyVersion ?? 1, result.data.keyVersionId ?? null, currentDeviceId ?? undefined).catch(() => {})
      }
      await fetchChannels(selectedServer)
      setSelectedChannel(result.data)
      setFeedback(`Canal "${result.data.name}" creat`)
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleInviteServerSubmit = async (username: string) => {
    if (!selectedServer) return
    const result = await serverCreateInvitation(selectedServer, username)
    if (result.success) {
      setFeedback(`Invitació enviada a ${username}. Haurà d'acceptar-la per unir-se al servidor.`)
    } else {
      setFeedback((result as any).error?.message ?? 'Error enviant la invitació')
    }
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

          if (!channelKey) throw new Error('No tens la clau local del canal per redistribuir-la')

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

  const handleManageDevices = () => { setPanel('devices') }
  const handleManageChannelKeys = () => { setPanel('channelKeys') }
  const handleManageFriends = () => { setPanel('friends') }
  const handleChangePassword = () => { setPanel('changePassword') }

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
    if (result.success) setFriends(result.data)
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
    const { usersSearch } = await import('../lib/api')
    const result = await usersSearch(query)
    return result.success ? result.data : []
  }

  const handleLogout = () => { setShowLogoutModal(true) }

  const handleLogoutConfirm = () => {
    setShowLogoutModal(false)
    disconnectLiveKit()
    disconnectSocket()
    logout()
  }

  const handleConfigureChannel = (channel?: Channel) => {
    if (channel && (channel.permissionLevel ?? 0) < 3) {
      setFeedback('No tens permisos per configurar aquest canal')
      return
    }
    if (channel) setSelectedChannel(channel)
    setPanel('channelConfig')
  }

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

  const handleServerMenuAction = (action: ServerMenuAction) => {
    switch (action) {
      case 'config':
      case 'invite':
        setPanel('serverConfig')
        break
      case 'createText':
        setPanel('createTextChannel')
        break
      case 'createVoice':
        setPanel('createVoiceChannel')
        break
      case 'leave': {
        const server = servers.find((s) => s.serverId === selectedServer)
        if (!server) break
        setLeaveServerConfirm({ serverId: server.serverId, serverName: server.name, isLastAdmin: false })
        break
      }
    }
  }

  const handleLeaveServerConfirm = async (force: boolean) => {
    if (!leaveServerConfirm) return
    setLeaveServerBusy(true)
    const result = await serverLeave(leaveServerConfirm.serverId, force)
    setLeaveServerBusy(false)

    if (!result.success) {
      if ((result as any).error?.code === 2009) {
        setLeaveServerConfirm({ ...leaveServerConfirm, isLastAdmin: true })
        return
      }
      setFeedback((result as any).error?.message ?? 'Error en sortir del servidor')
      setLeaveServerConfirm(null)
      return
    }

    setLeaveServerConfirm(null)
    setServers((prev) => prev.filter((s) => s.serverId !== leaveServerConfirm.serverId))
    if (selectedServer === leaveServerConfirm.serverId) {
      setSelectedServer(null)
      setChannels([])
      setSelectedChannel(null)
    }
  }

  const openTextTabs = channels.filter((channel) => channel.type === 'text' && openTextChannelIds.includes(channel.channelId))
  const activeVoiceChannel = voiceChannelId ? channels.find((channel) => channel.channelId === voiceChannelId) ?? null : null

  const mergedVoiceParticipants = voiceChannelId
    ? liveKitParticipants.map((participant) => {
        const socketPresence = (voicePresenceByChannel[voiceChannelId] ?? []).find(
          (presence) => presence.userId === participant.userId
        )
        if (!socketPresence) return participant
        return {
          ...participant,
          isSuppressed: socketPresence.isSuppressed,
          isDeafened: socketPresence.isDeafened,
          isSpeaking: participant.isSpeaking || socketPresence.isSpeaking,
        }
      })
    : []

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

  const showTabBar = panel !== 'none' || openTextTabs.length > 0 || !!activeVoiceChannel

  return (
    <div className="app-layout">
      <ServerBar
        servers={servers}
        selectedServer={selectedServer}
        onSelectServer={handleSelectServer}
        onCreateServer={handleCreateServer}
        canCreateServer={canCreateServer}
        isChannelListCollapsed={isChannelListCollapsed}
        onShowChannelList={() => setIsChannelListCollapsed(false)}
        onServerAction={handleServerMenuAction}
      />

      {selectedServer && !isChannelListCollapsed && (
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
          onShowInvitations={() => setShowServerInvitations(true)}
          pendingInvitationCount={pendingInvitationCount}
          onChangePassword={handleChangePassword}
          onManagePermissions={handleManagePermissions}
          onManageAdminUsers={handleManageAdminUsers}
          onCollapseList={() => setIsChannelListCollapsed(true)}
          onCreateTextChannel={canManageServer && canCreateTextChannel ? () => setPanel('createTextChannel') : undefined}
          onCreateVoiceChannel={canManageServer && canCreateVoiceChannel ? () => setPanel('createVoiceChannel') : undefined}
          canCreateTextChannel={canManageServer && canCreateTextChannel}
          canCreateVoiceChannel={canManageServer && canCreateVoiceChannel}
          canManageAdminUsers={user?.isAdmin ?? false}
          friends={friends}
          serverMembers={serverDetails?.members ?? []}
          serverMemberPresenceById={serverMemberPresenceById}
        />
      )}

      <div className="main-content-area">
        {showTabBar && (
          <div className="main-content-tabs">
            {openTextTabs.map((channel) => (
              <div
                key={channel.channelId}
                className={`main-content-tab ${resolvedSelectedChannel?.channelId === channel.channelId ? 'active' : ''}`}
                onClick={() => { setPanel('none'); setSelectedChannel(channel) }}
              >
                <span>#</span>
                <span>{channel.name}</span>
                {(channel.unreadCount ?? 0) > 0 && (
                  <span className="channel-unread-badge">{channel.unreadCount}</span>
                )}
                <button
                  type="button"
                  className="main-content-tab-close"
                  onClick={(event) => { event.stopPropagation(); handleCloseTextTab(channel.channelId) }}
                  title="Tancar pestanya"
                >
                  ✕
                </button>
              </div>
            ))}

            {activeVoiceChannel && (
              <div
                className={`main-content-tab ${resolvedSelectedChannel?.channelId === activeVoiceChannel.channelId ? 'active' : ''}`}
                onClick={() => { setPanel('none'); setSelectedChannel(activeVoiceChannel) }}
              >
                <span>🔊</span>
                <span>{activeVoiceChannel.name}</span>
                <button
                  type="button"
                  className="main-content-tab-close"
                  onClick={(event) => { event.stopPropagation(); handleLeaveVoiceChannel() }}
                  title="Surt del canal de veu"
                >
                  ✕
                </button>
              </div>
            )}

            <PanelTab icon="⚙️" label="Servidor" isActive={panel === 'serverConfig'} onClick={() => setPanel('serverConfig')} onClose={() => setPanel('none')} />
            <PanelTab icon="#" label={resolvedSelectedChannel?.name ?? ''} isActive={panel === 'channelConfig' && !!resolvedSelectedChannel} onClick={() => setPanel('channelConfig')} onClose={() => setPanel('none')} />
            <PanelTab icon="🛡️" label="Permisos" isActive={panel === 'permissions'} onClick={() => setPanel('permissions')} onClose={() => setPanel('none')} />
            <PanelTab icon="🛠️" label="Usuaris" isActive={panel === 'adminUsers'} onClick={() => setPanel('adminUsers')} onClose={() => setPanel('none')} />
            <PanelTab icon="➕" label="Nou servidor" isActive={panel === 'createServer'} onClick={() => setPanel('createServer')} onClose={() => setPanel('none')} />
            <PanelTab icon="👥" label="Amics" isActive={panel === 'friends'} onClick={() => setPanel('friends')} onClose={() => setPanel('none')} />
            <PanelTab icon="📱" label="Dispositius" isActive={panel === 'devices'} onClick={() => setPanel('devices')} onClose={() => setPanel('none')} />
            <PanelTab icon="🔒" label="Password" isActive={panel === 'changePassword'} onClick={() => setPanel('changePassword')} onClose={() => setPanel('none')} />
            <PanelTab icon="🔑" label="Claus" isActive={panel === 'channelKeys'} onClick={() => setPanel('channelKeys')} onClose={() => setPanel('none')} />
            <PanelTab icon="#" label="Nou text" isActive={panel === 'createTextChannel'} onClick={() => setPanel('createTextChannel')} onClose={() => setPanel('none')} />
            <PanelTab icon="🔊" label="Nou veu" isActive={panel === 'createVoiceChannel'} onClick={() => setPanel('createVoiceChannel')} onClose={() => setPanel('none')} />
          </div>
        )}

        {feedback && <div className="feedback-banner">{feedback}</div>}
        {liveKitError && <div className="feedback-banner" style={{ backgroundColor: '#ff4444' }}>{liveKitError}</div>}
        {quotaWarning && (
          <div className="feedback-banner feedback-banner--warning" style={{ backgroundColor: '#f59e0b', color: '#1f2937' }}>
            {quotaWarning}
            <button onClick={() => setQuotaWarning(null)} style={{ marginLeft: 8, background: 'none', border: 'none', cursor: 'pointer', fontWeight: 'bold' }}>✕</button>
          </div>
        )}

        {panel === 'createServer' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Crear servidor</h3>
            </div>
            <CreateServerPanel onClose={() => setPanel('none')} onCreate={handleCreateServerSubmit} />
          </div>
        ) : panel === 'friends' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Gestio d'amics</h3>
            </div>
            <FriendsPanel
              friends={friends}
              onAddFriend={handleAddFriend}
              onRemoveFriend={handleRemoveFriend}
              onSearchUsers={handleSearchUsers}
            />
          </div>
        ) : panel === 'devices' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Gestio de dispositius</h3>
            </div>
            <DeviceKeysPanel currentDeviceId={currentDeviceId} channels={channels} devices={user?.devices ?? []} />
          </div>
        ) : panel === 'changePassword' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Canviar password</h3>
            </div>
            <ChangePasswordPanel onClose={() => setPanel('none')} />
          </div>
        ) : panel === 'channelKeys' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Gestió de claus de canals</h3>
            </div>
            <ChannelKeysPanel channels={channels} serverName={serverDetails?.name} />
          </div>
        ) : panel === 'createTextChannel' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Crear canal de text</h3>
            </div>
            <CreateTextChannelPanel onClose={() => setPanel('none')} onCreate={handleCreateTextChannel} />
          </div>
        ) : panel === 'createVoiceChannel' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Crear canal de veu</h3>
            </div>
            <CreateVoiceChannelPanel onClose={() => setPanel('none')} onCreate={handleCreateVoiceChannel} />
          </div>
        ) : panel === 'adminUsers' ? (
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
            <PermissionsPanel server={serverDetails} channels={channels} currentDeviceId={currentDeviceId} />
          </div>
        ) : panel === 'serverConfig' && serverDetails ? (
          <ServerConfigPanel
            serverDetails={serverDetails}
            channels={channels}
            canManageServer={!!canManageServer}
            currentUserId={user?.userId}
            pendingMemberRemovalId={pendingMemberRemovalId}
            onSetPendingMemberRemovalId={setPendingMemberRemovalId}
            onSearchUsers={handleSearchUsers}
            onInviteServerSubmit={handleInviteServerSubmit}
            onConfigureChannel={handleConfigureChannel}
            onUpdateServerMemberRole={handleUpdateServerMemberRole}
            onRemoveServerMember={handleRemoveServerMember}
            onOpenPermissions={() => setPanel('permissions')}
          />
        ) : panel === 'channelConfig' && resolvedSelectedChannel ? (
          <ChannelConfigPanel
            channel={resolvedSelectedChannel}
            channelConfigName={channelConfigName}
            setChannelConfigName={setChannelConfigName}
            channelConfigMessageTTL={channelConfigMessageTTL}
            setChannelConfigMessageTTL={setChannelConfigMessageTTL}
            channelConfigIsPrivate={channelConfigIsPrivate}
            setChannelConfigIsPrivate={setChannelConfigIsPrivate}
            onSave={handleChannelConfigSave}
            onSearchUsers={handleSearchUsers}
            onInviteChannelSubmit={handleInviteChannelSubmit}
            onDeleteChannel={handleDeleteChannel}
            onBackToServer={() => setPanel('serverConfig')}
            canViewChannelExplicitPermissions={canViewChannelExplicitPermissions}
            channelExplicitPermissionsLoading={channelExplicitPermissionsLoading}
            channelPermissionRows={channelPermissionRows}
            updatingChannelPermissionUserId={updatingChannelPermissionUserId}
            onUpdateChannelExplicitPermission={handleUpdateChannelExplicitPermission}
          />
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
              onRepairKey={handleRepairKey}
              onRotateKey={handleRotateKey}
              keyActionBusy={dmKeyActionBusy}
              isChannelAdmin={canManageServer}
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

        {selectedServer && (
          <InviteMemberModal
            isOpen={showInviteServer}
            onClose={() => setShowInviteServer(false)}
            onInvite={handleInviteServerSubmit}
            onSearchUsers={handleSearchUsers}
            inviteType="server"
            targetName={selectedServerInfo?.name ?? selectedServer}
          />
        )}

        {showServerInvitations && (
          <ServerInvitationsModal
            onClose={() => { setShowServerInvitations(false); setPendingInvitationCount(0) }}
            onAccepted={() => {
              setShowServerInvitations(false)
              setPendingInvitationCount(0)
              void (async () => {
                const result = await serversList()
                if (result.success) setServers(result.data)
              })()
            }}
          />
        )}

        {showLogoutModal && (
          <LogoutBackupModal
            username={username}
            onConfirm={handleLogoutConfirm}
            onCancel={() => setShowLogoutModal(false)}
          />
        )}

        <LeaveServerModal
          confirm={leaveServerConfirm}
          busy={leaveServerBusy}
          onConfirm={handleLeaveServerConfirm}
          onCancel={() => setLeaveServerConfirm(null)}
        />
      </div>
    </div>
  )
}
