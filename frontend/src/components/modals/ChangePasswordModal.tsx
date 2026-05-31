import React from 'react'

import { Button } from '../shared/Button'

interface ChangePasswordPanelProps {
  onClose: () => void
}

function ChangePasswordContent({ onClose }: ChangePasswordPanelProps) {
  return (
    <>
      <div className="device-keys-section">
        <p>Ara mateix no hi ha un flux de canvi de contrasenya connectat al backend.</p>
        <p>Deixo aquesta entrada del menu preparada per quan l'endpoint estigui disponible.</p>
      </div>
      <div className="modal-form-actions">
        <Button type="button" variant="primary" onClick={onClose}>
          Tancar
        </Button>
      </div>
    </>
  )
}

export function ChangePasswordPanel({ onClose }: ChangePasswordPanelProps) {
  return <ChangePasswordContent onClose={onClose} />
}