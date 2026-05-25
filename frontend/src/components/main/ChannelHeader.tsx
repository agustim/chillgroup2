import React from 'react'
import { Channel } from '../../types'
import { EncryptionIcon } from '../shared/EncryptionIcon'

interface ChannelHeaderProps {
  channel: Channel
  onDmRepairKey?: () => void
  onDmRotateKey?: () => void
  dmKeyActionBusy?: boolean
}

export function ChannelHeader({ channel, onDmRepairKey, onDmRotateKey, dmKeyActionBusy = false }: ChannelHeaderProps) {
  return (
    <div className="channel-header">
      <div className="channel-header-info">
        {channel.scope === 'dm' ? (
          <span className="channel-icon">💬</span>
        ) : channel.type === 'voice' ? (
          <span className="channel-icon">🔊</span>
        ) : (
          <span className="channel-icon">#</span>
        )}
        <h2 className="channel-name">{channel.name}</h2>
        <EncryptionIcon type={channel.encryptionType} />
        {channel.scope === 'dm' && <span className="private-badge">DM</span>}
        {channel.isPrivate && <span className="private-badge">Privat</span>}
      </div>
      <div className="channel-header-actions">
        {channel.scope === 'dm' && (
          <>
            <button
              type="button"
              className="chillgroup-button chillgroup-button--ghost chillgroup-button--sm"
              onClick={onDmRepairKey}
              disabled={dmKeyActionBusy}
              title="Redistribueix la clau actual als dispositius del DM"
            >
              Reparar claus
            </button>
            <button
              type="button"
              className="chillgroup-button chillgroup-button--secondary chillgroup-button--sm"
              onClick={onDmRotateKey}
              disabled={dmKeyActionBusy}
              title="Genera una nova versió de clau per missatges futurs"
            >
              Rotar clau
            </button>
          </>
        )}
        {channel.type === 'voice' && <span className="voice-status">🎤 Connectat</span>}
      </div>
    </div>
  )
}