import React, { useState } from 'react'

import { Button } from '../shared/Button'
import { userChangePassword } from '../../lib/api'
import { getAllChannelKeys, storeChannelKey } from '../../lib/storage'
import { hasLocalVault, rotateLocalVaultPassphrase } from '../../lib/local-vault'

interface ChangePasswordPanelProps {
  onClose: () => void
}

function ChangePasswordContent({ onClose }: ChangePasswordPanelProps) {
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
      setError('Has d\'omplir tots els camps')
      return
    }

    if (trimmedNew.length < 8) {
      setError('La nova clau ha de tenir almenys 8 caràcters')
      return
    }

    if (trimmedOld === trimmedNew) {
      setError('La nova clau ha de ser diferent de l\'antiga')
      return
    }

    if (trimmedNew !== trimmedRepeat) {
      setError('La nova clau i la repetició no coincideixen')
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
      setSuccess('Contrasenya actualitzada correctament')
    } catch {
      setError('No s\'ha pogut canviar la contrasenya')
    } finally {
      setIsSubmitting(false)
    }
  }

  const handleLocalPassphraseSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    setLocalError(null)
    setLocalSuccess(null)

    if (!hasLocalVault()) {
      setLocalError('Aquest dispositiu encara no té vault local configurat')
      return
    }

    const trimmedCurrent = currentLocalPassphrase.trim()
    const trimmedNew = newLocalPassphrase.trim()
    const trimmedRepeat = repeatNewLocalPassphrase.trim()

    if (!trimmedCurrent || !trimmedNew || !trimmedRepeat) {
      setLocalError('Has d\'omplir tots els camps de la clau local')
      return
    }

    if (trimmedNew.length < 8) {
      setLocalError('La nova clau local ha de tenir almenys 8 caràcters')
      return
    }

    if (trimmedCurrent === trimmedNew) {
      setLocalError('La nova clau local ha de ser diferent de l\'actual')
      return
    }

    if (trimmedNew !== trimmedRepeat) {
      setLocalError('La nova clau local i la repetició no coincideixen')
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
      setLocalSuccess('Clau local actualitzada i dades locals re-xifrades correctament')
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'No s\'ha pogut canviar la clau local'
      setLocalError(msg)
    } finally {
      setIsSubmittingLocal(false)
    }
  }

  return (
    <>
      <form onSubmit={handleSubmit} className="modal-form">
        <div className="form-group">
          <label htmlFor="old-password">Clau antiga</label>
          <input
            id="old-password"
            type="password"
            value={oldPassword}
            onChange={(event) => setOldPassword(event.target.value)}
            autoComplete="current-password"
            placeholder="Introdueix la clau actual"
            disabled={isSubmitting}
          />
        </div>

        <div className="form-group">
          <label htmlFor="new-password">Nova clau</label>
          <input
            id="new-password"
            type="password"
            value={newPassword}
            onChange={(event) => setNewPassword(event.target.value)}
            autoComplete="new-password"
            placeholder="Mínim 8 caràcters"
            disabled={isSubmitting}
          />
        </div>

        <div className="form-group">
          <label htmlFor="repeat-new-password">Repetir nova clau</label>
          <input
            id="repeat-new-password"
            type="password"
            value={repeatNewPassword}
            onChange={(event) => setRepeatNewPassword(event.target.value)}
            autoComplete="new-password"
            placeholder="Repeteix la nova clau"
            disabled={isSubmitting}
          />
        </div>

        {error && <div className="modal-error">{error}</div>}
        {success && <div className="modal-success">{success}</div>}

        <div className="modal-form-actions">
          <Button type="button" variant="ghost" onClick={onClose} disabled={isSubmitting}>
            Tancar
          </Button>
          <Button type="submit" variant="primary" disabled={isSubmitting}>
            {isSubmitting ? 'Actualitzant...' : 'Canviar clau'}
          </Button>
        </div>
      </form>

      <hr style={{ margin: '16px 0', border: 'none', borderTop: '1px solid var(--bg-active)' }} />

      <form onSubmit={handleLocalPassphraseSubmit} className="modal-form">
        <h4 style={{ marginBottom: '8px' }}>Canviar clau local de desbloqueig</h4>
        <p style={{ color: 'var(--text-secondary)', fontSize: '13px', marginBottom: '12px' }}>
          Aquesta clau protegeix les dades locals xifrades del navegador. No s'envia al servidor.
        </p>

        <div className="form-group">
          <label htmlFor="current-local-passphrase">Clau local actual</label>
          <input
            id="current-local-passphrase"
            type="password"
            value={currentLocalPassphrase}
            onChange={(event) => setCurrentLocalPassphrase(event.target.value)}
            autoComplete="current-password"
            placeholder="Introdueix la clau local actual"
            disabled={isSubmittingLocal}
          />
        </div>

        <div className="form-group">
          <label htmlFor="new-local-passphrase">Nova clau local</label>
          <input
            id="new-local-passphrase"
            type="password"
            value={newLocalPassphrase}
            onChange={(event) => setNewLocalPassphrase(event.target.value)}
            autoComplete="new-password"
            placeholder="Mínim 8 caràcters"
            disabled={isSubmittingLocal}
          />
        </div>

        <div className="form-group">
          <label htmlFor="repeat-new-local-passphrase">Repetir nova clau local</label>
          <input
            id="repeat-new-local-passphrase"
            type="password"
            value={repeatNewLocalPassphrase}
            onChange={(event) => setRepeatNewLocalPassphrase(event.target.value)}
            autoComplete="new-password"
            placeholder="Repeteix la nova clau local"
            disabled={isSubmittingLocal}
          />
        </div>

        {localError && <div className="modal-error">{localError}</div>}
        {localSuccess && <div className="modal-success">{localSuccess}</div>}

        <div className="modal-form-actions">
          <Button type="submit" variant="primary" disabled={isSubmittingLocal}>
            {isSubmittingLocal ? 'Re-xifrant...' : 'Canviar clau local'}
          </Button>
        </div>
      </form>
    </>
  )
}

export function ChangePasswordPanel({ onClose }: ChangePasswordPanelProps) {
  return <ChangePasswordContent onClose={onClose} />
}