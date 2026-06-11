import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Button } from './shared/Button'
import { LanguageSwitcher } from './shared/LanguageSwitcher'
import { createLocalVault, unlockLocalVault } from '../lib/local-vault'
import { clearAll, migrateChannelKeysToLocalVault } from '../lib/storage'

interface DeviceUnlockScreenProps {
  mode: 'setup' | 'unlock'
  username: string
  onUnlocked: () => void
  onLogout: () => void
  onReset?: () => void
}

export function DeviceUnlockScreen({ mode, username, onUnlocked, onLogout, onReset }: DeviceUnlockScreenProps) {
  const { t } = useTranslation()
  const [passphrase, setPassphrase] = useState('')
  const [confirmPassphrase, setConfirmPassphrase] = useState('')
  const [isBusy, setIsBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [showResetConfirm, setShowResetConfirm] = useState(false)

  const isSetup = mode === 'setup'
  const needsConfirm = isSetup
  const mismatch = needsConfirm && confirmPassphrase.length > 0 && passphrase !== confirmPassphrase

  const handleReset = async () => {
    setIsBusy(true)
    try {
      await clearAll()
      onReset?.()
    } catch (err) {
      setError(err instanceof Error ? err.message : t('unlock.errReset'))
      setShowResetConfirm(false)
    } finally {
      setIsBusy(false)
    }
  }

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    setError(null)

    if (!passphrase.trim()) {
      setError(t('unlock.errPassphraseRequired'))
      return
    }

    if (needsConfirm && passphrase !== confirmPassphrase) {
      setError(t('unlock.errMismatch'))
      return
    }

    setIsBusy(true)
    try {
      if (isSetup) {
        await createLocalVault(passphrase)
      } else {
        await unlockLocalVault(passphrase)
      }
      await migrateChannelKeysToLocalVault()
      onUnlocked()
    } catch (err) {
      setError(err instanceof Error ? err.message : t('unlock.errUnlock'))
    } finally {
      setIsBusy(false)
    }
  }

  return (
    <div className="login-screen">
      <div className="login-container">
        <div className="login-header">
          <h1>{isSetup ? t('unlock.titleSetup') : t('unlock.titleUnlock')}</h1>
          <p>{t('unlock.account', { username })}</p>
        </div>

        <div className="unlock-warning" role="note">
          <strong>{t('unlock.whyTitle')}</strong>
          <p>{t('unlock.whyBody')}</p>
        </div>

        <form onSubmit={handleSubmit} className="login-form">
          <div className="form-group">
            <label htmlFor="local-unlock-passphrase">
              {isSetup ? t('unlock.passphraseLabelSetup') : t('unlock.passphraseLabel')}
            </label>
            <input
              id="local-unlock-passphrase"
              type="password"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
              placeholder={t('unlock.passphrasePlaceholder')}
              autoComplete={isSetup ? 'new-password' : 'current-password'}
              disabled={isBusy}
              autoFocus
            />
          </div>

          {needsConfirm && (
            <div className="form-group">
              <label htmlFor="local-unlock-passphrase-confirm">{t('unlock.confirmLabel')}</label>
              <input
                id="local-unlock-passphrase-confirm"
                type="password"
                value={confirmPassphrase}
                onChange={(e) => setConfirmPassphrase(e.target.value)}
                placeholder={t('unlock.confirmPlaceholder')}
                autoComplete="new-password"
                disabled={isBusy}
              />
              {mismatch && <span className="password-hint">{t('unlock.errMismatch')}</span>}
            </div>
          )}

          {error && <div className="error-message">{error}</div>}

          <div className="form-actions" style={{ display: 'flex', gap: '8px' }}>
            <Button type="button" variant="secondary" onClick={onLogout} disabled={isBusy}>
              {t('common.logout')}
            </Button>
            <Button type="submit" disabled={isBusy || mismatch}>
              {isBusy ? t('common.processing') : isSetup ? t('unlock.submitSetup') : t('unlock.submitUnlock')}
            </Button>
          </div>
        </form>

        {!isSetup && onReset && (
          <div className="reset-device-section">
            {!showResetConfirm ? (
              <button
                type="button"
                className="reset-device-link"
                onClick={() => setShowResetConfirm(true)}
                disabled={isBusy}
              >
                {t('unlock.resetLink')}
              </button>
            ) : (
              <div className="reset-device-confirm">
                <p className="reset-device-warning">
                  <strong>{t('unlock.resetWarningStrong')}</strong> {t('unlock.resetWarningBody')}
                </p>
                <div className="form-actions" style={{ display: 'flex', gap: '8px' }}>
                  <Button
                    type="button"
                    variant="secondary"
                    onClick={() => setShowResetConfirm(false)}
                    disabled={isBusy}
                  >
                    {t('common.cancel')}
                  </Button>
                  <Button
                    type="button"
                    variant="danger"
                    onClick={handleReset}
                    disabled={isBusy}
                  >
                    {isBusy ? t('unlock.resetting') : t('unlock.resetConfirm')}
                  </Button>
                </div>
              </div>
            )}
          </div>
        )}

        <div className="login-language">
          <LanguageSwitcher />
        </div>
      </div>
    </div>
  )
}
