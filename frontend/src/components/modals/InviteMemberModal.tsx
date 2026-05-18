import React, { useState } from 'react'
import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'

interface InviteMemberModalProps {
  isOpen: boolean
  onClose: () => void
  onInvite: (username: string) => Promise<void>
  inviteType: 'server' | 'channel'
  targetName: string
}

export function InviteMemberModal({
  isOpen,
  onClose,
  onInvite,
  inviteType,
  targetName,
}: InviteMemberModalProps) {
  const [username, setUsername] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState('')
  const [success, setSuccess] = useState('')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    const trimmed = username.trim()
    if (!trimmed) {
      setError('El nom d\'usuari és obligatori')
      return
    }
    if (trimmed.length < 3) {
      setError('El nom d\'usuari ha de tenir almenys 3 caràcters')
      return
    }
    setError('')
    setSuccess('')
    setIsSubmitting(true)
    try {
      await onInvite(trimmed)
      setSuccess(`Invitació enviada a ${trimmed}`)
      setUsername('')
      setTimeout(() => {
        setSuccess('')
        onClose()
      }, 1500)
    } catch {
      setError('No s\'ha pogut enviar la invitació')
    } finally {
      setIsSubmitting(false)
    }
  }

  const contextLabel = inviteType === 'server' ? 'servidor' : 'canal'

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={`Convidar al ${contextLabel}`}>
      <form onSubmit={handleSubmit} className="modal-form">
        <p style={{ color: 'var(--text-secondary)', fontSize: '14px', marginBottom: '8px' }}>
          Convida un usuari a <strong>{targetName}</strong>
        </p>

        <div className="form-group">
          <label htmlFor="invite-username">Nom d&apos;usuari</label>
          <input
            id="invite-username"
            type="text"
            value={username}
            onChange={(e) => setUsername(e.target.value)}
            placeholder="Nom d&apos;usuari"
            autoFocus
            autoComplete="off"
          />
        </div>

        {error && <div className="modal-error">{error}</div>}
        {success && <div className="modal-success">{success}</div>}

        <div className="modal-form-actions">
          <Button type="button" variant="ghost" onClick={onClose} disabled={isSubmitting}>
            Cancel·lar
          </Button>
          <Button type="submit" variant="primary" disabled={isSubmitting || !username.trim()}>
            {isSubmitting ? 'Enviant...' : 'Convidar'}
          </Button>
        </div>
      </form>
    </Modal>
  )
}
