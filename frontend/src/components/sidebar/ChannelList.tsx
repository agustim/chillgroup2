import React, { useEffect, useRef, useState } from 'react'
import { Channel, FriendPresence, ServerMember, VoiceConnection, VoiceParticipant } from '../../types'
import { EncryptionIcon } from '../shared/EncryptionIcon'

interface ChannelListProps {
  channels: Channel[]
  selectedChannel: Channel | null
  voiceConnection: VoiceConnection | null
  voicePresenceByChannel?: Record<string, VoiceParticipant[]>
  isMuted?: boolean
  isDeafened?: boolean
  isCameraOn?: boolean
  isScreenSharing?: boolean
  onToggleMute?: () => void
  onToggleDeafen?: () => void
  onToggleCamera?: () => void
  onToggleScreenShare?: () => void
  onSelectChannel: (channel: Channel) => void
  onConfigureChannel?: (channel: Channel) => void
  username: string
  onManageDevices?: () => void
  onManageChannelKeys?: () => void
  onManageFriends?: () => void
  onChangePassword?: () => void
  onManagePermissions?: () => void
  onLogout?: () => void
  onCreateTextChannel?: () => void
  onCreateVoiceChannel?: () => void
  canCreateChannel?: boolean
  friends?: FriendPresence[]
  serverMembers?: ServerMember[]
  serverMemberPresenceById?: Record<string, boolean>
}

