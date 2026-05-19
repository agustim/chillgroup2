import React from 'react'
import { Channel, VoiceConnection, VoiceParticipant } from '../../types'
import { EncryptionIcon } from '../shared/EncryptionIcon'

interface ChannelListProps {
  channels: Channel[]
  selectedChannel: Channel | null
  voiceConnection: VoiceConnection | null
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
  onSelectChannel,
  username,
  onLogout,
  onManageDevices,
  onCreateTextChannel,
  onCreateVoiceChannel,
  canCreateChannel = false,
}: ChannelListProps) {
  const textChannels = channels.filter((c) => c.type === 'text')
  const voiceChannels = channels.filter((c) => c.type === 'voice')

  // Get participants for a voice channel (from the active connection or mock)
  const getParticipants = (channel: Channel): VoiceParticipant[] => {
    if (voiceConnection && voiceConnection.channelId === channel.channelId) {
      return voiceConnection.participants
    }
    // Mock participants for voice channels you're not in (shows # connected)
    return []
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
                      <span className="participant-avatar-small">
                        {p.username.charAt(0).toUpperCase()}
                      </span>
                      <span className="participant-name-small">{p.username}</span>
                      {p.isSpeaking && <span className="speaking-dot">🗣️</span>}
                      {p.isDeafened && <span className="deafened-dot">🔕</span>}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )
        })}
      </div>

      {/* Active voice connection indicator */}
      {voiceConnection && (
        <div className="voice-connection-indicator">
          <span className="voice-indicator-icon">🔊</span>
          <span className="voice-indicator-text">Unit a: {voiceConnection.channelName}</span>
          <button 
            className="voice-indicator-leave"
            onClick={() => onSelectChannel({
              ...voiceConnection,
              type: 'voice',
              channelId: voiceConnection.channelId,
              name: voiceConnection.channelName,
              encryptionType: 'none',
              messageTTL: null,
              isPrivate: false,
              createdAt: '',
            })}
            title="Surt del canal de veu"
          >
            Surt
          </button>
        </div>
      )}
    </div>
  )
}
