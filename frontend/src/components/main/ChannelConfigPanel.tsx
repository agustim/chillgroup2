import React from 'react'
import { Channel, UserSearchResult } from '../../types'
import { InviteUserSearch } from '../shared/InviteUserSearch'
import { ChannelPermissionRow } from '../../hooks/useChannelConfig'

interface ChannelConfigPanelProps {
  channel: Channel
  channelConfigName: string
  setChannelConfigName: (v: string) => void
  channelConfigMessageTTL: string
  setChannelConfigMessageTTL: (v: string) => void
  channelConfigIsPrivate: boolean
  setChannelConfigIsPrivate: (v: boolean) => void
  onSave: (event: React.FormEvent) => void
  onSearchUsers: (query: string) => Promise<UserSearchResult[]>
  onInviteChannelSubmit: (username: string) => Promise<void>
  onDeleteChannel: (channelId: string) => void
  onBackToServer: () => void
  canViewChannelExplicitPermissions: boolean
  channelExplicitPermissionsLoading: boolean
  channelPermissionRows: ChannelPermissionRow[]
  updatingChannelPermissionUserId: string | null
  onUpdateChannelExplicitPermission: (userId: string, value: string) => void
}

export function ChannelConfigPanel({
  channel,
  channelConfigName,
  setChannelConfigName,
  channelConfigMessageTTL,
  setChannelConfigMessageTTL,
  channelConfigIsPrivate,
  setChannelConfigIsPrivate,
  onSave,
  onSearchUsers,
  onInviteChannelSubmit,
  onDeleteChannel,
  onBackToServer,
  canViewChannelExplicitPermissions,
  channelExplicitPermissionsLoading,
  channelPermissionRows,
  updatingChannelPermissionUserId,
  onUpdateChannelExplicitPermission,
}: ChannelConfigPanelProps) {
  return (
    <div className="panel admin-users-panel">
      <div className="admin-users-panel-header">
        <h3>Configuració integrada del canal</h3>
        <button className="admin-panel-tab" onClick={onBackToServer}>
          Tornar a servidor
        </button>
      </div>

      <form onSubmit={onSave} className="modal-form" style={{ marginBottom: '12px' }}>
        <div className="form-group">
          <label htmlFor="integrated-channel-name">Nom del canal</label>
          <input
            id="integrated-channel-name"
            type="text"
            value={channelConfigName}
            onChange={(e) => setChannelConfigName(e.target.value)}
            maxLength={30}
          />
        </div>
        <div className="form-group">
          <label htmlFor="integrated-channel-ttl">TTL (segons)</label>
          <input
            id="integrated-channel-ttl"
            type="number"
            value={channelConfigMessageTTL}
            onChange={(e) => setChannelConfigMessageTTL(e.target.value)}
            placeholder="Sense límit"
            min="0"
          />
        </div>
        <label style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '12px' }}>
          <input
            type="checkbox"
            checked={channelConfigIsPrivate}
            onChange={(e) => setChannelConfigIsPrivate(e.target.checked)}
          />
          Canal privat
        </label>
        <div className="modal-form-actions" style={{ justifyContent: 'flex-end' }}>
          <button type="submit" className="admin-panel-tab active">
            Desar canvis
          </button>
        </div>
      </form>

      <div className="modal-form" style={{ marginBottom: '12px' }}>
        <h4 style={{ marginBottom: '8px' }}>Convidar usuari</h4>
        <InviteUserSearch
          onSearchUsers={onSearchUsers}
          onInvite={onInviteChannelSubmit}
        />
      </div>

      <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
        <button
          type="button"
          className="admin-panel-tab"
          style={{ borderColor: '#ff6b6b', color: '#ff6b6b' }}
          onClick={() => onDeleteChannel(channel.channelId)}
        >
          Esborrar canal
        </button>
      </div>

      {canViewChannelExplicitPermissions && (
        <div className="server-members" style={{ marginTop: '12px' }}>
          <h4>Permisos del canal (efectius + override explícit)</h4>
          {channelExplicitPermissionsLoading ? (
            <p>Carregant permisos explícits...</p>
          ) : channelPermissionRows.length > 0 ? (
            <div style={{ overflowX: 'auto' }}>
              <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
                <thead>
                  <tr>
                    <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>Usuari</th>
                    <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>Permís efectiu</th>
                    <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>Origen</th>
                    <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>Override explícit</th>
                  </tr>
                </thead>
                <tbody>
                  {channelPermissionRows.map((entry) => (
                    <tr key={entry.userId}>
                      <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>{entry.username}</td>
                      <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>
                        {entry.effectivePermission} ({entry.effectiveLevel})
                      </td>
                      <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>
                        <span
                          style={{
                            display: 'inline-block',
                            padding: '2px 8px',
                            borderRadius: '999px',
                            fontSize: '11px',
                            border: '1px solid var(--bg-active)',
                            background: entry.explicitLevel === null ? 'transparent' : 'var(--bg-active)',
                          }}
                        >
                          {entry.explicitLevel === null ? 'heretat' : 'explícit'}
                        </span>
                      </td>
                      <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>
                        <select
                          value={entry.explicitLevel === null ? 'inherited' : String(entry.explicitLevel)}
                          onChange={(event) => {
                            void onUpdateChannelExplicitPermission(entry.userId, event.target.value)
                          }}
                          disabled={updatingChannelPermissionUserId === entry.userId}
                          className="device-keys-input"
                          style={{ width: '180px', padding: '4px 8px' }}
                        >
                          <option value="inherited">heretat (rol servidor)</option>
                          <option value="1">read (1)</option>
                          <option value="2">write (2)</option>
                          <option value="3">manage (3)</option>
                        </select>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <p>No hi ha dades de permisos visibles en aquest canal.</p>
          )}
        </div>
      )}
    </div>
  )
}
