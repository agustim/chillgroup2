import React, { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'
import { Channel } from '../../types'

interface ConfigureChannelModalProps {
  isOpen: boolean
  onClose: () => void
  channel: Channel | null
  onUpdate: (name: string, messageTTL: number | null, isPrivate: boolean) => Promise<void>
  onDelete: (channelId: string) => Promise<void>
  onInviteChannel?: (username: string) => Promise<void>
}

export function ConfigureChannelModal({
  isOpen,
  onClose,
  channel,
  onUpdate,
  onDelete,
  onInviteChannel,
}: ConfigureChannelModalProps) {
  const { t } = useTranslation()
  const [name, setName] = useState('')
  const [messageTTL, setMessageTTL] = useState('')
  const [isPrivate, setIsPrivate] = useState(false)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [error, setError] = useState('')
  const [success, setSuccess] = useState('')
  const [deleting, setDeleting] = useState(false)
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false)
  const [inviteUsername, setInviteUsername] = useState('')
  const [inviting, setInviting] = useState(false)
  const [inviteError, setInviteError] = useState('')
  const [inviteSuccess, setInviteSuccess] = useState('')

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
    setInviteUsername('')
    setInviting(false)
    setInviteError('')
    setInviteSuccess('')
  }, [channel, isOpen])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    const trimmedName = name.trim()
    if (!trimmedName) {
      setError(t('configureChannel.errNameRequired'))
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
          setError(t('configureChannel.errTtl'))
          setIsSubmitting(false)
          return
        }
        ttl = parsed
      } else {
        ttl = null
      }
      await onUpdate(trimmedName, ttl, isPrivate)
      setSuccess(t('configureChannel.success'))
      setTimeout(() => {
        setSuccess('')
        onClose()
      }, 1500)
    } catch {
      setError(t('configureChannel.errUpdate'))
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

  const handleInvite = async (e: React.FormEvent) => {
    e.preventDefault()
    if (!onInviteChannel) {
      return
    }

    const trimmed = inviteUsername.trim()
    if (!trimmed) {
      setInviteError(t('configureChannel.inviteErrUsername'))
      return
    }

    setInviting(true)
    setInviteError('')
    setInviteSuccess('')
    try {
      await onInviteChannel(trimmed)
      setInviteSuccess(t('configureChannel.inviteSuccess', { username: trimmed }))
      setInviteUsername('')
    } catch {
      setInviteError(t('configureChannel.inviteErr'))
    } finally {
      setInviting(false)
    }
  }

  if (!channel) return null

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={t('configureChannel.title')}>
      <form onSubmit={handleSubmit} className="modal-form">
        <div className="form-group">
          <label htmlFor="config-channel-name">{t('configureChannel.nameLabel')}</label>
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
            {t('configureChannel.ttlLabel')}
            <span style={{ display: 'block', fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
              {t('configureChannel.ttlHint')}
            </span>
          </label>
          <input
            id="config-ttl"
            type="number"
            value={messageTTL}
            onChange={(e) => setMessageTTL(e.target.value)}
            placeholder={t('configureChannel.ttlPlaceholder')}
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
            <span style={{ fontSize: '14px' }}>{t('configureChannel.privateLabel')}</span>
          </label>
          <span style={{ display: 'block', fontSize: '12px', color: 'var(--text-muted)', marginTop: '4px' }}>
            {isPrivate ? t('configureChannel.privateHintOn') : t('configureChannel.privateHintOff')}
          </span>
        </div>

        <div style={{ padding: '12px', background: 'var(--bg-app)', borderRadius: '8px' }}>
          <div style={{ fontSize: '13px', color: 'var(--text-secondary)' }}>
            <div>
              <strong>{t('configureChannel.typeRow')}</strong> {channel.type === 'text' ? t('configureChannel.typeText') : t('configureChannel.typeVoice')}
            </div>
            <div>
              <strong>{t('configureChannel.encRow')}</strong> {channel.encryptionType === 'symmetric' ? t('configureChannel.encSymmetric') : channel.encryptionType === 'asymmetric' ? t('configureChannel.encAsymmetric') : t('configureChannel.encNone')}
            </div>
            <div>
              <strong>{t('configureChannel.privateRow')}</strong> {channel.isPrivate ? t('common.yes') : t('common.no')}
            </div>
          </div>
        </div>

        {onInviteChannel && (
          <section className="modal-inline-section">
            <h4>{t('configureChannel.inviteUser')}</h4>
            <form onSubmit={handleInvite} className="modal-form">
              <div className="form-group">
                <label htmlFor="channel-invite-username">{t('login.username')}</label>
                <input
                  id="channel-invite-username"
                  type="text"
                  value={inviteUsername}
                  onChange={(e) => setInviteUsername(e.target.value)}
                  placeholder={t('login.username')}
                />
              </div>
              {inviteError && <div className="modal-error">{inviteError}</div>}
              {inviteSuccess && <div className="modal-success">{inviteSuccess}</div>}
              <div className="modal-form-actions" style={{ marginTop: 0 }}>
                <Button type="submit" variant="secondary" size="sm" disabled={inviting}>
                  {inviting ? t('configureChannel.inviting') : t('configureChannel.invite')}
                </Button>
              </div>
            </form>
          </section>
        )}

        {showDeleteConfirm && (
          <div className="modal-error" style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
            <div>{t('configureChannel.deleteConfirm')}</div>
            <div style={{ display: 'flex', gap: '8px', justifyContent: 'flex-end' }}>
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={handleCancelDelete}
                disabled={deleting}
              >
                {t('common.cancel')}
              </Button>
              <Button
                type="button"
                variant="danger"
                size="sm"
                onClick={handleConfirmDelete}
                disabled={deleting}
              >
                {deleting ? t('common.erasing') : t('common.erase')}
              </Button>
            </div>
          </div>
        )}

        {error && <div className="modal-error">{error}</div>}
        {success && <div className="modal-success">{success}</div>}

        <div className="modal-form-actions">
          <Button type="button" variant="ghost" onClick={onClose} disabled={isSubmitting}>
            {t('common.cancel')}
          </Button>
          <Button type="submit" variant="primary" disabled={isSubmitting || deleting}>
            {isSubmitting ? t('common.saving') : t('configureChannel.saveChanges')}
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
            {t('configureChannel.deleteChannel')}
          </Button>
        </div>
    </Modal>
  )
}
