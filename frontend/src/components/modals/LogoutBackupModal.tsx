import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../shared/Button'
import { encryptBackup, exportFullBackup } from '../../lib/device-keys'
import { clearAll } from '../../lib/storage'

interface LogoutBackupModalProps {
  username: string
  onConfirm: () => void
  onCancel: () => void
}

export function LogoutBackupModal({ username, onConfirm, onCancel }: LogoutBackupModalProps) {
  const { t } = useTranslation()
  const [isBusy, setIsBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [password, setPassword] = useState('')
  const [confirmPassword, setConfirmPassword] = useState('')
  const [clearLocalData, setClearLocalData] = useState(false)

  const passwordsMatch = password === confirmPassword
  const hasPassword = password.length > 0

  const triggerDownload = (content: string) => {
    const blob = new Blob([content], { type: 'application/json' })
    const url = URL.createObjectURL(blob)
    const a = document.createElement('a')
    const date = new Date().toISOString().slice(0, 10)
    a.href = url
    a.download = `chillgroup-backup-${username}-${date}.json`
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
    URL.revokeObjectURL(url)
  }

  const finalizeLogout = async () => {
    if (clearLocalData) {
      await clearAll()
    }
    onConfirm()
  }

  const handleDownloadAndLogout = async () => {
    if (hasPassword && !passwordsMatch) {
      setError(t('logoutBackup.mismatch'))
      return
    }
    setIsBusy(true)
    setError(null)
    try {
      const json = await exportFullBackup()
      const output = hasPassword ? await encryptBackup(json, password) : json
      triggerDownload(output)
      await finalizeLogout()
    } catch {
      setError(t('logoutBackup.errBackup'))
      setIsBusy(false)
    }
  }

  const handleLogoutWithoutBackup = async () => {
    setIsBusy(true)
    try {
      await finalizeLogout()
    } finally {
      // finalizeLogout ja tanca sessió
    }
  }

  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2 className="modal-title">{t('logoutBackup.title')}</h2>
          <button className="modal-close" onClick={onCancel} disabled={isBusy}>✕</button>
        </div>
        <div className="modal-body">
          <p style={{ marginBottom: '8px' }}>
            {t('logoutBackup.intro')}
          </p>
          <p style={{ color: 'var(--text-secondary)', fontSize: '13px', marginBottom: '16px' }}>
            {t('logoutBackup.introNote')}
          </p>

          <div className="form-group" style={{ marginBottom: '16px' }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={clearLocalData}
                onChange={(e) => setClearLocalData(e.target.checked)}
                disabled={isBusy}
              />
              <span>{t('logoutBackup.clearLabel')}</span>
            </label>
            <span className="password-hint" style={{ marginTop: '6px' }}>
              {clearLocalData ? t('logoutBackup.clearHintOn') : t('logoutBackup.clearHintOff')}
            </span>
          </div>

          <div className="modal-form" style={{ marginBottom: '16px' }}>
            <div className="form-group">
              <label htmlFor="backup-password">{t('logoutBackup.passwordLabel')}</label>
              <input
                id="backup-password"
                type="password"
                className="form-input"
                value={password}
                onChange={(e) => { setPassword(e.target.value); setError(null) }}
                placeholder={t('logoutBackup.passwordPlaceholder')}
                autoComplete="new-password"
                disabled={isBusy}
              />
            </div>
            {hasPassword && (
              <div className="form-group">
                <label htmlFor="backup-password-confirm">{t('logoutBackup.confirmLabel')}</label>
                <input
                  id="backup-password-confirm"
                  type="password"
                  className="form-input"
                  value={confirmPassword}
                  onChange={(e) => { setConfirmPassword(e.target.value); setError(null) }}
                  placeholder={t('logoutBackup.confirmPlaceholder')}
                  autoComplete="new-password"
                  disabled={isBusy}
                />
                {confirmPassword.length > 0 && !passwordsMatch && (
                  <span className="password-hint" style={{ color: 'var(--error)' }}>
                    {t('logoutBackup.mismatch')}
                  </span>
                )}
              </div>
            )}
          </div>

          {error && <div className="modal-error" style={{ marginBottom: '12px' }}>{error}</div>}

          <div className="modal-form-actions">
            <Button variant="secondary" onClick={onCancel} disabled={isBusy}>
              {t('common.cancel')}
            </Button>
            <Button variant="secondary" onClick={() => void handleLogoutWithoutBackup()} disabled={isBusy}>
              {t('logoutBackup.logoutNoBackup')}
            </Button>
            <Button
              variant="primary"
              onClick={() => void handleDownloadAndLogout()}
              disabled={isBusy || (hasPassword && !passwordsMatch)}
            >
              {isBusy ? t('common.processing') : hasPassword ? t('logoutBackup.downloadEncrypted') : t('logoutBackup.download')}
            </Button>
          </div>
        </div>
      </div>
    </div>
  )
}
