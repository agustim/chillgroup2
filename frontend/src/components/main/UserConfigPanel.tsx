import React, { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ChangePasswordContent } from '../modals/ChangePasswordModal'
import { PlanSettingsPanel } from '../modals/PlanSettingsModal'
import { ServerInvitationsContent } from '../modals/ServerInvitationsModal'

interface UserConfigPanelProps {
  onClose: () => void
  notificationsEnabled: boolean
  notificationPermission?: string
  onToggleNotifications: () => void
  onInvitationAccepted: (serverId: string) => void
}

type Tab = 'password' | 'plan' | 'invitations' | 'notifications'

export function UserConfigPanel({
  onClose,
  notificationsEnabled,
  notificationPermission,
  onToggleNotifications,
  onInvitationAccepted,
}: UserConfigPanelProps) {
  const { t } = useTranslation()
  const [tab, setTab] = useState<Tab>('password')

  const tabs: { id: Tab; label: string }[] = [
    { id: 'password', label: t('userConfig.tabPassword') },
    { id: 'plan', label: t('userConfig.tabPlan') },
    { id: 'invitations', label: t('userConfig.tabInvitations') },
    { id: 'notifications', label: t('userConfig.tabNotifications') },
  ]

  return (
    <div className="user-config-panel">
      <div className="user-config-tabs">
        {tabs.map(({ id, label }) => (
          <button
            key={id}
            className={`user-config-tab ${tab === id ? 'active' : ''}`}
            onClick={() => setTab(id)}
          >
            {label}
          </button>
        ))}
      </div>

      <div className="user-config-content">
        {tab === 'password' && (
          <ChangePasswordContent onClose={onClose} />
        )}

        {tab === 'plan' && (
          <PlanSettingsPanel onClose={onClose} />
        )}

        {tab === 'invitations' && (
          <ServerInvitationsContent onAccepted={onInvitationAccepted} />
        )}

        {tab === 'notifications' && (
          <div className="user-config-notifications">
            <p style={{ color: 'var(--text-secondary)', fontSize: 13, marginBottom: 16 }}>
              {t('userConfig.notificationsDesc')}
            </p>
            {notificationPermission === 'denied' ? (
              <div className="modal-error">{t('channels.notificationsBlocked')}</div>
            ) : (
              <>
                <p style={{ color: notificationsEnabled ? 'var(--text-primary)' : 'var(--text-secondary)', fontSize: 13, marginBottom: 12 }}>
                  {notificationsEnabled ? t('userConfig.notificationsStatusOn') : t('userConfig.notificationsStatusOff')}
                </p>
                <button
                  className={`voice-user-btn ${notificationsEnabled ? 'active-off' : 'active-on'}`}
                  onClick={onToggleNotifications}
                  style={{ fontSize: 14, padding: '8px 16px', width: 'auto' }}
                >
                  {notificationsEnabled ? '🔕' : '🔔'}{' '}
                  {notificationsEnabled ? t('channels.notificationsDisable') : t('channels.notificationsEnable')}
                </button>
              </>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
