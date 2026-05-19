import React, { useState, useEffect } from 'react'
import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'
import { Channel } from '../../types'

interface ConfigureChannelModalProps {
  isOpen: boolean
  onClose: () => void
  channel: Channel | null
  onUpdate: (name: string, messageTTL: number | null, isPrivate: boolean) => Promise<void>
  onDelete: (channelId: string) => Promise<void>
}

export function ConfigureChannelModal({
  isOpen,
  onClose,
  channel,
  onUpdate,
  onDelete,
}: ConfigureChannelModalProps) {
  const [name, setName] = useState('')
  const [messageTTL, setMessageTTL] = useState('')
  const [isPrivate, setIsPrivate] = useState(false)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState('')
  const [success, setSuccess] = useState('')
  const [deleting, setDeleting] = useState(false)
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false)

  // Quan s'obre el modal, carregar els valors actuals del canal
  useEffect(() => {
    if (channel) {
      setName(channel.name)
      setMessageTTL(channel.messageTTL?.toString() ?? '')
      setIsPrivate(channel.isPrivate)
    }
    setError('')
    setSuccess('')
    setShowDeleteConfirm(false)
    setDeleting(false)
  }, [channel, isOpen])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    const trimmedName = name.trim()
    if (!trimmedName) {
      setError('El nom del canal és obligatori')
      return
    }
    setError('')
    setIsSubmitting(true)
    try {
      const trimmed = messageTTL.trim()
      let ttl: number | null
      if (trimmed) {
        const parsed = Number(trimmed)
        if (isNaN(parsed) || parsed < 0) {
          setError('TTL ha de ser un número positiu o buit per cap límit')
          setIsSubmitting(false)
          return
        }
        ttl = parsed
      } else {
        ttl = null
      }
      await onUpdate(trimmedName, ttl, isPrivate)
      setSuccess('Canal actualitzat correctament')
      setTimeout(() => {
        setSuccess('')
        onClose()
      }, 1500)
    } catch {
      setError('No s\'ha pogut actualitzar el canal')
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleTogglePrivacy = async () => {
    if (!channel) return
    const newVal = !isPrivate
    setIsPrivate(newVal)
    try {
      await onUpdate(channel.name, channel.messageTTL, newVal)
    } catch {
      setIsPrivate(!newVal)
    }
  }

  const handleRequestDelete = () => {
    setShowDeleteConfirm(true)
  }

  const handleConfirmDelete = async () => {
    if (!channel) return
    setShowDeleteConfirm(false)
    setDeleting(true)
    try {
      await onDelete(channel.channelId)
      onClose()
    } catch {
      setDeleting(false)
    }
  }

  const handleCancelDelete = () => {
    setShowDeleteConfirm(false)
  }

  if (!channel) return null

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Configuració del canal">
      <form onSubmit={handleSubmit} className="modal-form">
        <div className="form-group">
          <label htmlFor="config-channel-name">Nom del canal</label>
          <input
            id="config-channel-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            maxLength={30}
            autoFocus
          />
        </div>

        <div className="form-group">
          <label htmlFor="config-ttl">
            Durada dels missatges (TTL en segons)
            <span style={{ display: 'block', fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
              Deixa buit per cap límit. Els missatges s'esborraran automàticament.
            </span>
          </label>
          <input
            id="config-ttl"
            type="number"
            value={messageTTL}
            onChange={(e) => setMessageTTL(e.target.value)}
            placeholder="Sense límit"
            min="0"
          />
        </div>

        <div className="form-group">
          <label
            htmlFor="config-is-private"
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: '10px',
              cursor: 'pointer',
              marginBottom: '4px',
            }}
          >
            <input
              id="config-is-private"
              type="checkbox"
              checked={isPrivate}
              onChange={handleTogglePrivacy}
              disabled={isSubmitting || deleting}
              style={{ width: '18px', height: '18px', cursor: 'pointer' }}
            />
            <span style={{ fontSize: '14px' }}>Canal privat</span>
          </label>
          <span style={{ display: 'block', fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
            {isPrivate
              ? 'Només els usuaris invitats poden accedir al canal.'
              : 'Qualsevol membre del servidor pot veure i accedir al canal.'}
          </span>
        </div>

        <div style={{ padding: '12px', background: 'var(--bg-app)', borderRadius: '8px' }}>
          <div style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>
            <div>
              <strong>Tipus:</strong> {channel.type === 'text' ? '# Text' : '🔊 Veu'}
            </div>
            <div>
              <strong>Encriptació:</strong> {channel.encryptionType === 'symmetric' ? '🔒 Simètrica' : channel.encryptionType === 'asymmetric' ? '🔐 Asimètrica' : '❌ Cap'}
            </div>
            <div>
              <strong>Privat:</strong> {channel.isPrivate ? 'Sí' : 'No'}
            </div>
          </div>
        </div>

        {showDeleteConfirm && (
          <div className="modal-error" style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <div>Estàs segur que vols esborrar aquest canal? Aquesta acció no es pot desfer.</div>
            <div style={{ display: 'flex', gap: '8px', justifyContent: 'flex-end' }}>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={handleCancelDelete}
                disabled={deleting}
              >
                Cancel·lar
              </Button>
              <Button
                type="button"
                variant="danger"
                size="sm"
                onClick={handleConfirmDelete}
                disabled={deleting}
              >
                {deleting ? 'Esborrant...' : 'Esborrar'}
              </Button>
            </div>
          </div>
        )}

        {error && <div className="modal-error">{error}</div>}
        {success && <div className="modal-success">{success}</div>}

        <div className="modal-form-actions">
          <Button type="button" variant="ghost" onClick={onClose} disabled={isSubmitting}>
            Cancel·lar
          </Button>
          <Button type="submit" variant="primary" disabled={isSubmitting || deleting}>
            {isSubmitting ? 'Desant...' : 'Desar canvis'}
          </Button>
        </div>
      </form>

        <div style={{ display: 'flex', justifyContent: 'center', padding: '0 20px 16px 20px' }}>
          <Button
            variant="danger"
            size="sm"
            onClick={handleRequestDelete}
            disabled={isSubmitting || deleting}
          >
            Esborrar canal
          </Button>
        </div>
    </Modal>
  )
}
