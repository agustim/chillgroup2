import React, { useState } from 'react'
import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'

interface CreateVoiceChannelModalProps {
  isOpen: boolean
  onClose: () => void
  onCreate: (name: string, encryptionType: string) => Promise<void>
}

export function CreateVoiceChannelModal({ isOpen, onClose, onCreate }: CreateVoiceChannelModalProps) {
  const [name, setName] = useState('')
  const [encryptionType, setEncryptionType] = useState<'none' | 'symmetric' | 'asymmetric'>('none')
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
      await onCreate(trimmed.toLowerCase(), encryptionType)
      setName('')
      setEncryptionType('none')
      onClose()
    } catch {
      setError('No s\'ha pogut crear el canal')
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Crear canal de veu">
      <form onSubmit={handleSubmit} className="modal-form">
        <div className="form-group">
          <label htmlFor="voice-channel-name">Nom del canal</label>
          <input
            id="voice-channel-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value.toLowerCase())}
            placeholder="sala de reunió"
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
          <label htmlFor="voice-encryption-type">Encriptació</label>
          <select
            id="voice-encryption-type"
            value={encryptionType}
            onChange={(e) => setEncryptionType(e.target.value as 'none' | 'symmetric' | 'asymmetric')}
            disabled={isSubmitting}
          >
            <option value="none">Sense encriptació</option>
            <option value="symmetric">Simètrica (clau compartida)</option>
            <option value="asymmetric">Asimètrica (clau pública/privada)</option>
          </select>
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
