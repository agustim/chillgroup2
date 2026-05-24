import React, { useEffect, useMemo, useState } from 'react'
import { useAuth } from '../contexts/AuthContext'
import { ServerBar } from './sidebar/ServerBar'
import { ChannelList } from './sidebar/ChannelList'
import { MainContent } from './main/MainContent'
import { CreateServerModal } from './modals/CreateServerModal'
import { CreateTextChannelModal } from './modals/CreateTextChannelModal'
import { CreateVoiceChannelModal } from './modals/CreateVoiceChannelModal'
import { InviteMemberModal } from './modals/InviteMemberModal'
import { ConfigureChannelModal } from './modals/ConfigureChannelModal'
import { DeviceKeysModal } from './modals/DeviceKeysModal'
import { ChannelKeysModal } from './modals/ChannelKeysModal'
import { PermissionsModal } from './modals/PermissionsModal'
import { ChangePasswordModal } from './modals/ChangePasswordModal'
import { FriendsModal } from './modals/FriendsModal'
import { useLiveKit } from '../hooks/useLiveKit'
import { Channel, Friend, FriendPresence, Server, ServerFullInfo, VoiceParticipant } from '../types'
import { getSocket } from '../lib/socket'
import { hasLocalDeviceKeypair } from '../lib/device-keys'
import { ensureChannelKey, distributeChannelKey } from '../lib/channel-crypto'
import { getChannelKey, getLatestChannelKey } from '../lib/storage'
import {
  serverInviteMember,
  serversCreate,
  serversGet,
  serversList,
  channelsCreate,
  channelsList,
  channelsMarkRead,
  channelInvite,
  channelsUpdate,
  channelDelete,
} from '../lib/api'

interface AppLayoutProps {
  username: string
  onLogout?: () => void
}

type PanelType = 'none' | 'serverConfig' | 'channelConfig' | 'devices'
type ServerMenuAction = 'config' | 'invite' | 'createText' | 'createVoice' | null

interface FriendRecord extends Friend {
  isOnline: boolean
}

const FRIENDS_STORAGE_KEY = 'chillgroup-friends'

function normalizeName(name: string): string {
  return name.trim().toLowerCase()
}

function friendKey(friend: Friend) {
  return normalizeName(friend.username)
}

