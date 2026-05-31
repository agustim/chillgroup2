import React, { useState } from 'react'

import { Button } from '../shared/Button'
import { userChangePassword } from '../../lib/api'

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
    </>
  )
}

export function ChangePasswordPanel({ onClose }: ChangePasswordPanelProps) {
  return <ChangePasswordContent onClose={onClose} />
}