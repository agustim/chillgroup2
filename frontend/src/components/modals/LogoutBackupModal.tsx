import React, { useState } from 'react'

import { Button } from '../shared/Button'
import { encryptBackup, exportFullBackup } from '../../lib/device-keys'
import { clearAll } from '../../lib/storage'

interface LogoutBackupModalProps {
  username: string
  onConfirm: () => void
  onCancel: () => void
}

export function LogoutBackupModal({ username, onConfirm, onCancel }: LogoutBackupModalProps) {
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
      setError('Les contrasenyes no coincideixen')
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
      setError('No s\'ha pogut generar el backup. Prova de tancar sessió sense backup.')
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
          <h2 className="modal-title">Tancar sessió</h2>
          <button className="modal-close" onClick={onCancel} disabled={isBusy}>✕</button>
        </div>
        <div className="modal-body">
          <p style={{ marginBottom: '8px' }}>
            Pots decidir si vols fer backup i si vols esborrar les dades locals en aquesta sortida.
          </p>
          <p style={{ color: 'var(--text-secondary)', fontSize: '13px', marginBottom: '16px' }}>
            Si mantens les dades locals, continuaran xifrades i només es podran obrir amb la clau local de desbloqueig.
          </p>

          <div className="form-group" style={{ marginBottom: '16px' }}>
            <label style={{ display: 'flex', alignItems: 'center', gap: '8px', cursor: 'pointer' }}>
              <input
                type="checkbox"
                checked={clearLocalData}
                onChange={(e) => setClearLocalData(e.target.checked)}
                disabled={isBusy}
              />
              <span>Esborrar dades locals en tancar sessió</span>
            </label>
            <span className="password-hint" style={{ marginTop: '6px' }}>
              {clearLocalData
                ? 'S\'eliminaran claus i dades locals del navegador després de sortir.'
                : 'Les dades locals es conservaran xifrades per al proper inici de sessió.'}
            </span>
          </div>

          <div className="modal-form" style={{ marginBottom: '16px' }}>
            <div className="form-group">
              <label htmlFor="backup-password">Contrasenya del backup</label>
              <input
                id="backup-password"
                type="password"
                className="form-input"
                value={password}
                onChange={(e) => { setPassword(e.target.value); setError(null) }}
                placeholder="Deixa buit per desar sense xifrar"
                autoComplete="new-password"
                disabled={isBusy}
              />
            </div>
            {hasPassword && (
              <div className="form-group">
                <label htmlFor="backup-password-confirm">Confirmar contrasenya</label>
                <input
                  id="backup-password-confirm"
                  type="password"
                  className="form-input"
                  value={confirmPassword}
                  onChange={(e) => { setConfirmPassword(e.target.value); setError(null) }}
                  placeholder="Repeteix la contrasenya"
                  autoComplete="new-password"
                  disabled={isBusy}
                />
                {confirmPassword.length > 0 && !passwordsMatch && (
                  <span className="password-hint" style={{ color: 'var(--error)' }}>
                    Les contrasenyes no coincideixen
                  </span>
                )}
              </div>
            )}
          </div>

          {error && <div className="modal-error" style={{ marginBottom: '12px' }}>{error}</div>}

          <div className="modal-form-actions">
            <Button variant="secondary" onClick={onCancel} disabled={isBusy}>
              Cancel·lar
            </Button>
            <Button variant="secondary" onClick={() => void handleLogoutWithoutBackup()} disabled={isBusy}>
              Sortir sense backup
            </Button>
            <Button
              variant="primary"
              onClick={() => void handleDownloadAndLogout()}
              disabled={isBusy || (hasPassword && !passwordsMatch)}
            >
              {isBusy ? 'Processant...' : hasPassword ? 'Descarregar backup xifrat i sortir' : 'Descarregar backup i sortir'}
            </Button>
          </div>
        </div>
      </div>
    </div>
  )
}
