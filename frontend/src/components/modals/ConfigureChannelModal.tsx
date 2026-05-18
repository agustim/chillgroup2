import React, { useState, useEffect } from 'react'
import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'
import { Channel } from '../../types'

interface ConfigureChannelModalProps {
  isOpen: boolean
  onClose: () => void
  channel: Channel | null
  onSave: (name: string, messageTTL: number | null) => Promise<void>
}

export function ConfigureChannelModal({
  isOpen,
  onClose,
  channel,
  onSave,
}: ConfigureChannelModalProps) {
  const [name, setName] = useState('')
  const [messageTTL, setMessageTTL] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState('')
  const [success, setSuccess] = useState('')

  // Quan s'obre el modal, carregar els valors actuals del canal
  useEffect(() => {
    if (channel) {
      setName(channel.name)
      setMessageTTL(channel.messageTTL?.toString() ?? '')
    }
    setError('')
    setSuccess('')
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
      if (trimmed) {
        const ttl = Number(trimmed)
        if (isNaN(ttl) || ttl < 0) {
          setError('TTL ha de ser un número positiu o buit per cap límit')
          setIsSubmitting(false)
          return
        }
        await onSave(trimmedName, ttl)
      } else {
        await onSave(trimmedName, null)
      }
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

        {error && <div className="modal-error">{error}</div>}
        {success && <div className="modal-success">{success}</div>}

        <div className="modal-form-actions">
          <Button type="button" variant="ghost" onClick={onClose} disabled={isSubmitting}>
            Cancel·lar
          </Button>
          <Button type="submit" variant="primary" disabled={isSubmitting}>
            {isSubmitting ? 'Desant...' : 'Desar canvis'}
          </Button>
        </div>
      </form>
    </Modal>
  )
}