export function ChannelList({
  channels,
  selectedChannel,
  voiceConnection,
  voicePresenceByChannel = {},
  isMuted = true,
  isDeafened = false,
  isCameraOn = false,
  isScreenSharing = false,
  onToggleMute,
  onToggleDeafen,
  onToggleCamera,
  onToggleScreenShare,
  onSelectChannel,
  onConfigureChannel,
  username,
  onManageDevices,
  onManageChannelKeys,
  onManageFriends,
  onChangePassword,
  onManagePermissions,
  onLogout,
  onCreateTextChannel,
  onCreateVoiceChannel,
  canCreateChannel = false,
  friends = [],
  serverMembers = [],
  serverMemberPresenceById = {},
}: ChannelListProps) {
  const [isUserMenuOpen, setIsUserMenuOpen] = useState(false)
  const [collapsedSections, setCollapsedSections] = useState({
    text: false,
    voice: false,
    friends: false,
    members: false,
  })
  const userActionsRef = useRef<HTMLDivElement | null>(null)
  const voiceControlsEnabled = !!voiceConnection
  const textChannels = channels.filter((c) => c.type === 'text')
  const voiceChannels = channels.filter((c) => c.type === 'voice')

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (userActionsRef.current && !userActionsRef.current.contains(event.target as Node)) {
        setIsUserMenuOpen(false)
      }
    }

    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  // Get participants for a voice channel (from the active connection or mock)
  const getParticipants = (channel: Channel): VoiceParticipant[] => {
    if (voiceConnection && voiceConnection.channelId === channel.channelId) {
      return voiceConnection.participants
    }
    return voicePresenceByChannel[channel.channelId] ?? []
  }

  const toggleSection = (section: 'text' | 'voice' | 'friends' | 'members') => {
    setCollapsedSections((current) => ({
      ...current,
      [section]: !current[section],
    }))
  }

  return (
    <div className="channel-list">
      {/* User Info */}
      <div className="channel-list-user">
        <div className="user-avatar">{username.charAt(0).toUpperCase()}</div>
        <span className="user-name">{username}</span>
        <div className="user-actions" ref={userActionsRef}>
          <button
            className={`user-actions-toggle ${isUserMenuOpen ? 'active' : ''}`}
            onClick={() => setIsUserMenuOpen((current) => !current)}
            title="Menú d'usuari"
          >
            ⚙️
          </button>
          {isUserMenuOpen && (
            <div className="user-actions-menu">
              <button onClick={() => { setIsUserMenuOpen(false); onManageDevices?.() }}>Gestió de dispositius</button>
              <button onClick={() => { setIsUserMenuOpen(false); onManageChannelKeys?.() }}>Gestió claus-canals</button>
              <button onClick={() => { setIsUserMenuOpen(false); onManageFriends?.() }}>Gestió d'amics</button>
              <button onClick={() => { setIsUserMenuOpen(false); onChangePassword?.() }}>Canviar password</button>
              <button onClick={() => { setIsUserMenuOpen(false); onManagePermissions?.() }}>Permisos</button>
              <button onClick={() => { setIsUserMenuOpen(false); onLogout?.() }}>Sortir</button>
            </div>
          )}
        </div>
      </div>

      {/* Text Channels */}
      <div className="channel-category">
        <div className="category-header">
          <button
            className="category-toggle"
            onClick={() => toggleSection('text')}
            aria-expanded={!collapsedSections.text}
            title={collapsedSections.text ? 'Desplegar secció' : 'Plegar secció'}
          >
            <span className="category-name"># CANALS DE TEXT</span>
            <span className="category-chevron">{collapsedSections.text ? '🔻' : '🔺'}</span>
          </button>
          {canCreateChannel && onCreateTextChannel && (
            <button
              className="create-channel-btn"
              onClick={onCreateTextChannel}
              title="Crear canal de text"
            >
              +
            </button>
          )}
        </div>
        {!collapsedSections.text && textChannels.map((channel) => (
          <div
            key={channel.channelId}
            className={`channel-item ${selectedChannel?.channelId === channel.channelId ? 'active' : ''}`}
            onClick={() => onSelectChannel(channel)}
          >
            <span className="channel-hash">#</span>
            <span className="channel-name">{channel.name}</span>
            {(channel.unreadCount ?? 0) > 0 && (
              <span className="channel-unread-badge">{channel.unreadCount}</span>
            )}
            <EncryptionIcon type={channel.encryptionType} />
            {onConfigureChannel && (
              <button
                className="channel-item-settings-btn"
                onClick={(event) => {
                  event.stopPropagation()
                  onConfigureChannel(channel)
                }}
                title="Configuració del canal"
              >
                ⚙️
              </button>
            )}
          </div>
        ))}
      </div>

      {/* Voice Channels */}
      <div className="channel-category">
        <div className="category-header">
          <button
            className="category-toggle"
            onClick={() => toggleSection('voice')}
            aria-expanded={!collapsedSections.voice}
            title={collapsedSections.voice ? 'Desplegar secció' : 'Plegar secció'}
          >
            <span className="category-name">🔊 CANALS DE VEUS</span>
            <span className="category-chevron">{collapsedSections.voice ? '🔻' : '🔺'}</span>
          </button>
          {canCreateChannel && onCreateVoiceChannel && (
            <button
              className="create-channel-btn"
              onClick={onCreateVoiceChannel}
              title="Crear canal de veu"
            >
              +
            </button>
          )}
        </div>
        {!collapsedSections.voice && voiceChannels.map((channel) => {
          const participants = getParticipants(channel)

          return (
            <div key={channel.channelId} className="voice-channel-wrapper">
              <div
                className={`channel-item voice ${selectedChannel?.channelId === channel.channelId ? 'active' : ''}`}
                onClick={() => onSelectChannel(channel)}
              >
                <span className="channel-voice-icon">🔊</span>
                <span className="channel-name">{channel.name}</span>
                <EncryptionIcon type={channel.encryptionType} />
                {onConfigureChannel && (
                  <button
                    className="channel-item-settings-btn"
                    onClick={(event) => {
                      event.stopPropagation()
                      onConfigureChannel(channel)
                    }}
                    title="Configuració del canal"
                  >
                    ⚙️
                  </button>
                )}
              </div>
              
              {/* Show connected users indented below the channel */}
              {participants.length > 0 && (
                <div className="voice-channel-participants">
                  {participants.map((p) => (
                    <div key={p.userId} className="voice-participant-indicator">
                      <span className={`participant-avatar-small ${p.isSpeaking ? 'speaking' : ''}`}>
                        {p.username.charAt(0).toUpperCase()}
                      </span>
                      <span className="participant-name-small">{p.username}</span>
                      {p.isSuppressed && <span className="deafened-dot" title="Micròfon apagat">🔕</span>}
                      {p.isDeafened && <span className="deafened-dot" title="Altaveu apagat">🔇</span>}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )
        })}
      </div>

      <div className="channel-category friends-category">
        <div className="category-header">
          <button
            className="category-toggle"
            onClick={() => toggleSection('friends')}
            aria-expanded={!collapsedSections.friends}
            title={collapsedSections.friends ? 'Desplegar secció' : 'Plegar secció'}
          >
            <span className="category-name">💛 AMICS</span>
            <span className="category-chevron">{collapsedSections.friends ? '🔻' : '🔺'}</span>
          </button>
        </div>
        {!collapsedSections.friends && (friends.length > 0 ? (
          <div className="friends-list">
            {friends.map((friend) => (
              <div key={friend.userId} className="friend-item">
                <div className={`friend-avatar ${friend.isOnline ? 'online' : 'offline'}`}>
                  {friend.username.charAt(0).toUpperCase()}
                </div>
                <div className="friend-meta">
                  <span className="friend-name">{friend.username}</span>
                  <span className={`friend-status ${friend.isOnline ? 'online' : 'offline'}`}>
                    {friend.isOnline ? 'Actiu' : 'Inactiu'}
                  </span>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <p className="friends-empty-state">Encara no tens amics guardats.</p>
        ))}
      </div>

      <div className="channel-category server-members-category friends-category">
        <div className="category-header">
          <button
            className="category-toggle"
            onClick={() => toggleSection('members')}
            aria-expanded={!collapsedSections.members}
            title={collapsedSections.members ? 'Desplegar secció' : 'Plegar secció'}
          >
            <span className="category-name">👥 USUARIS DEL SERVIDOR</span>
            <span className="category-chevron">{collapsedSections.members ? '🔻' : '🔺'}</span>
          </button>
        </div>
        {!collapsedSections.members && (serverMembers.length > 0 ? (
          <div className="friends-list">
            {serverMembers.map((member) => {
              const isActive = !!serverMemberPresenceById[member.userId]
              return (
                <div key={member.userId} className="friend-item">
                  <div className={`friend-avatar ${isActive ? 'online' : 'offline'}`}>
                    {member.username.charAt(0).toUpperCase()}
                  </div>
                  <div className="friend-meta">
                    <span className="friend-name">{member.username}</span>
                    <span className={`friend-status ${isActive ? 'online' : 'offline'}`}>
                      {isActive ? 'Actiu' : 'Inactiu'}
                    </span>
                  </div>
                </div>
              )
            })}
          </div>
        ) : (
          <p className="friends-empty-state">Aquest servidor encara no té membres visibles.</p>
        ))}
      </div>

      <div className="channel-list-bottom-controls">
        <button
          className={`voice-user-btn ${isMuted ? 'active-off' : 'active-on'}`}
          onClick={onToggleMute}
          title={isMuted ? 'Activar micròfon' : 'Silenciar micròfon'}
          disabled={!voiceControlsEnabled}
        >
          🎤
        </button>
        <button
          className={`voice-user-btn ${isDeafened ? 'active-off' : 'active-on'}`}
          onClick={onToggleDeafen}
          title={isDeafened ? 'Activar so' : 'Desactivar so'}
          disabled={!voiceControlsEnabled}
        >
          🔊
        </button>
        <button
          className={`voice-user-btn ${isCameraOn ? 'active-on' : 'active-off'}`}
          onClick={onToggleCamera}
          title={isCameraOn ? 'Apagar càmera' : 'Activar càmera'}
          disabled={!voiceControlsEnabled}
        >
          🎥
        </button>
        <button
          className={`voice-user-btn ${isScreenSharing ? 'active-on' : 'active-off'}`}
          onClick={onToggleScreenShare}
          title={isScreenSharing ? 'Aturar compartir pantalla' : 'Compartir pantalla'}
          disabled={!voiceControlsEnabled}
        >
          🖥️
        </button>
      </div>
    </div>
  )
}
