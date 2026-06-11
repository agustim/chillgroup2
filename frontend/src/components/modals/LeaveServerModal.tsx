import React from 'react'
import { useTranslation } from 'react-i18next'
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
  const { t } = useTranslation()
  if (!confirm) return null

  return (
    <div className="modal-overlay" onClick={() => !busy && onCancel()}>
      <div className="modal-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2 className="modal-title">{t('leaveServer.title')}</h2>
        </div>
        <div className="modal-body">
          {confirm.isLastAdmin ? (
            <p style={{ color: 'var(--text-secondary)', margin: 0 }}>
              {t('leaveServer.lastAdminPre')} <strong>{confirm.serverName}</strong>{t('leaveServer.lastAdminPost')}
            </p>
          ) : (
            <p style={{ color: 'var(--text-secondary)', margin: 0 }}>
              {t('leaveServer.leavePre')} <strong>{confirm.serverName}</strong>{t('leaveServer.leavePost')}
            </p>
          )}
        </div>
        <div className="modal-form-actions">
          <Button variant="secondary" onClick={onCancel} disabled={busy}>
            {t('common.cancel')}
          </Button>
          <Button variant="danger" onClick={() => onConfirm(confirm.isLastAdmin)} disabled={busy}>
            {busy ? t('leaveServer.leaving') : t('leaveServer.leave')}
          </Button>
        </div>
      </div>
    </div>
  )
}
