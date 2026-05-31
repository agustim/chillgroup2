import React, { useState } from 'react'
import { Button } from '../shared/Button'

interface CreateServerPanelProps {
  onClose: () => void
  onCreate: (name: string, iconUrl: string | null) => Promise<void>
}

interface CreateServerFormProps {
  onClose: () => void
  onCreate: (name: string, iconUrl: string | null) => Promise<void>
}

function CreateServerForm({ onClose, onCreate }: CreateServerFormProps) {
  const [name, setName] = useState('')
  const [iconUrl, setIconUrl] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState('')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    const trimmed = name.trim()
    if (!trimmed) {
      setError('El nom del servidor és obligatori')
      return
    }
    if (trimmed.length < 2) {
      setError('El nom ha de tenir almenys 2 caràcters')
      return
    }
    setError('')
    setIsSubmitting(true)
    try {
      await onCreate(trimmed, iconUrl.trim() || null)
      setName('')
      setIconUrl('')
      onClose()
    } catch {
      setError('No s\'ha pogut crear el servidor')
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <form onSubmit={handleSubmit} className="modal-form">
      <div className="form-group">
        <label htmlFor="server-name">Nom del servidor</label>
        <input
          id="server-name"
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Ex: El meu servidor"
          autoFocus
          maxLength={50}
        />
      </div>

      <div className="form-group">
        <label htmlFor="server-icon">URL de la icona (opcional)</label>
        <input
          id="server-icon"
          type="url"
          value={iconUrl}
          onChange={(e) => setIconUrl(e.target.value)}
          placeholder="https://exemple.com/icona.png"
        />
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

export function CreateServerPanel({ onClose, onCreate }: CreateServerPanelProps) {
  return <CreateServerForm onClose={onClose} onCreate={onCreate} />
}
