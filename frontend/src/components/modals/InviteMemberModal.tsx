import React from 'react'
import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'
import { InviteUserSearch } from '../shared/InviteUserSearch'
import type { UserSearchResult } from '../../types'

interface InviteMemberModalProps {
  isOpen: boolean
  onClose: () => void
  onInvite: (username: string) => Promise<void>
  onSearchUsers: (query: string) => Promise<UserSearchResult[]>
  inviteType: 'server' | 'channel'
  targetName: string
}

export function InviteMemberModal({
  isOpen,
  onClose,
  onInvite,
  onSearchUsers,
  inviteType,
  targetName,
}: InviteMemberModalProps) {
  const contextLabel = inviteType === 'server' ? 'servidor' : 'canal'

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={`Convidar al ${contextLabel}`}>
      <div className="modal-form">
        <p style={{ color: 'var(--text-secondary)', fontSize: '14px', marginBottom: '8px' }}>
          Convida un usuari a <strong>{targetName}</strong>
        </p>

        <InviteUserSearch onSearchUsers={onSearchUsers} onInvite={onInvite} />

        <div className="modal-form-actions">
          <Button type="button" variant="ghost" onClick={onClose}>
            Tancar
          </Button>
        </div>
      </div>
    </Modal>
  )
}
