import { useState } from 'react'
import { Channel } from '../../types'
import { EncryptionIcon } from '../shared/EncryptionIcon'

interface ChannelHeaderProps {
  channel: Channel
  onRepairKey?: () => void
  onRotateKey?: () => void
  keyActionBusy?: boolean
  isChannelAdmin?: boolean
}

export function ChannelHeader({ channel, onRepairKey, onRotateKey, keyActionBusy = false, isChannelAdmin = false }: ChannelHeaderProps) {
  const [confirmingRotate, setConfirmingRotate] = useState(false)

  const showRepair = channel.encryptionType === 'asymmetric'
  const showRotate = channel.encryptionType === 'asymmetric' || (channel.encryptionType === 'symmetric' && isChannelAdmin)

  const handleRotateClick = () => setConfirmingRotate(true)
  const handleRotateConfirm = () => {
    setConfirmingRotate(false)
    onRotateKey?.()
  }
  const handleRotateCancel = () => setConfirmingRotate(false)

  return (
    <>
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
          {showRepair && (
            <button
              type="button"
              className="chillgroup-button chillgroup-button--ghost chillgroup-button--sm"
              onClick={onRepairKey}
              disabled={keyActionBusy || confirmingRotate}
              title="Redistribueix la clau actual als dispositius del canal"
            >
              Arreglar claus
            </button>
          )}
          {showRotate && (
            <button
              type="button"
              className="chillgroup-button chillgroup-button--secondary chillgroup-button--sm"
              onClick={handleRotateClick}
              disabled={keyActionBusy || confirmingRotate}
              title="Genera una nova versió de clau per missatges futurs"
            >
              Rotar clau
            </button>
          )}
          {channel.type === 'voice' && <span className="voice-status">🎤 Connectat</span>}
        </div>
      </div>
      {confirmingRotate && (
        <div className="feedback-banner feedback-banner--warning">
          <span>Els altres usuaris no podran llegir nous missatges fins que rebin la nova clau. Vols continuar?</span>
          <div className="feedback-banner__actions">
            <button
              type="button"
              className="chillgroup-button chillgroup-button--danger chillgroup-button--sm"
              onClick={handleRotateConfirm}
              disabled={keyActionBusy}
            >
              Rotar
            </button>
            <button
              type="button"
              className="chillgroup-button chillgroup-button--ghost chillgroup-button--sm"
              onClick={handleRotateCancel}
              disabled={keyActionBusy}
            >
              Cancel·lar
            </button>
          </div>
        </div>
      )}
    </>
  )
}
