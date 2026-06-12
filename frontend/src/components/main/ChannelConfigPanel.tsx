import React from 'react'
import { useTranslation } from 'react-i18next'
import { Channel, UserSearchResult } from '../../types'
import { InviteUserSearch } from '../shared/InviteUserSearch'
import { ChannelPermissionRow } from '../../hooks/useChannelConfig'
import { TTLSelector } from '../shared/TTLSelector'

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
  const { t } = useTranslation()
  return (
    <div className="panel admin-users-panel">
      <div className="admin-users-panel-header">
        <h3>{t('channelConfigPanel.title')}</h3>
        <button className="admin-panel-tab" onClick={onBackToServer}>
          {t('channelConfigPanel.backToServer')}
        </button>
      </div>

      <form onSubmit={onSave} className="modal-form" style={{ marginBottom: '12px' }}>
        <div className="form-group">
          <label htmlFor="integrated-channel-name">{t('channelForm.nameLabel')}</label>
          <input
            id="integrated-channel-name"
            type="text"
            value={channelConfigName}
            onChange={(e) => setChannelConfigName(e.target.value)}
            maxLength={30}
          />
        </div>
        <div className="form-group">
          <label>{t('channelForm.ttlLabel')}</label>
          <TTLSelector
            value={channelConfigMessageTTL ? Number(channelConfigMessageTTL) : null}
            onChange={(ttl) => setChannelConfigMessageTTL(ttl === null ? '' : String(ttl))}
          />
        </div>
        <label style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '12px' }}>
          <input
            type="checkbox"
            checked={channelConfigIsPrivate}
            onChange={(e) => setChannelConfigIsPrivate(e.target.checked)}
          />
          {t('configureChannel.privateLabel')}
        </label>
        <div className="modal-form-actions" style={{ justifyContent: 'flex-end' }}>
          <button type="submit" className="admin-panel-tab active">
            {t('configureChannel.saveChanges')}
          </button>
        </div>
      </form>

      <div className="modal-form" style={{ marginBottom: '12px' }}>
        <h4 style={{ marginBottom: '8px' }}>{t('configureChannel.inviteUser')}</h4>
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
          {t('configureChannel.deleteChannel')}
        </button>
      </div>

      {canViewChannelExplicitPermissions && (
        <div className="server-members" style={{ marginTop: '12px' }}>
          <h4>{t('channelConfigPanel.permsTitle')}</h4>
          {channelExplicitPermissionsLoading ? (
            <p>{t('channelConfigPanel.loadingPerms')}</p>
          ) : channelPermissionRows.length > 0 ? (
            <div style={{ overflowX: 'auto' }}>
              <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
                <thead>
                  <tr>
                    <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>{t('permissions.thUser')}</th>
                    <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>{t('channelConfigPanel.thEffective')}</th>
                    <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>{t('channelConfigPanel.thOrigin')}</th>
                    <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>{t('channelConfigPanel.thOverride')}</th>
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
                          {entry.explicitLevel === null ? t('channelConfigPanel.inherited') : t('channelConfigPanel.explicit')}
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
                          <option value="inherited">{t('channelConfigPanel.inheritedOption')}</option>
                          {channel.type === 'voice' ? (
                            <>
                              <option value="1">{t('channelConfigPanel.voiceOpt1')}</option>
                              <option value="2">{t('channelConfigPanel.voiceOpt2')}</option>
                              <option value="3">{t('channelConfigPanel.voiceOpt3')}</option>
                            </>
                          ) : (
                            <>
                              <option value="1">{t('channelConfigPanel.textOpt1')}</option>
                              <option value="2">{t('channelConfigPanel.textOpt2')}</option>
                              <option value="3">{t('channelConfigPanel.textOpt3')}</option>
                            </>
                          )}
                        </select>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <p>{t('channelConfigPanel.noPerms')}</p>
          )}
        </div>
      )}
    </div>
  )
}
