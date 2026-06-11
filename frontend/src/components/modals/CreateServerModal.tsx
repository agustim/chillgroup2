import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
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
  const { t } = useTranslation()
  const [name, setName] = useState('')
  const [iconUrl, setIconUrl] = useState('')
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState('')

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    const trimmed = name.trim()
    if (!trimmed) {
      setError(t('createServer.errNameRequired'))
      return
    }
    if (trimmed.length < 2) {
      setError(t('createServer.errNameMin'))
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
      setError(t('createServer.errCreate'))
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <form onSubmit={handleSubmit} className="modal-form">
      <div className="form-group">
        <label htmlFor="server-name">{t('createServer.nameLabel')}</label>
        <input
          id="server-name"
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t('createServer.namePlaceholder')}
          autoFocus
          maxLength={50}
        />
      </div>

      <div className="form-group">
        <label htmlFor="server-icon">{t('createServer.iconLabel')}</label>
        <input
          id="server-icon"
          type="url"
          value={iconUrl}
          onChange={(e) => setIconUrl(e.target.value)}
          placeholder={t('createServer.iconPlaceholder')}
        />
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
  )
}

export function CreateServerPanel({ onClose, onCreate }: CreateServerPanelProps) {
  return <CreateServerForm onClose={onClose} onCreate={onCreate} />
}
