import { useState } from 'react'
import { Channel } from '../../types'
import { EncryptionIcon } from '../shared/EncryptionIcon'
import { TTLSelector, formatTTL } from '../shared/TTLSelector'

interface ChannelHeaderProps {
  channel: Channel
  onRepairKey?: () => void
  onRotateKey?: () => void
  onUpdateTTL?: (ttl: number | null) => Promise<void>
  keyActionBusy?: boolean
  isChannelAdmin?: boolean
}

export function ChannelHeader({ channel, onRepairKey, onRotateKey, onUpdateTTL, keyActionBusy = false, isChannelAdmin = false }: ChannelHeaderProps) {
  const [confirmingRotate, setConfirmingRotate] = useState(false)
  const [editingTTL, setEditingTTL] = useState(false)
  const [ttlBusy, setTtlBusy] = useState(false)

  const showRepair = channel.encryptionType === 'asymmetric'
  const showRotate = channel.encryptionType === 'asymmetric' || (channel.encryptionType === 'symmetric' && isChannelAdmin)
  const isDM = channel.scope === 'dm'

  const handleRotateClick = () => setConfirmingRotate(true)
  const handleRotateConfirm = () => {
    setConfirmingRotate(false)
    onRotateKey?.()
  }
  const handleRotateCancel = () => setConfirmingRotate(false)

  const handleTTLChange = async (value: number | null) => {
    if (!onUpdateTTL) return
    setTtlBusy(true)
    try {
      await onUpdateTTL(value)
    } finally {
      setTtlBusy(false)
      setEditingTTL(false)
    }
  }

  return (
    <>
      <div className="channel-header">
        <div className="channel-header-info">
          {isDM ? (
            <span className="channel-icon">💬</span>
          ) : channel.type === 'voice' ? (
            <span className="channel-icon">🔊</span>
          ) : (
            <span className="channel-icon">#</span>
          )}
          <h2 className="channel-name">{channel.name}</h2>
          <EncryptionIcon type={channel.encryptionType} />
          {isDM && <span className="private-badge">DM</span>}
          {channel.isPrivate && <span className="private-badge">Privat</span>}
          {channel.messageTTL != null && (
            <span
              className="ttl-badge"
              title={`TTL missatges: ${formatTTL(channel.messageTTL)}`}
            >
              ⏱ {formatTTL(channel.messageTTL)}
            </span>
          )}
        </div>
        <div className="channel-header-actions">
          {isDM && onUpdateTTL && (
            <button
              type="button"
              className="chillgroup-button chillgroup-button--ghost chillgroup-button--sm"
              onClick={() => setEditingTTL((v) => !v)}
              disabled={ttlBusy || confirmingRotate}
              title="Configura el temps d'expiració dels missatges"
            >
              TTL
            </button>
          )}
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
      {editingTTL && (
        <div className="feedback-banner">
          <span>Expiració de missatges:</span>
          <TTLSelector
            value={channel.messageTTL}
            onChange={(ttl) => void handleTTLChange(ttl)}
            disabled={ttlBusy}
          />
          <div className="feedback-banner__actions">
            <button
              type="button"
              className="chillgroup-button chillgroup-button--ghost chillgroup-button--sm"
              onClick={() => setEditingTTL(false)}
              disabled={ttlBusy}
            >
              Cancel·lar
            </button>
          </div>
        </div>
      )}
    </>
  )
}
