import React, { useEffect, useState } from 'react'
import { Button } from '../shared/Button'
import { PendingServerInvitation, serverInvitationAccept, serverInvitationDecline, serverInvitationsList } from '../../lib/api'

interface ServerInvitationsModalProps {
  onClose: () => void
  onAccepted: (serverId: string) => void
}

export function ServerInvitationsModal({ onClose, onAccepted }: ServerInvitationsModalProps) {
  const [invitations, setInvitations] = useState<PendingServerInvitation[]>([])
  const [loading, setLoading] = useState(true)
  const [busyId, setBusyId] = useState<string | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    void load()
  }, [])

  async function load() {
    setLoading(true)
    const result = await serverInvitationsList()
    setLoading(false)
    if (result.success) {
      setInvitations(result.data)
    }
  }

  async function handleAccept(inv: PendingServerInvitation) {
    setBusyId(inv.invitationId)
    setError(null)
    const result = await serverInvitationAccept(inv.serverId, inv.invitationId)
    setBusyId(null)
    if (result.success) {
      setInvitations((prev) => prev.filter((i) => i.invitationId !== inv.invitationId))
      onAccepted(inv.serverId)
    } else {
      setError((result as any).error?.message ?? 'Error en acceptar la invitació')
    }
  }

  async function handleDecline(inv: PendingServerInvitation) {
    setBusyId(inv.invitationId)
    setError(null)
    const result = await serverInvitationDecline(inv.serverId, inv.invitationId)
    setBusyId(null)
    if (result.success) {
      setInvitations((prev) => prev.filter((i) => i.invitationId !== inv.invitationId))
    } else {
      setError((result as any).error?.message ?? 'Error en declinar la invitació')
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-dialog" onClick={(e) => e.stopPropagation()} style={{ maxWidth: 480 }}>
        <div className="modal-header">
          <h2 className="modal-title">Invitacions de servidor</h2>
          <button className="modal-close" onClick={onClose}>✕</button>
        </div>
        <div className="modal-body">
          {error && <div className="modal-error" style={{ marginBottom: 12 }}>{error}</div>}
          {loading ? (
            <p style={{ color: 'var(--text-secondary)', textAlign: 'center' }}>Carregant...</p>
          ) : invitations.length === 0 ? (
            <p style={{ color: 'var(--text-secondary)', textAlign: 'center' }}>
              No tens invitacions pendents.
            </p>
          ) : (
            <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: 12 }}>
              {invitations.map((inv) => (
                <li
                  key={inv.invitationId}
                  style={{
                    background: 'var(--bg-active)',
                    borderRadius: 8,
                    padding: '12px 16px',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'space-between',
                    gap: 12,
                  }}
                >
                  <div>
                    <div style={{ fontWeight: 600, color: 'var(--text-primary)' }}>{inv.serverName}</div>
                    <div style={{ fontSize: 13, color: 'var(--text-secondary)' }}>
                      Convidat per <strong>{inv.inviterUsername}</strong>
                    </div>
                  </div>
                  <div style={{ display: 'flex', gap: 8, flexShrink: 0 }}>
                    <Button
                      variant="secondary"
                      size="sm"
                      disabled={busyId === inv.invitationId}
                      onClick={() => void handleDecline(inv)}
                    >
                      Declinar
                    </Button>
                    <Button
                      variant="primary"
                      size="sm"
                      disabled={busyId === inv.invitationId}
                      onClick={() => void handleAccept(inv)}
                    >
                      Acceptar
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>
    </div>
  )
}
