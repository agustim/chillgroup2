import React from 'react'
import { Channel, ServerFullInfo, UserSearchResult } from '../../types'
import { InviteUserSearch } from '../shared/InviteUserSearch'

interface ServerConfigPanelProps {
  serverDetails: ServerFullInfo
  channels: Channel[]
  canManageServer: boolean
  currentUserId: string | undefined
  pendingMemberRemovalId: string | null
  onSetPendingMemberRemovalId: (id: string | null) => void
  onSearchUsers: (query: string) => Promise<UserSearchResult[]>
  onInviteServerSubmit: (username: string) => Promise<void>
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
  onConfigureChannel,
  onUpdateServerMemberRole,
  onRemoveServerMember,
  onOpenPermissions,
}: ServerConfigPanelProps) {
  return (
    <div className="panel admin-users-panel">
      <div className="admin-users-panel-header">
        <h3>Configuració del servidor</h3>
        <button className="admin-panel-tab" onClick={onOpenPermissions}>
          Permisos usuaris/canals
        </button>
      </div>

      <p>
        <strong>{serverDetails.name}</strong> · Rol: {serverDetails.myRole}
      </p>

      {canManageServer && (
        <div className="modal-form" style={{ marginTop: '12px', marginBottom: '12px' }}>
          <h4 style={{ marginBottom: '8px' }}>Convidar membre al servidor</h4>
          <InviteUserSearch
            onSearchUsers={onSearchUsers}
            onInvite={onInviteServerSubmit}
          />
        </div>
      )}

      <div className="server-members" style={{ marginTop: '12px' }}>
        <h4>Canals del servidor</h4>
        <ul>
          {channels.filter((channel) => channel.scope !== 'dm').map((channel) => (
            <li key={channel.channelId} style={{ display: 'flex', justifyContent: 'space-between', gap: '8px' }}>
              <span>{channel.type === 'voice' ? '🔊' : '#'} {channel.name}</span>
              <button
                type="button"
                className="admin-panel-tab"
                onClick={() => onConfigureChannel(channel)}
              >
                Configurar
              </button>
            </li>
          ))}
        </ul>
      </div>

      <div className="server-members" style={{ marginTop: '12px' }}>
        <h4>Membres</h4>
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
                        aria-label={`Rol de ${member.username}`}
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
                      Eliminar
                    </button>
                    {pendingMemberRemovalId === member.userId && (
                      <>
                        <button
                          type="button"
                          className="admin-panel-tab"
                          onClick={() => onRemoveServerMember(member.userId)}
                        >
                          Confirmar
                        </button>
                        <button
                          type="button"
                          className="admin-panel-tab"
                          onClick={() => onSetPendingMemberRemovalId(null)}
                        >
                          Cancel·lar
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
