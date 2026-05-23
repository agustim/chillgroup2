import React from 'react'

import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'

interface ChangePasswordModalProps {
  isOpen: boolean
  onClose: () => void
}

export function ChangePasswordModal({ isOpen, onClose }: ChangePasswordModalProps) {
  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Canviar password">
      <div className="device-keys-section">
        <p>Ara mateix no hi ha un flux de canvi de contrasenya connectat al backend.</p>
        <p>Deixo aquesta entrada del menú preparada per quan l'endpoint estigui disponible.</p>
      </div>
      <div className="modal-form-actions">
        <Button type="button" variant="primary" onClick={onClose}>
          Tancar
        </Button>
      </div>
    </Modal>
  )
}