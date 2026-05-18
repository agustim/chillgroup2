import React, { useState } from 'react'
import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'

interface CreateChannelModalProps {
  isOpen: boolean
  onClose: () => void
  onCreate: (name: string, type: 'text' | 'voice') => Promise<void>
}

export function CreateChannelModal({ isOpen, onClose, onCreate }: CreateChannelModalProps) {
  const [name, setName] = useState('')
  const [channelType, setChannelType] = useState<'text' | 'voice'>('text')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState('')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    const trimmed = name.trim()
    if (!trimmed) {
      setError('El nom del canal és obligatori')
      return
    }
    if (!/^[a-z0-9-]+$/.test(trimmed.toLowerCase())) {
      setError('Nom vàlid: només lletres minúscules, números i guions (ex: general)')
      return
    }
    setError('')
    setIsSubmitting(true)
    try {
      await onCreate(trimmed.toLowerCase(), channelType)
      setName('')
      onClose()
    } catch {
      setError('No s\'ha pogut crear el canal')
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Crear canal">
      <form onSubmit={handleSubmit} className="modal-form">
        <div className="form-group">
          <label htmlFor="channel-name">Nom del canal</label>
          <input
            id="channel-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value.toLowerCase())}
            placeholder="general"
            autoFocus
            maxLength={30}
            pattern="[a-z0-9-]+"
            title="Només lletres minúscules, números i guions"
          />
          <span style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
            Només lletres minúscules, números i guions
          </span>
        </div>

        <div className="form-group">
          <label>Tipo de canal</label>
          <div style={{ display: 'flex', gap: '8px' }}>
            <button
              type="button"
              className={`chillgroup-button ${
                channelType === 'text'
                  ? 'chillgroup-button--primary'
                  : 'chillgroup-button--ghost'
              }`}
              onClick={() => setChannelType('text')}
              disabled={isSubmitting}
            >
              # Text
            </button>
            <button
              type="button"
              className={`chillgroup-button ${
                channelType === 'voice'
                  ? 'chillgroup-button--primary'
                  : 'chillgroup-button--ghost'
              }`}
              onClick={() => setChannelType('voice')}
              disabled={isSubmitting}
            >
              🔊 Veu
            </button>
          </div>
        </div>

        {error && <div className="modal-error">{error}</div>}

        <div className="modal-form-actions">
          <Button type="button" variant="ghost" onClick={onClose} disabled={isSubmitting}>
            Cancel·lar
          </Button>
          <Button type="submit" variant="primary" disabled={isSubmitting || !name.trim()}>
            {isSubmitting ? 'Creant...' : 'Crear'}
          </Button>
        </div>
      </form>
    </Modal>
  )
}
