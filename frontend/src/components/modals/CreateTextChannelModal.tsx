import React, { useState } from 'react'
import { Button } from '../shared/Button'
import { TTLSelector } from '../shared/TTLSelector'

interface TextChannelPanelProps {
  onClose: () => void
  onCreate: (name: string, encryptionType: string, messageTTL: number | null, isPrivate: boolean) => Promise<void>
}

interface VoiceChannelPanelProps {
  onClose: () => void
  onCreate: (name: string, encryptionType: string, isPrivate: boolean) => Promise<void>
}

interface CreateChannelFormProps {
  type: 'text' | 'voice'
  onClose: () => void
  onCreate: (name: string, encryptionType: string, messageTTL: number | null, isPrivate: boolean) => Promise<void>
}

function CreateChannelForm({ type, onClose, onCreate }: CreateChannelFormProps) {
  const [name, setName] = useState('')
  const [encryptionType, setEncryptionType] = useState<'none' | 'symmetric' | 'asymmetric'>('none')
  const [messageTTL, setMessageTTL] = useState<number | null>(null)
  const [isPrivate, setIsPrivate] = useState(false)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState('')

  const idPrefix = type === 'text' ? 'channel' : 'voice-channel'
  const placeholder = type === 'text' ? 'general' : 'sala de reunió'

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
      await onCreate(trimmed.toLowerCase(), encryptionType, messageTTL, isPrivate)
      setName('')
      setMessageTTL(null)
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
        <label htmlFor={`${idPrefix}-name`}>Nom del canal</label>
        <input
          id={`${idPrefix}-name`}
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value.toLowerCase())}
          placeholder={placeholder}
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
        <label htmlFor={`${idPrefix}-encryption-type`}>Encriptació</label>
        <select
          id={`${idPrefix}-encryption-type`}
          value={encryptionType}
          onChange={(e) => setEncryptionType(e.target.value as 'none' | 'symmetric' | 'asymmetric')}
          disabled={isSubmitting}
        >
          <option value="none">Sense encriptació</option>
          <option value="symmetric">Simètrica (clau compartida)</option>
          <option value="asymmetric">Asimètrica (clau pública/privada)</option>
        </select>
      </div>

      {type === 'text' && (
        <div className="form-group">
          <label>TTL missatges</label>
          <TTLSelector
            value={messageTTL}
            onChange={setMessageTTL}
            disabled={isSubmitting}
          />
        </div>
      )}

      <div className="form-group">
        <label htmlFor={`${idPrefix}-private`} style={{ display: 'flex', alignItems: 'center', gap: '10px' }}>
          <input
            id={`${idPrefix}-private`}
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

export function CreateTextChannelPanel({ onClose, onCreate }: TextChannelPanelProps) {
  return <CreateChannelForm type="text" onClose={onClose} onCreate={onCreate} />
}

export function CreateVoiceChannelPanel({ onClose, onCreate }: VoiceChannelPanelProps) {
  return (
    <CreateChannelForm
      type="voice"
      onClose={onClose}
      onCreate={(name, encType, _ttl, isPrivate) => onCreate(name, encType, isPrivate)}
    />
  )
}
