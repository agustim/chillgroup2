import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
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
  const { t } = useTranslation()
  const [name, setName] = useState('')
  const [encryptionType, setEncryptionType] = useState<'none' | 'symmetric' | 'asymmetric'>('none')
  const [messageTTL, setMessageTTL] = useState<number | null>(null)
  const [isPrivate, setIsPrivate] = useState(false)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState('')

  const idPrefix = type === 'text' ? 'channel' : 'voice-channel'
  const placeholder = type === 'text' ? 'general' : t('channelForm.voicePlaceholder')

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
      await onCreate(trimmed.toLowerCase(), encryptionType, messageTTL, isPrivate)
      setName('')
      setMessageTTL(null)
      setEncryptionType('none')
      setIsPrivate(false)
      onClose()
    } catch {
      setError(t('channelForm.errCreate'))
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <form onSubmit={handleSubmit} className="modal-form">
      <div className="form-group">
        <label htmlFor={`${idPrefix}-name`}>{t('channelForm.nameLabel')}</label>
        <input
          id={`${idPrefix}-name`}
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value.toLowerCase())}
          placeholder={placeholder}
          autoFocus
          maxLength={30}
          pattern="[a-z0-9-]+"
          title={t('channelForm.nameHint')}
        />
        <span style={{ fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
          {t('channelForm.nameHint')}
        </span>
      </div>

      <div className="form-group">
        <label htmlFor={`${idPrefix}-encryption-type`}>{t('channelForm.encryptionLabel')}</label>
        <select
          id={`${idPrefix}-encryption-type`}
          value={encryptionType}
          onChange={(e) => setEncryptionType(e.target.value as 'none' | 'symmetric' | 'asymmetric')}
          disabled={isSubmitting}
        >
          <option value="none">{t('channelForm.encNone')}</option>
          <option value="symmetric">{t('channelForm.encSymmetric')}</option>
          <option value="asymmetric">{t('channelForm.encAsymmetric')}</option>
        </select>
      </div>

      {type === 'text' && (
        <div className="form-group">
          <label>{t('channelForm.ttlLabel')}</label>
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
          <span>{t('channelForm.privateLabel')}</span>
        </label>
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
