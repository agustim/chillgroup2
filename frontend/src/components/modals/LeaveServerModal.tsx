import React from 'react'
import { Button } from '../shared/Button'

interface LeaveServerConfirm {
  serverId: string
  serverName: string
  isLastAdmin: boolean
}

interface LeaveServerModalProps {
  confirm: LeaveServerConfirm | null
  busy: boolean
  onConfirm: (force: boolean) => void
  onCancel: () => void
}

export function LeaveServerModal({ confirm, busy, onConfirm, onCancel }: LeaveServerModalProps) {
  if (!confirm) return null

  return (
    <div className="modal-overlay" onClick={() => !busy && onCancel()}>
      <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2 className="modal-title">Sortir del servidor</h2>
        </div>
        <div className="modal-body">
          {confirm.isLastAdmin ? (
            <p style={{ color: 'var(--text-secondary)', margin: 0 }}>
              Ets l'últim administrador de <strong>{confirm.serverName}</strong>.
              Si surts, el servidor es quedarà sense admins. Vols continuar igualment?
            </p>
          ) : (
            <p style={{ color: 'var(--text-secondary)', margin: 0 }}>
              Estàs segur que vols sortir de <strong>{confirm.serverName}</strong>?
              Hauràs de ser convidat de nou per tornar-hi.
            </p>
          )}
        </div>
        <div className="modal-form-actions">
          <Button variant="secondary" onClick={onCancel} disabled={busy}>
            Cancel·lar
          </Button>
          <Button variant="danger" onClick={() => onConfirm(confirm.isLastAdmin)} disabled={busy}>
            {busy ? 'Sortint...' : 'Sortir del servidor'}
          </Button>
        </div>
      </div>
    </div>
  )
}
