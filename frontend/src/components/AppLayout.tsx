import React, { useEffect, useState } from 'react'
import { useAuth } from '../contexts/AuthContext'
import { ServerBar } from './sidebar/ServerBar'
import { ChannelList } from './sidebar/ChannelList'
import { MainContent } from './main/MainContent'
import { ChannelHeader } from './main/ChannelHeader'
import { Channel, Server, ServerFullInfo } from '../types'
import {
  serverInviteMember,
  serversCreate,
  serversGet,
  serversList,
  channelsCreate,
  channelsList,
  channelsUpdate,
  channelInvite,
} from '../lib/api'

interface AppLayoutProps {
  username: string
  onLogout?: () => void
}

type PanelType = 'none' | 'serverConfig' | 'channelConfig' | 'devices'

export function AppLayout({ username, onLogout }: AppLayoutProps) {
  const { user } = useAuth()
  const [servers, setServers] = useState<Server[]>([])
  const [selectedServer, setSelectedServer] = useState<string | null>(null)
  const [serverDetails, setServerDetails] = useState<ServerFullInfo | null>(null)
  const [channels, setChannels] = useState<Channel[]>([])
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null)
  const [voiceJoined, setVoiceJoined] = useState(false)
  const [panel, setPanel] = useState<PanelType>('none')
  const [feedback, setFeedback] = useState<string | null>(null)

  const selectedServerInfo = selectedServer ? servers.find((server) => server.serverId === selectedServer) : undefined
  const canManageServer =
    selectedServerInfo?.myRole === 'owner' ||
    selectedServerInfo?.myRole === 'admin' ||
    serverDetails?.myRole === 'owner' ||
    serverDetails?.myRole === 'admin'
  const canManageChannel = canManageServer && !!selectedChannel

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
    fetchServers()
  }, [])

  useEffect(() => {
    if (selectedServer) {
      setSelectedChannel(null)
      setPanel('none')
      fetchServerDetails(selectedServer)
      fetchChannels(selectedServer)
    }
  }, [selectedServer])

  const handleSelectServer = (serverId: string) => {
    setSelectedServer(serverId)
  }

  const handleCreateServer = async () => {
    const name = window.prompt('Nom del servidor')?.trim()
    if (!name) {
      return
    }
    const iconUrl = window.prompt('URL de la icona (opcional)')?.trim() || null
    const result = await serversCreate(name, iconUrl)
    if (result.success) {
      await fetchServers()
      setSelectedServer(result.data.serverId)
      setFeedback(`Servidor "${result.data.name}" creat`)
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleCreateTextChannel = async () => {
    if (!selectedServer) return
    const name = window.prompt('Nom del canal de text')?.trim()
    if (!name) {
      return
    }

    const result = await channelsCreate(selectedServer, name, 'text', 'none')
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
    const usernameToInvite = window.prompt('Nom d’usuari a convidar al servidor')?.trim()
    if (!usernameToInvite) return

    const result = await serverInviteMember(selectedServer, usernameToInvite)
    if (result.success) {
      setFeedback(`Invitació enviada a ${usernameToInvite}`)
      await fetchServerDetails(selectedServer)
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleConfigureChannel = async () => {
    if (!selectedChannel || !selectedServer) return

    const name = window.prompt('Nou nom del canal', selectedChannel.name)
    if (name === null) return

    const ttlString = window.prompt('Durada TTL de missatge en segons (deixa buit per cap)', selectedChannel.messageTTL?.toString() ?? '')
    const messageTTL = ttlString ? Number(ttlString) : null

    const result = await channelsUpdate(selectedChannel.channelId, name.trim() || selectedChannel.name, messageTTL)
    if (result.success) {
      await fetchChannels(selectedServer)
      setSelectedChannel(result.data)
      setFeedback('Canal actualitzat')
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleInviteChannel = async () => {
    if (!selectedChannel) return
    const usernameToInvite = window.prompt('Nom d’usuari a convidar al canal')?.trim()
    if (!usernameToInvite) return

    const result = await channelInvite(selectedChannel.channelId, usernameToInvite)
    if (result.success) {
      setFeedback(`Invitació al canal enviada a ${usernameToInvite}`)
    } else {
      setFeedback(result.error.message)
    }
  }

  const handleManageDevices = () => {
    setPanel((current) => (current === 'devices' ? 'none' : 'devices'))
  }

  return (
    <div className="app-layout">
      <ServerBar
        servers={servers}
        selectedServer={selectedServer}
        onSelectServer={handleSelectServer}
        onCreateServer={handleCreateServer}
      />

      <ChannelList
        channels={channels}
        selectedChannel={selectedChannel}
        onSelectChannel={(channel) => setSelectedChannel(channel)}
        username={username}
        onLogout={onLogout}
        onManageDevices={handleManageDevices}
        onCreateChannel={handleCreateTextChannel}
        canCreateChannel={canManageServer}
      />

      <div className="main-content-area">
        {selectedServer && (
          <div className="server-actions">
            {canManageServer && (
              <>
                <button onClick={() => setPanel('serverConfig')}>Configurar servidor</button>
                <button onClick={handleInviteServerMember}>Convidar al servidor</button>
                <button onClick={handleCreateTextChannel}>Crear canal de text</button>
              </>
            )}
            <button onClick={handleManageDevices}>Gestió dispositius</button>
          </div>
        )}

        {feedback && <div className="feedback-banner">{feedback}</div>}

        {selectedChannel ? (
          <>
            <ChannelHeader
              channel={selectedChannel}
              canManageChannel={canManageChannel}
              onConfigureChannel={handleConfigureChannel}
              onInviteChannel={handleInviteChannel}
            />
            <MainContent
              channel={selectedChannel}
              voiceJoined={voiceJoined}
              onToggleVoice={() => setVoiceJoined(!voiceJoined)}
            />
          </>
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

        {panel === 'channelConfig' && selectedChannel && (
          <div className="panel channel-config-panel">
            <h3>Configuració del canal</h3>
            <p>{selectedChannel.name}</p>
            <button onClick={handleConfigureChannel}>Editar canal</button>
            <button onClick={handleInviteChannel}>Convidar usuari</button>
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
      </div>
    </div>
  )
}
