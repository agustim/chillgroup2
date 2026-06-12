import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'

interface CreateChannelModalProps {
  isOpen: boolean
  onClose: () => void
  onCreate: (name: string, type: 'text' | 'voice') => Promise<void>
}

export function CreateChannelModal({ isOpen, onClose, onCreate }: CreateChannelModalProps) {
  const { t } = useTranslation()
  const [name, setName] = useState('')
  const [channelType, setChannelType] = useState<'text' | 'voice'>('text')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState('')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    const trimmed = name.trim()
    if (!trimmed) {
      setError(t('channelForm.errNameRequired'))
      return
    }
    if (!/^[a-z0-9-]+$/.test(trimmed.toLowerCase())) {
      setError(t('channelForm.errNamePattern'))
      return
    }
    setError('')
    setIsSubmitting(true)
    try {
      await onCreate(trimmed.toLowerCase(), channelType)
      setName('')
      onClose()
    } catch {
      setError(t('channelForm.errCreate'))
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={t('channelForm.title')}>
      <form onSubmit={handleSubmit} className="modal-form">
        <div className="form-group">
          <label htmlFor="channel-name">{t('channelForm.nameLabel')}</label>
          <input
            id="channel-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value.toLowerCase())}
            placeholder="general"
            autoFocus
            maxLength={30}
            pattern="[a-z0-9_-]+"
            title={t('channelForm.nameHint')}
          />
          <span style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
            {t('channelForm.nameHint')}
          </span>
        </div>

        <div className="form-group">
          <label>{t('channelForm.typeLabel')}</label>
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
              {t('channelForm.typeText')}
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
              {t('channelForm.typeVoice')}
            </button>
          </div>
        </div>

        {error && <div className="modal-error">{error}</div>}

        <div className="modal-form-actions">
          <Button type="button" variant="ghost" onClick={onClose} disabled={isSubmitting}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" variant="primary" disabled={isSubmitting || !name.trim()}>
            {isSubmitting ? t('common.creating') : t('common.create')}
          </Button>
        </div>
      </form>
    </Modal>
  )
}
