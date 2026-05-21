import React, { useEffect, useState } from 'react'
import { useAuth } from '../contexts/AuthContext'
import { ServerBar } from './sidebar/ServerBar'
import { ChannelList } from './sidebar/ChannelList'
import { MainContent } from './main/MainContent'
import { CreateServerModal } from './modals/CreateServerModal'
import { CreateTextChannelModal } from './modals/CreateTextChannelModal'
import { CreateVoiceChannelModal } from './modals/CreateVoiceChannelModal'
import { InviteMemberModal } from './modals/InviteMemberModal'
import { ConfigureChannelModal } from './modals/ConfigureChannelModal'
import { useLiveKit } from '../hooks/useLiveKit'
import { Channel, Server, ServerFullInfo, VoiceParticipant } from '../types'
import { getSocket } from '../lib/socket'
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
type ServerMenuAction = 'config' | 'invite' | 'createText' | 'createVoice' | 'devices' | null

export function AppLayout({ username, onLogout }: AppLayoutProps) {
  const { user, logout } = useAuth()
  const [servers, setServers] = useState<Server[]>([])
  const [selectedServer, setSelectedServer] = useState<string | null>(null)
  const [serverDetails, setServerDetails] = useState<ServerFullInfo | null>(null)
  const [channels, setChannels] = useState<Channel[]>([])
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null)
  const [voiceChannelId, setVoiceChannelId] = useState<string | null>(null)
  const [voiceChannelName, setVoiceChannelName] = useState<string>('')
  const [voicePresenceByChannel, setVoicePresenceByChannel] = useState<Record<string, VoiceParticipant[]>>({})
  const [panel, setPanel] = useState<PanelType>('none')
  const [feedback, setFeedback] = useState<string | null>(null)
  
  // Modal states
  const [showCreateServer, setShowCreateServer] = useState(false)
  const [showCreateTextChannel, setShowCreateTextChannel] = useState(false)
  const [showCreateVoiceChannel, setShowCreateVoiceChannel] = useState(false)
  const [showInviteServer, setShowInviteServer] = useState(false)
  const [showInviteChannel, setShowInviteChannel] = useState(false)
  const [showConfigureChannel, setShowConfigureChannel] = useState(false)
  
  // LiveKit hook
  const {
    isConnected: liveKitConnected,
    isPublishing,
    isMuted: liveKitMuted,
    isDeafened: liveKitDeafened,
    isCameraOn: liveKitCameraOn,
    localVideoTrack,
    remoteVideoTracks,
    participants: liveKitParticipants,
    connectToChannel: connectLiveKit,
    disconnect: disconnectLiveKit,
    toggleMute: toggleLiveKitMute,
    toggleDeafen: toggleLiveKitDeafen,
    toggleCamera: toggleLiveKitCamera,
    error: liveKitError,
  } = useLiveKit()

  // Auto-dismiss feedback
  useEffect(() => {
    if (feedback) {
      const timer = setTimeout(() => setFeedback(null), 3000)
      return () => clearTimeout(timer)
    }
  }, [feedback])

  const selectedServerInfo = selectedServer ? servers.find((server) => server.serverId === selectedServer) : undefined
  const canManageServer =
    selectedServerInfo?.myRole === 'owner' ||
    selectedServerInfo?.myRole === 'admin' ||
    serverDetails?.myRole === 'owner' ||
    serverDetails?.myRole === 'admin'

  const canManageChannel = serverDetails?.myRole === 'owner' || serverDetails?.myRole === 'admin' || false

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
    if (!selectedChannel || selectedChannel.type !== 'text') {
      return
    }

    channelsMarkRead(selectedChannel.channelId).catch(() => {
      // Best effort: el socket reconcilia unread igualment.
    })

    setChannels((prev) =>
      prev.map((c) =>
        c.channelId === selectedChannel.channelId ? { ...c, unreadCount: 0 } : c
      )
    )
  }, [selectedChannel?.channelId, selectedChannel?.type])

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
      await connectLiveKit(channel.channelId, channel.name)
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
    if (liveKitMuted !== undefined) {
      toggleLiveKitMute()
    }
  }

  const handleToggleDeafen = () => {
    toggleLiveKitDeafen()
  }

  const handleToggleCamera = async () => {
    await toggleLiveKitCamera()
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
  const handleCreateTextChannel = async (name: string, encryptionType: string, messageTTL: number | null) => {
    if (!selectedServer) return
    const result = await channelsCreate(selectedServer, name, 'text', encryptionType, messageTTL)
    if (result.success) {
      await fetchChannels(selectedServer)
      setSelectedChannel(result.data)
      setFeedback(`Canal "${result.data.name}" creat`)
    } else {
      setFeedback(result.error.message)
    }
  }

  // Crear canal de veu
  const handleCreateVoiceChannel = async (name: string, encryptionType: string) => {
    if (!selectedServer) return
    const result = await channelsCreate(selectedServer, name, 'voice', encryptionType)
    if (result.success) {
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
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleInviteChannel = async () => {
    if (!selectedChannel) return
    setShowInviteChannel(true)
  }

  const handleInviteChannelSubmit = async (username: string) => {
    const channel = selectedChannel
    if (!channel) return
    const result = await channelInvite(channel.channelId, username)
    if (result.success) {
      setFeedback(`Invitació al canal enviada a ${username}`)
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleManageDevices = () => {
    setPanel((current) => (current === 'devices' ? 'none' : 'devices'))
  }

  // Obrir modal de configuració de canal
  const handleConfigureChannel = () => {
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
      case 'devices':
        setPanel('devices')
        break
    }
  }

  // Build voice connection object from LiveKit state
  const voiceConnection = voiceChannelId
    ? {
        channelId: voiceChannelId,
        channelName: voiceChannelName,
        participants: liveKitParticipants,
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
          onSelectChannel={(channel) => {
            if (channel.type === 'voice') {
              handleVoiceChannelClick(channel)
            } else {
              setSelectedChannel(channel)
            }
          }}
          username={username}
          onLogout={logout}
          onManageDevices={handleManageDevices}
          onCreateTextChannel={canManageServer ? () => setShowCreateTextChannel(true) : undefined}
          onCreateVoiceChannel={canManageServer ? () => setShowCreateVoiceChannel(true) : undefined}
          canCreateChannel={canManageServer}
        />
      )}

      <div className="main-content-area">
        {selectedServer && (
          <div className="server-actions">
            {canManageServer && (
              <>
                <button onClick={() => setPanel('serverConfig')}>Configurar servidor</button>
                <button onClick={handleInviteServerMember}>Convidar al servidor</button>
              </>
            )}
            <button onClick={handleManageDevices}>Gestió dispositius</button>
          </div>
        )}

        {feedback && <div className="feedback-banner">{feedback}</div>}
        {liveKitError && <div className="feedback-banner" style={{ backgroundColor: '#ff4444' }}>{liveKitError}</div>}

        {selectedChannel ? (
          <>
            <MainContent
              channel={selectedChannel}
              voiceConnection={voiceConnection}
              onToggleMute={handleToggleMute}
              onToggleDeafen={handleToggleDeafen}
              onToggleCamera={handleToggleCamera}
              onLeaveVoice={handleLeaveVoiceChannel}
              onUnreadUpdated={handleUnreadUpdated}
              onConfigureChannel={handleConfigureChannel}
              onInviteChannel={handleInviteChannel}
              canManageChannel={canManageChannel}
              localVideoTrack={localVideoTrack}
              remoteVideoTracks={remoteVideoTracks}
            />
          </>
        ) : voiceConnection ? (
          <MainContent
            channel={null}
            voiceConnection={voiceConnection}
            onToggleMute={handleToggleMute}
            onToggleDeafen={handleToggleDeafen}
            onToggleCamera={handleToggleCamera}
            onLeaveVoice={handleLeaveVoiceChannel}
            localVideoTrack={localVideoTrack}
            remoteVideoTracks={remoteVideoTracks}
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

        {panel === 'devices' && user && (
          <div className="panel devices-panel">
            <h3>Gestió de dispositius</h3>
            <ul>
              {user.devices.map((device) => (
                <li key={device.deviceId}>
                  <strong>{device.label}</strong> ({device.deviceId})
                  <div className="device-status">
                    {device.revoked ? 'Revocat' : 'Actiu'} · darrera connexió: {device.lastSeen}
                  </div>
                </li>
              ))}
            </ul>
            <p className="panel-note">Les accions de dispositiu estaran disponibles quan el backend implementi l'API corresponent.</p>
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

        {selectedChannel && (
          <InviteMemberModal
            isOpen={showInviteChannel}
            onClose={() => setShowInviteChannel(false)}
            onInvite={handleInviteChannelSubmit}
            inviteType="channel"
            targetName={selectedChannel.name}
          />
        )}

        <ConfigureChannelModal
          isOpen={showConfigureChannel}
          onClose={() => setShowConfigureChannel(false)}
          channel={selectedChannel}
          onUpdate={handleConfigureChannelSubmit}
          onDelete={handleDeleteChannel}
        />
      </div>
    </div>
  )
}
