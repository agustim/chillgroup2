import React from 'react'
import { Channel } from '../../types'
import { EncryptionIcon } from '../shared/EncryptionIcon'

interface ChannelHeaderProps {
  channel: Channel
}

export function ChannelHeader({ channel }: ChannelHeaderProps) {
  return (
    <div className="channel-header">
      <div className="channel-header-info">
        {channel.type === 'voice' ? (
          <span className="channel-icon">🔊</span>
        ) : (
          <span className="channel-icon">#</span>
        )}
        <h2 className="channel-name">{channel.name}</h2>
        <EncryptionIcon type={channel.encryptionType} />
        {channel.isPrivate && <span className="private-badge">Privat</span>}
      </div>
      <div className="channel-header-actions">
        {channel.type === 'voice' && (
          <span className="voice-status">🎤 Connectat</span>
        )}
        <button className="channel-settings-btn" title="Configuració">
          ⚙️
        </button>
      </div>
    </div>
  )
}