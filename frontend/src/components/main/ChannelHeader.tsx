import React from 'react'
import { Channel } from '../../types'
import { EncryptionIcon } from '../shared/EncryptionIcon'

interface ChannelHeaderProps {
  channel: Channel
  canManageChannel?: boolean
  onConfigureChannel?: () => void
  onInviteChannel?: () => void
}

export function ChannelHeader({ channel, canManageChannel = false, onConfigureChannel, onInviteChannel }: ChannelHeaderProps) {
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
        {canManageChannel && onInviteChannel && (
          <button className="channel-button" onClick={onInviteChannel} title="Convidar usuari al canal">
            ➕ Convidar
          </button>
        )}
        {canManageChannel && onConfigureChannel && (
          <button className="channel-settings-btn" onClick={onConfigureChannel} title="Configuració del canal">
            ⚙️
          </button>
        )}
      </div>
    </div>
  )
}