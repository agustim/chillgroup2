import React from 'react'
import { useTranslation } from 'react-i18next'
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
  const { t } = useTranslation()
  const contextLabel = inviteType === 'server' ? t('inviteMember.contextServer') : t('inviteMember.contextChannel')

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={t('inviteMember.title', { context: contextLabel })}>
      <div className="modal-form">
        <p style={{ color: 'var(--text-secondary)', fontSize: '14px', marginBottom: '8px' }}>
          {t('inviteMember.invitePrompt')} <strong>{targetName}</strong>
        </p>

        <InviteUserSearch onSearchUsers={onSearchUsers} onInvite={onInvite} />

        <div className="modal-form-actions">
          <Button type="button" variant="ghost" onClick={onClose}>
            {t('common.close')}
          </Button>
        </div>
      </div>
    </Modal>
  )
}
