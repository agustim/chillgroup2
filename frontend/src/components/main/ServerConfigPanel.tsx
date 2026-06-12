import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Channel, ServerFullInfo, UserSearchResult } from '../../types'
import { InviteUserSearch } from '../shared/InviteUserSearch'
import { Button } from '../shared/Button'

interface ServerConfigPanelProps {
  serverDetails: ServerFullInfo
  channels: Channel[]
  canManageServer: boolean
  currentUserId: string | undefined
  pendingMemberRemovalId: string | null
  onSetPendingMemberRemovalId: (id: string | null) => void
  onSearchUsers: (query: string) => Promise<UserSearchResult[]>
  onInviteServerSubmit: (username: string) => Promise<void>
  onAddMemberDirect?: (username: string, role: 'admin' | 'member') => Promise<void>
  onConfigureChannel: (channel: Channel) => void
  onUpdateServerMemberRole: (userId: string, role: 'admin' | 'member') => void
  onRemoveServerMember: (userId: string) => void
  onOpenPermissions: () => void
}

export function ServerConfigPanel({
  serverDetails,
  channels,
  canManageServer,
  currentUserId,
  pendingMemberRemovalId,
  onSetPendingMemberRemovalId,
  onSearchUsers,
  onInviteServerSubmit,
  onAddMemberDirect,
  onConfigureChannel,
  onUpdateServerMemberRole,
  onRemoveServerMember,
  onOpenPermissions,
}: ServerConfigPanelProps) {
  const { t } = useTranslation()
  const [addMemberUsername, setAddMemberUsername] = useState('')
  const [addMemberRole, setAddMemberRole] = useState<'member' | 'admin'>('member')
  const [addingMember, setAddingMember] = useState(false)
  return (
    <div className="panel admin-users-panel">
      <div className="admin-users-panel-header">
        <h3>{t('serverConfig.title')}</h3>
        <button className="admin-panel-tab" onClick={onOpenPermissions}>
          {t('serverConfig.permsTab')}
        </button>
      </div>

      <p>
        <strong>{serverDetails.name}</strong> · {t('serverConfig.role')} {serverDetails.myRole}
      </p>

      {canManageServer && (
        <div className="modal-form" style={{ marginTop: '12px', marginBottom: '12px' }}>
          <h4 style={{ marginBottom: '8px' }}>{t('serverConfig.inviteMember')}</h4>
          <InviteUserSearch
            onSearchUsers={onSearchUsers}
            onInvite={onInviteServerSubmit}
          />
        </div>
      )}

      {canManageServer && onAddMemberDirect && (
        <div className="modal-form" style={{ marginTop: '12px', marginBottom: '12px' }}>
          <h4 style={{ marginBottom: '8px' }}>{t('serverConfig.addMemberDirect')}</h4>
          <div className="form-group">
            <label htmlFor="add-member-username">Username</label>
            <input
              id="add-member-username"
              type="text"
              value={addMemberUsername}
              onChange={(e) => setAddMemberUsername(e.target.value)}
              placeholder="username"
              disabled={addingMember}
              style={{ marginBottom: '8px' }}
            />
          </div>
          <div className="form-group">
            <label htmlFor="add-member-role">Role</label>
            <select
              id="add-member-role"
              value={addMemberRole}
              onChange={(e) => setAddMemberRole(e.target.value as 'member' | 'admin')}
              disabled={addingMember}
              style={{ marginBottom: '8px' }}
            >
              <option value="member">member</option>
              <option value="admin">admin</option>
            </select>
          </div>
          <Button
            type="button"
            variant="primary"
            size="sm"
            disabled={!addMemberUsername || addingMember}
            onClick={async () => {
              setAddingMember(true)
              try {
                await onAddMemberDirect(addMemberUsername, addMemberRole)
                setAddMemberUsername('')
                setAddMemberRole('member')
              } finally {
                setAddingMember(false)
              }
            }}
          >
            {addingMember ? t('common.adding') : t('serverConfig.addMember')}
          </Button>
        </div>
      )}

      <div className="server-members" style={{ marginTop: '12px' }}>
        <h4>{t('serverConfig.serverChannels')}</h4>
        <ul>
          {channels.filter((channel) => channel.scope !== 'dm').map((channel) => (
            <li key={channel.channelId} style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
              <span>{channel.type === 'voice' ? '🔊' : '#'} {channel.name}</span>
              <button
                type="button"
                className="admin-panel-tab"
                onClick={() => onConfigureChannel(channel)}
              >
                {t('common.configure')}
              </button>
            </li>
          ))}
        </ul>
      </div>

      <div className="server-members" style={{ marginTop: '12px' }}>
        <h4>{t('serverConfig.members')}</h4>
        <ul>
          {serverDetails.members.map((member) => {
            const isCurrentUser = member.userId === currentUserId
            const canModify = canManageServer && member.role !== 'owner' && !isCurrentUser
            return (
              <li key={member.userId} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '8px' }}>
                <span>{member.username} — {member.role}</span>
                {canManageServer && (
                  <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                    {member.role !== 'owner' && (
                      <select
                        aria-label={t('serverConfig.roleOf', { username: member.username })}
                        value={member.role}
                        onChange={(e) => {
                          const nextRole = e.target.value as 'admin' | 'member'
                          onUpdateServerMemberRole(member.userId, nextRole)
                        }}
                        className="device-keys-input"
                        style={{ width: '120px', padding: '4px 8px' }}
                      >
                        <option value="member">member</option>
                        <option value="admin">admin</option>
                      </select>
                    )}
                    <button
                      type="button"
                      className="admin-panel-tab"
                      style={{ borderColor: '#ff6b6b', color: '#ff6b6b' }}
                      disabled={!canModify}
                      onClick={() => onSetPendingMemberRemovalId(member.userId)}
                    >
                      {t('common.remove')}
                    </button>
                    {pendingMemberRemovalId === member.userId && (
                      <>
                        <button
                          type="button"
                          className="admin-panel-tab"
                          onClick={() => onRemoveServerMember(member.userId)}
                        >
                          {t('common.confirmAction')}
                        </button>
                        <button
                          type="button"
                          className="admin-panel-tab"
                          onClick={() => onSetPendingMemberRemovalId(null)}
                        >
                          {t('common.cancel')}
                        </button>
                      </>
                    )}
                  </div>
                )}
              </li>
            )
          })}
        </ul>
      </div>
    </div>
  )
}
