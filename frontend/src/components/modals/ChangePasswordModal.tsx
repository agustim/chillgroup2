import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../shared/Button'
import { userChangePassword } from '../../lib/api'
import { getAllChannelKeys, storeChannelKey } from '../../lib/storage'
import { hasLocalVault, rotateLocalVaultPassphrase } from '../../lib/local-vault'

interface ChangePasswordPanelProps {
  onClose: () => void
}

function ChangePasswordContent({ onClose }: ChangePasswordPanelProps) {
  const { t } = useTranslation()
  const [oldPassword, setOldPassword] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [repeatNewPassword, setRepeatNewPassword] = useState('')
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [currentLocalPassphrase, setCurrentLocalPassphrase] = useState('')
  const [newLocalPassphrase, setNewLocalPassphrase] = useState('')
  const [repeatNewLocalPassphrase, setRepeatNewLocalPassphrase] = useState('')
  const [localError, setLocalError] = useState<string | null>(null)
  const [localSuccess, setLocalSuccess] = useState<string | null>(null)
  const [isSubmittingLocal, setIsSubmittingLocal] = useState(false)

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    setError(null)
    setSuccess(null)

    const trimmedOld = oldPassword.trim()
    const trimmedNew = newPassword.trim()
    const trimmedRepeat = repeatNewPassword.trim()

    if (!trimmedOld || !trimmedNew || !trimmedRepeat) {
      setError(t('changePassword.errAllFields'))
      return
    }

    if (trimmedNew.length < 8) {
      setError(t('changePassword.errNewMin'))
      return
    }

    if (trimmedOld === trimmedNew) {
      setError(t('changePassword.errSameAsOld'))
      return
    }

    if (trimmedNew !== trimmedRepeat) {
      setError(t('changePassword.errMismatch'))
      return
    }

    setIsSubmitting(true)

    try {
      const result = await userChangePassword(trimmedOld, trimmedNew)
      if (!result.success) {
        setError(result.error.message)
        return
      }

      setOldPassword('')
      setNewPassword('')
      setRepeatNewPassword('')
      setSuccess(t('changePassword.success'))
    } catch {
      setError(t('changePassword.errChange'))
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleLocalPassphraseSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    setLocalError(null)
    setLocalSuccess(null)

    if (!hasLocalVault()) {
      setLocalError(t('changePassword.errNoVault'))
      return
    }

    const trimmedCurrent = currentLocalPassphrase.trim()
    const trimmedNew = newLocalPassphrase.trim()
    const trimmedRepeat = repeatNewLocalPassphrase.trim()

    if (!trimmedCurrent || !trimmedNew || !trimmedRepeat) {
      setLocalError(t('changePassword.errLocalAllFields'))
      return
    }

    if (trimmedNew.length < 8) {
      setLocalError(t('changePassword.errLocalMin'))
      return
    }

    if (trimmedCurrent === trimmedNew) {
      setLocalError(t('changePassword.errLocalSame'))
      return
    }

    if (trimmedNew !== trimmedRepeat) {
      setLocalError(t('changePassword.errLocalMismatch'))
      return
    }

    setIsSubmittingLocal(true)

    try {
      const currentKeys = await getAllChannelKeys()
      await rotateLocalVaultPassphrase(trimmedCurrent, trimmedNew)

      for (const key of currentKeys) {
        await storeChannelKey(
          key.channelId,
          key.keyBytes,
          key.type,
          key.keyVersion,
          key.keyVersionId ?? null
        )
      }

      setCurrentLocalPassphrase('')
      setNewLocalPassphrase('')
      setRepeatNewLocalPassphrase('')
      setLocalSuccess(t('changePassword.localSuccess'))
    } catch (err) {
      const msg = err instanceof Error ? err.message : t('changePassword.errLocalChange')
      setLocalError(msg)
    } finally {
      setIsSubmittingLocal(false)
    }
  }

  return (
    <>
      <form onSubmit={handleSubmit} className="modal-form">
        <div className="form-group">
          <label htmlFor="old-password">{t('changePassword.oldLabel')}</label>
          <input
            id="old-password"
            type="password"
            value={oldPassword}
            onChange={(event) => setOldPassword(event.target.value)}
            autoComplete="current-password"
            placeholder={t('changePassword.oldPlaceholder')}
            disabled={isSubmitting}
          />
        </div>

        <div className="form-group">
          <label htmlFor="new-password">{t('changePassword.newLabel')}</label>
          <input
            id="new-password"
            type="password"
            value={newPassword}
            onChange={(event) => setNewPassword(event.target.value)}
            autoComplete="new-password"
            placeholder={t('changePassword.minPlaceholder')}
            disabled={isSubmitting}
          />
        </div>

        <div className="form-group">
          <label htmlFor="repeat-new-password">{t('changePassword.newRepeatLabel')}</label>
          <input
            id="repeat-new-password"
            type="password"
            value={repeatNewPassword}
            onChange={(event) => setRepeatNewPassword(event.target.value)}
            autoComplete="new-password"
            placeholder={t('changePassword.repeatPlaceholder')}
            disabled={isSubmitting}
          />
        </div>

        {error && <div className="modal-error">{error}</div>}
        {success && <div className="modal-success">{success}</div>}

        <div className="modal-form-actions">
          <Button type="button" variant="ghost" onClick={onClose} disabled={isSubmitting}>
            {t('common.close')}
          </Button>
          <Button type="submit" variant="primary" disabled={isSubmitting}>
            {isSubmitting ? t('changePassword.submitting') : t('changePassword.submit')}
          </Button>
        </div>
      </form>

      <hr style={{ margin: '16px 0', border: 'none', borderTop: '1px solid var(--bg-active)' }} />

      <form onSubmit={handleLocalPassphraseSubmit} className="modal-form">
        <h4 style={{ marginBottom: '8px' }}>{t('changePassword.localTitle')}</h4>
        <p style={{ color: 'var(--text-secondary)', fontSize: '13px', marginBottom: '12px' }}>
          {t('changePassword.localDesc')}
        </p>

        <div className="form-group">
          <label htmlFor="current-local-passphrase">{t('changePassword.localCurrentLabel')}</label>
          <input
            id="current-local-passphrase"
            type="password"
            value={currentLocalPassphrase}
            onChange={(event) => setCurrentLocalPassphrase(event.target.value)}
            autoComplete="current-password"
            placeholder={t('changePassword.localCurrentPlaceholder')}
            disabled={isSubmittingLocal}
          />
        </div>

        <div className="form-group">
          <label htmlFor="new-local-passphrase">{t('changePassword.localNewLabel')}</label>
          <input
            id="new-local-passphrase"
            type="password"
            value={newLocalPassphrase}
            onChange={(event) => setNewLocalPassphrase(event.target.value)}
            autoComplete="new-password"
            placeholder={t('changePassword.minPlaceholder')}
            disabled={isSubmittingLocal}
          />
        </div>

        <div className="form-group">
          <label htmlFor="repeat-new-local-passphrase">{t('changePassword.localRepeatLabel')}</label>
          <input
            id="repeat-new-local-passphrase"
            type="password"
            value={repeatNewLocalPassphrase}
            onChange={(event) => setRepeatNewLocalPassphrase(event.target.value)}
            autoComplete="new-password"
            placeholder={t('changePassword.localRepeatPlaceholder')}
            disabled={isSubmittingLocal}
          />
        </div>

        {localError && <div className="modal-error">{localError}</div>}
        {localSuccess && <div className="modal-success">{localSuccess}</div>}

        <div className="modal-form-actions">
          <Button type="submit" variant="primary" disabled={isSubmittingLocal}>
            {isSubmittingLocal ? t('changePassword.localSubmitting') : t('changePassword.localSubmit')}
          </Button>
        </div>
      </form>
    </>
  )
}

export function ChangePasswordPanel({ onClose }: ChangePasswordPanelProps) {
  return <ChangePasswordContent onClose={onClose} />
}