function readStoredFriends(): FriendRecord[] {
  try {
    const raw = localStorage.getItem(FRIENDS_STORAGE_KEY)
    if (!raw) return []
    const parsed = JSON.parse(raw) as Array<Partial<FriendRecord>>
    return parsed
      .filter((friend): friend is FriendRecord => !!friend?.userId && !!friend?.username)
      .map((friend) => ({
        userId: friend.userId,
        username: friend.username,
        isOnline: !!friend.isOnline,
      }))
  } catch {
    return []
  }
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
  const [friends, setFriends] = useState<FriendRecord[]>([])
  const [friendsLoaded, setFriendsLoaded] = useState(false)
  const [panel, setPanel] = useState<PanelType>('none')
  const [feedback, setFeedback] = useState<string | null>(null)
  
  // Modal states
  const [showCreateServer, setShowCreateServer] = useState(false)
  const [showCreateTextChannel, setShowCreateTextChannel] = useState(false)
  const [showCreateVoiceChannel, setShowCreateVoiceChannel] = useState(false)
  const [showInviteServer, setShowInviteServer] = useState(false)
  const [showConfigureChannel, setShowConfigureChannel] = useState(false)
  const [showDeviceKeysModal, setShowDeviceKeysModal] = useState(false)
  const [showChannelKeysModal, setShowChannelKeysModal] = useState(false)
  const [showPermissionsModal, setShowPermissionsModal] = useState(false)
  const [showChangePasswordModal, setShowChangePasswordModal] = useState(false)
  const [showFriendsModal, setShowFriendsModal] = useState(false)
  
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
    setFriends(readStoredFriends())
    setFriendsLoaded(true)
  }, [])

  useEffect(() => {
    if (!friendsLoaded) return
    try {
      localStorage.setItem(FRIENDS_STORAGE_KEY, JSON.stringify(friends))
    } catch {
      // Best effort: la llista d'amics és una preferència local.
    }
  }, [friends, friendsLoaded])

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

  const selectedServerInfo = selectedServer ? servers.find((server) => server.serverId === selectedServer) : undefined
  const resolvedSelectedChannel = selectedChannel
    ? channels.find((channel) => channel.channelId === selectedChannel.channelId) ?? selectedChannel
    : null
  const canManageServer =
    selectedServerInfo?.myRole === 'owner' ||
    selectedServerInfo?.myRole === 'admin' ||
    serverDetails?.myRole === 'owner' ||
    serverDetails?.myRole === 'admin'

  const activeFriendIds = useMemo(() => {
    const ids = new Set<string>()
    for (const participants of Object.values(voicePresenceByChannel)) {
      for (const participant of participants) {
        ids.add(participant.userId)
      }
    }
    return ids
  }, [voicePresenceByChannel])

  const friendsWithPresence = useMemo(
    () => friends.map((friend) => ({ ...friend, isOnline: activeFriendIds.has(friend.userId) })),
    [friends, activeFriendIds]
  )

  const knownUsers = useMemo<FriendPresence[]>(() => {
    const known = new Map<string, FriendPresence>()

    for (const member of serverDetails?.members ?? []) {
      if (member.userId === user?.userId) {
        continue
      }
      known.set(member.userId, {
        userId: member.userId,
        username: member.username,
        isOnline: activeFriendIds.has(member.userId),
      })
    }

    for (const participants of Object.values(voicePresenceByChannel)) {
      for (const participant of participants) {
        if (participant.userId === user?.userId || known.has(participant.userId)) {
          continue
        }
        known.set(participant.userId, {
          userId: participant.userId,
          username: participant.username,
          isOnline: true,
        })
      }
    }

    return Array.from(known.values()).sort((left, right) => left.username.localeCompare(right.username))
  }, [activeFriendIds, serverDetails?.members, user?.userId, voicePresenceByChannel])

  useEffect(() => {
    if (!friendsLoaded || friends.length > 0 || (serverDetails?.members ?? []).length === 0) {
      return
    }

    const seedFriends = (serverDetails?.members ?? [])
      .filter((member) => member.userId !== user?.userId)
      .slice(0, 6)
      .map((member) => ({
        userId: member.userId,
        username: member.username,
        isOnline: activeFriendIds.has(member.userId),
      }))

    if (seedFriends.length > 0) {
      setFriends(seedFriends)
    }
  }, [activeFriendIds, friends.length, friendsLoaded, serverDetails?.members, user?.userId])

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
    const result = await channelsList(serverId)
    if (result.success) {
      setChannels(result.data)
    }
  }

  useEffect(() => {
    const checkServers = async () => {
      const result = await serversList()
      if (result.success && result.data.length === 0) {
        setShowCreateServer(true)
      } else if (result.success && result.data.length > 0) {
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
      setPanel('none')
      fetchServerDetails(selectedServer)
      fetchChannels(selectedServer)
    }
  }, [selectedServer])

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
    if (!selectedServer) {
      setVoicePresenceByChannel({})
      return
    }
    const socket = getSocket()
    setVoicePresenceByChannel({})
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
          console.error('[E2EE] Ha fallat la redistribució després de convidar al canal', {
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
    setShowPermissionsModal(true)
  }

  const handleAddFriend = (friend: Friend) => {
    const normalized = friendKey(friend)
    setFriends((current) => {
      if (current.some((entry) => friendKey(entry) === normalized)) {
        return current.map((entry) => (
          friendKey(entry) === normalized
            ? { ...entry, username: friend.username }
            : entry
        ))
      }

      return [
        ...current,
        {
          userId: friend.userId,
          username: friend.username,
          isOnline: activeFriendIds.has(friend.userId),
        },
      ]
    })
  }

  const handleRemoveFriend = (userId: string) => {
    setFriends((current) => current.filter((friend) => friend.userId !== userId))
  }

  // Obrir modal de configuració de canal
  const handleConfigureChannel = (channel?: Channel) => {
    if (channel) {
      setSelectedChannel(channel)
    }
    setShowConfigureChannel(true)
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
      setFeedback('Canal esborrat')
    } else {
      setFeedback(result.error.message)
    }
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
      onClick={() => setSelectedChannel(channel)}
    >
      <span>#</span>
      <span>{channel.name}</span>
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
      onClick={() => setSelectedChannel(activeVoiceChannel)}
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
          onConfigureChannel={handleConfigureChannel}
          username={username}
          onLogout={logout}
          onManageDevices={handleManageDevices}
          onManageChannelKeys={handleManageChannelKeys}
          onManageFriends={handleManageFriends}
          onChangePassword={handleChangePassword}
          onManagePermissions={handleManagePermissions}
          onCreateTextChannel={canManageServer ? () => setShowCreateTextChannel(true) : undefined}
          onCreateVoiceChannel={canManageServer ? () => setShowCreateVoiceChannel(true) : undefined}
          canCreateChannel={canManageServer}
          friends={friendsWithPresence}
        />
      )}

      <div className="main-content-area">
        {selectedServer && (openTextTabs.length > 0 || activeVoiceChannel) && (
          <div className="main-content-tabs">
            {textTabNodes}
            {voiceTabNode}
          </div>
        )}

        {feedback && <div className="feedback-banner">{feedback}</div>}
        {liveKitError && <div className="feedback-banner" style={{ backgroundColor: '#ff4444' }}>{liveKitError}</div>}

        {resolvedSelectedChannel ? (
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

        {panel === 'serverConfig' && serverDetails && (
          <div className="panel server-config-panel">
            <h3>Configuració del servidor</h3>
            <p>
              <strong>{serverDetails.name}</strong> · Rol: {serverDetails.myRole}
            </p>
            <div className="server-members">
              <h4>Membres</h4>
              <ul>
                {serverDetails.members.map((member) => (
                  <li key={member.userId}>
                    {member.username} — {member.role}
                  </li>
                ))}
              </ul>
            </div>
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

        <ConfigureChannelModal
          isOpen={showConfigureChannel}
          onClose={() => setShowConfigureChannel(false)}
          channel={resolvedSelectedChannel}
          onUpdate={handleConfigureChannelSubmit}
          onDelete={handleDeleteChannel}
          onInviteChannel={handleInviteChannelSubmit}
        />

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

        <PermissionsModal
          isOpen={showPermissionsModal}
          onClose={() => setShowPermissionsModal(false)}
          server={serverDetails}
          channels={channels}
          currentDeviceId={currentDeviceId}
        />

        <FriendsModal
          isOpen={showFriendsModal}
          onClose={() => setShowFriendsModal(false)}
          friends={friendsWithPresence}
          knownUsers={knownUsers}
          onAddFriend={handleAddFriend}
          onRemoveFriend={handleRemoveFriend}
        />

        <ChangePasswordModal
          isOpen={showChangePasswordModal}
          onClose={() => setShowChangePasswordModal(false)}
        />
      </div>
    </div>
  )
}
