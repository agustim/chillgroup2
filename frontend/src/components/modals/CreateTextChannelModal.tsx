import React, { useState } from 'react'
import { Button } from '../shared/Button'

interface CreateTextChannelPanelProps {
  onClose: () => void
  onCreate: (name: string, encryptionType: string, messageTTL: number | null, isPrivate: boolean) => Promise<void>
}

export function CreateTextChannelPanel({ onClose, onCreate }: CreateTextChannelPanelProps) {
  const [name, setName] = useState('')
  const [encryptionType, setEncryptionType] = useState<'none' | 'symmetric' | 'asymmetric'>('none')
  const [messageTTL, setMessageTTL] = useState<string>('')
  const [isPrivate, setIsPrivate] = useState(false)
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
      const ttl = messageTTL ? parseInt(messageTTL, 10) : null
      const finalTTL = ttl !== null && ttl > 0 ? ttl : null
      await onCreate(trimmed.toLowerCase(), encryptionType, finalTTL, isPrivate)
      setName('')
      setMessageTTL('')
      setEncryptionType('none')
      setIsPrivate(false)
      onClose()
    } catch {
      setError('No s\'ha pogut crear el canal')
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
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
        <label htmlFor="encryption-type">Encriptació</label>
        <select
          id="encryption-type"
          value={encryptionType}
          onChange={(e) => setEncryptionType(e.target.value as 'none' | 'symmetric' | 'asymmetric')}
          disabled={isSubmitting}
        >
          <option value="none">Sense encriptació</option>
          <option value="symmetric">Simètrica (clau compartida)</option>
          <option value="asymmetric">Asimètrica (clau pública/privada)</option>
        </select>
      </div>

      <div className="form-group">
        <label htmlFor="message-ttl">TTL missatges (minuts, 0 = infinit)</label>
        <input
          id="message-ttl"
          type="number"
          value={messageTTL}
          onChange={(e) => setMessageTTL(e.target.value)}
          placeholder="0"
          min="0"
          disabled={isSubmitting}
        />
      </div>

      <div className="form-group">
        <label htmlFor="channel-private" style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <input
            id="channel-private"
            type="checkbox"
            checked={isPrivate}
            onChange={(e) => setIsPrivate(e.target.checked)}
            disabled={isSubmitting}
          />
          <span>Canal privat (secret)</span>
        </label>
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
  )
}
