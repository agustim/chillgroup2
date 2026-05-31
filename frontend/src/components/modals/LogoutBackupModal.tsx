import React, { useState } from 'react'

import { Button } from '../shared/Button'
import { exportFullBackup } from '../../lib/device-keys'
import { clearAll } from '../../lib/storage'

interface LogoutBackupModalProps {
  username: string
  onConfirm: () => void
  onCancel: () => void
}

export function LogoutBackupModal({ username, onConfirm, onCancel }: LogoutBackupModalProps) {
  const [isBusy, setIsBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const triggerDownload = (json: string) => {
    const blob = new Blob([json], { type: 'application/json' })
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

  const handleDownloadAndLogout = async () => {
    setIsBusy(true)
    setError(null)
    try {
      const json = await exportFullBackup()
      triggerDownload(json)
      await clearAll()
      onConfirm()
    } catch {
      setError('No s\'ha pogut generar el backup. Prova de tancar sessió sense backup.')
      setIsBusy(false)
    }
  }

  const handleLogoutWithoutBackup = async () => {
    setIsBusy(true)
    try {
      await clearAll()
    } finally {
      onConfirm()
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
            En tancar sessió, les claus criptogràfiques locals s'esborraran del navegador per seguretat.
          </p>
          <p style={{ color: 'var(--text-secondary)', fontSize: '13px', marginBottom: '16px' }}>
            Desa un backup per poder restaurar les claus (dispositiu i canals) quan tornis a entrar.
          </p>

          {error && <div className="modal-error" style={{ marginBottom: '12px' }}>{error}</div>}

          <div className="modal-form-actions">
            <Button variant="secondary" onClick={onCancel} disabled={isBusy}>
              Cancel·lar
            </Button>
            <Button variant="secondary" onClick={() => void handleLogoutWithoutBackup()} disabled={isBusy}>
              Sortir sense backup
            </Button>
            <Button variant="primary" onClick={() => void handleDownloadAndLogout()} disabled={isBusy}>
              {isBusy ? 'Generant...' : 'Descarregar backup i sortir'}
            </Button>
          </div>
        </div>
      </div>
    </div>
  )
}
