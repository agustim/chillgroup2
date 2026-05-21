import React from 'react'
import { Channel, VoiceConnection, VoiceParticipant } from '../../types'
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
  username: string
  onLogout?: () => void
  onManageDevices?: () => void
  onCreateTextChannel?: () => void
  onCreateVoiceChannel?: () => void
  canCreateChannel?: boolean
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
  username,
  onLogout,
  onManageDevices,
  onCreateTextChannel,
  onCreateVoiceChannel,
  canCreateChannel = false,
}: ChannelListProps) {
  const voiceControlsEnabled = !!voiceConnection
  const textChannels = channels.filter((c) => c.type === 'text')
  const voiceChannels = channels.filter((c) => c.type === 'voice')

  // Get participants for a voice channel (from the active connection or mock)
  const getParticipants = (channel: Channel): VoiceParticipant[] => {
    if (voiceConnection && voiceConnection.channelId === channel.channelId) {
      return voiceConnection.participants
    }
    return voicePresenceByChannel[channel.channelId] ?? []
  }

  const isInVoiceChannel = (channelId: string): boolean => {
    return voiceConnection !== null && voiceConnection.channelId === channelId
  }

  return (
    <div className="channel-list">
      {/* User Info */}
      <div className="channel-list-user">
        <div className="user-avatar">{username.charAt(0).toUpperCase()}</div>
        <span className="user-name">{username}</span>
        <div className="user-actions">
          {onLogout && (
            <button className="logout-btn" onClick={onLogout} title="Tancar sessió">
              🚪
            </button>
          )}
          {onManageDevices && (
            <button className="device-btn" onClick={onManageDevices} title="Gestió de dispositius">
              🖥️
            </button>
          )}
        </div>
      </div>

      {/* Text Channels */}
      <div className="channel-category">
        <div className="category-header">
          <span className="category-name"># CANALS DE TEXT</span>
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
        {textChannels.map((channel) => (
          <button
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
          </button>
        ))}
      </div>

      {/* Voice Channels */}
      <div className="channel-category">
        <div className="category-header">
          <span className="category-name">🔊 CANALS DE VEUS</span>
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
        {voiceChannels.map((channel) => {
          const participants = getParticipants(channel)
          const connected = isInVoiceChannel(channel.channelId)

          return (
            <div key={channel.channelId} className="voice-channel-wrapper">
              <button
                className={`channel-item voice ${selectedChannel?.channelId === channel.channelId ? 'active' : ''}`}
                onClick={() => onSelectChannel(channel)}
              >
                <span className="channel-voice-icon">🔊</span>
                <span className="channel-name">{channel.name}</span>
                <EncryptionIcon type={channel.encryptionType} />
              </button>
              
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
          🔁
        </button>
      </div>
    </div>
  )
}
