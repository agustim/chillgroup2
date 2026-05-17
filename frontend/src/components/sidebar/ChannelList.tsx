import React from 'react'
import { Channel } from '../../types'
import { EncryptionIcon } from '../shared/EncryptionIcon'

interface ChannelListProps {
  channels: Channel[]
  selectedChannel: Channel | null
  onSelectChannel: (channel: Channel) => void
  username: string
  onLogout?: () => void
  onManageDevices?: () => void
  onCreateChannel?: () => void
  canCreateChannel?: boolean
}

export function ChannelList({
  channels,
  selectedChannel,
  onSelectChannel,
  username,
  onLogout,
  onManageDevices,
  onCreateChannel,
  canCreateChannel = false,
}: ChannelListProps) {
  const textChannels = channels.filter((c) => c.type === 'text')
  const voiceChannels = channels.filter((c) => c.type === 'voice')

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
          {canCreateChannel && onCreateChannel && (
            <button className="create-channel-btn" onClick={onCreateChannel} title="Crear canal de text">
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
        <span className="category-name">🔊 CANALS DE VEUS</span>
        {voiceChannels.map((channel) => (
          <button
            key={channel.channelId}
            className={`channel-item voice ${selectedChannel?.channelId === channel.channelId ? 'active' : ''}`}
            onClick={() => onSelectChannel(channel)}
          >
            <span className="channel-voice-icon">🔊</span>
            <span className="channel-name">{channel.name}</span>
            <EncryptionIcon type={channel.encryptionType} />
          </button>
        ))}
      </div>
    </div>
  )
}