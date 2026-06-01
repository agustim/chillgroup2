import React, { useState } from 'react'
import { Button } from './shared/Button'
import { createLocalVault, unlockLocalVault } from '../lib/local-vault'
import { migrateChannelKeysToLocalVault } from '../lib/storage'

interface DeviceUnlockScreenProps {
  mode: 'setup' | 'unlock'
  username: string
  onUnlocked: () => void
  onLogout: () => void
}

export function DeviceUnlockScreen({ mode, username, onUnlocked, onLogout }: DeviceUnlockScreenProps) {
  const [passphrase, setPassphrase] = useState('')
  const [confirmPassphrase, setConfirmPassphrase] = useState('')
  const [isBusy, setIsBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const isSetup = mode === 'setup'
  const needsConfirm = isSetup
  const mismatch = needsConfirm && confirmPassphrase.length > 0 && passphrase !== confirmPassphrase

  const handleSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    setError(null)

    if (!passphrase.trim()) {
      setError('Introdueix la clau de desbloqueig local')
      return
    }

    if (needsConfirm && passphrase !== confirmPassphrase) {
      setError('Les claus no coincideixen')
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
      setError(err instanceof Error ? err.message : 'No s\'ha pogut desbloquejar el dispositiu')
    } finally {
      setIsBusy(false)
    }
  }

  return (
    <div className="login-screen">
      <div className="login-container">
        <div className="login-header">
          <h1>{isSetup ? 'Protegeix aquest dispositiu' : 'Desbloqueja el dispositiu'}</h1>
          <p>Compte: {username}</p>
        </div>

        <div className="unlock-warning" role="note">
          <strong>Per què existeix aquest bloqueig?</strong>
          <p>
            Les claus dels canals es guarden xifrades al navegador. Aquesta clau local evita que algú amb accés al teu perfil
            pugui llegir les dades d'IndexedDB en fred.
          </p>
        </div>

        <form onSubmit={handleSubmit} className="login-form">
          <div className="form-group">
            <label htmlFor="local-unlock-passphrase">
              {isSetup ? 'Nova clau de desbloqueig local' : 'Clau de desbloqueig local'}
            </label>
            <input
              id="local-unlock-passphrase"
              type="password"
              value={passphrase}
              onChange={(e) => setPassphrase(e.target.value)}
              placeholder="Introdueix la clau local"
              autoComplete={isSetup ? 'new-password' : 'current-password'}
              disabled={isBusy}
              autoFocus
            />
          </div>

          {needsConfirm && (
            <div className="form-group">
              <label htmlFor="local-unlock-passphrase-confirm">Confirma la clau local</label>
              <input
                id="local-unlock-passphrase-confirm"
                type="password"
                value={confirmPassphrase}
                onChange={(e) => setConfirmPassphrase(e.target.value)}
                placeholder="Repeteix la clau local"
                autoComplete="new-password"
                disabled={isBusy}
              />
              {mismatch && <span className="password-hint">Les claus no coincideixen</span>}
            </div>
          )}

          {error && <div className="error-message">{error}</div>}

          <div className="form-actions" style={{ display: 'flex', gap: '8px' }}>
            <Button type="button" variant="secondary" onClick={onLogout} disabled={isBusy}>
              Tancar sessió
            </Button>
            <Button type="submit" disabled={isBusy || mismatch}>
              {isBusy ? 'Processant...' : isSetup ? 'Crear i desbloquejar' : 'Desbloquejar'}
            </Button>
          </div>
        </form>
      </div>
    </div>
  )
}
