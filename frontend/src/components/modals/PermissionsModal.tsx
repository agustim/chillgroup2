import React, { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Modal } from '../ui/Modal'
import type { Channel, ServerFullInfo } from '../../types'
import { channelGetMemberDevices, channelGetPermissions } from '../../lib/api'

interface PermissionsModalProps {
  isOpen: boolean
  onClose: () => void
  server: ServerFullInfo | null
  channels: Channel[]
  currentDeviceId?: string | null
}

interface PermissionsPanelProps {
  server: ServerFullInfo | null
  channels: Channel[]
  currentDeviceId?: string | null
}

type ChannelMemberDevice = {
  deviceId: string
  publicKey: string
  kemPublicKey: string
  dsaPublicKey: string
}

type ChannelPermissionRow = {
  userId: string
  username: string
  permissionLevel: number
  permission: 'none' | 'read' | 'write' | 'manage'
}

function PermissionsContent({
  server,
  channels,
  currentDeviceId,
  isOpen,
}: {
  server: ServerFullInfo | null
  channels: Channel[]
  currentDeviceId?: string | null
  isOpen: boolean
}) {
  const { t } = useTranslation()
  const [memberDevicesByChannel, setMemberDevicesByChannel] = useState<Record<string, ChannelMemberDevice[]>>({})
  const [permissionsByChannel, setPermissionsByChannel] = useState<Record<string, ChannelPermissionRow[]>>({})
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const asymmetricChannels = useMemo(
    () => channels.filter((channel) => channel.encryptionType === 'asymmetric'),
    [channels]
  )

  useEffect(() => {
    if (!isOpen || !server) {
      return
    }

    let cancelled = false
    const load = async () => {
      setLoading(true)
      setError('')
      const entries = await Promise.all(
        asymmetricChannels.map(async (channel) => {
          const result = await channelGetMemberDevices(channel.channelId)
          return [channel.channelId, result.success ? result.data : []] as const
        })
      )

      if (!cancelled) {
        setMemberDevicesByChannel(Object.fromEntries(entries))
        setLoading(false)
      }
    }

    void load().catch((err) => {
      if (!cancelled) {
        setError(err instanceof Error ? err.message : t('permissions.errLoad'))
        setLoading(false)
      }
    })

    return () => {
      cancelled = true
    }
  }, [isOpen, server, asymmetricChannels])

  const roleLabel = server?.myRole === 'owner' ? t('permissions.roleOwner') : server?.myRole === 'admin' ? t('permissions.roleAdmin') : t('permissions.roleMember')
  const canSeeManagement = server?.myRole === 'owner' || server?.myRole === 'admin'
  const serverScopedChannels = useMemo(() => channels.filter((channel) => channel.scope !== 'dm'), [channels])

  useEffect(() => {
    if (!isOpen || !server || !canSeeManagement) {
      return
    }

    let cancelled = false
    const loadPermissions = async () => {
      const entries = await Promise.all(
        serverScopedChannels.map(async (channel) => {
          const result = await channelGetPermissions(channel.channelId)
          return [channel.channelId, result.success ? result.data : []] as const
        })
      )

      if (!cancelled) {
        setPermissionsByChannel(Object.fromEntries(entries))
      }
    }

    void loadPermissions().catch((err) => {
      if (!cancelled) {
        setError(err instanceof Error ? err.message : t('permissions.errLoadUser'))
      }
    })

    return () => {
      cancelled = true
    }
  }, [isOpen, server, canSeeManagement, serverScopedChannels])

  return (
    <div className="modal-inline-stack">
        <section className="device-keys-section">
          <h4>{t('permissions.server')}</h4>
          {server ? (
            <>
              <p><strong>{server.name}</strong></p>
              <p>{t('permissions.yourRole')} <strong>{roleLabel}</strong></p>
              <p>{t('permissions.serverLabel')} <strong>{canSeeManagement ? t('permissions.mgmtVisible') : t('permissions.mgmtRestricted')}</strong></p>
            </>
          ) : (
            <p>{t('permissions.noServer')}</p>
          )}
        </section>

        <section className="device-keys-section">
          <h4>{t('permissions.channels')}</h4>
          {channels.length > 0 ? (
            <ul className="device-keys-list">
              {channels.map((channel) => (
                <li key={channel.channelId} className="device-keys-list-item">
                  <div className="device-keys-list-main">
                    <strong>{channel.type === 'voice' ? '🔊' : '#'} {channel.name}</strong>
                    <span>{t('permissions.privateLabel')} {channel.isPrivate ? t('common.yesLower') : t('common.noLower')}</span>
                    <span>{t('permissions.encryptionLabel')} {channel.encryptionType}</span>
                    <span>{t('permissions.keyVersionLabel')} {channel.keyVersion ?? t('permissions.noVersion')}</span>
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <p>{t('permissions.noChannels')}</p>
          )}
        </section>

        {canSeeManagement && (
          <section className="device-keys-section">
            <h4>{t('permissions.permsByUser')}</h4>
            {serverScopedChannels.length > 0 ? (
              serverScopedChannels.map((channel) => {
                const rows = permissionsByChannel[channel.channelId] ?? []
                return (
                  <div key={channel.channelId} style={{ marginBottom: '12px' }}>
                    <strong>{channel.type === 'voice' ? '🔊' : '#'} {channel.name}</strong>
                    <span style={{ display: 'block', marginTop: '4px', color: 'var(--text-muted)', fontSize: '12px' }}>
                      {channel.isPrivate ? t('permissions.channelPrivate') : t('permissions.channelPublic')}
                    </span>

                    {rows.length > 0 ? (
                      <div style={{ marginTop: '8px', overflowX: 'auto' }}>
                        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
                          <thead>
                            <tr>
                              <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>{t('permissions.thUser')}</th>
                              <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>{t('permissions.thLevel')}</th>
                              <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>{t('permissions.thPermission')}</th>
                            </tr>
                          </thead>
                          <tbody>
                            {rows.map((row) => (
                              <tr key={`${channel.channelId}-${row.userId}`}>
                                <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>{row.username}</td>
                                <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>{row.permissionLevel}</td>
                                <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>
                                  {channel.type === 'voice'
                                    ? row.permission === 'read' ? t('permissions.voiceRead')
                                      : row.permission === 'write' ? t('permissions.voiceWrite')
                                      : row.permission === 'manage' ? t('permissions.voiceManage')
                                      : row.permission
                                    : row.permission}
                                </td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    ) : (
                      <p style={{ marginTop: '8px' }}>{t('permissions.noPermsData')}</p>
                    )}
                  </div>
                )
              })
            ) : (
              <p>{t('permissions.noServerChannels')}</p>
            )}
          </section>
        )}

        <section className="device-keys-section">
          <h4>{t('permissions.asymKeys')}</h4>
          {loading && <p>{t('permissions.loadingKeys')}</p>}
          {error && <div className="modal-error">{error}</div>}
          {asymmetricChannels.length > 0 ? (
            asymmetricChannels.map((channel) => {
              const devices = memberDevicesByChannel[channel.channelId] ?? []
              return (
                <div key={channel.channelId} style={{ marginBottom: '12px' }}>
                  <strong>{channel.name}</strong>
                  {devices.length > 0 ? (
                    <ul className="device-keys-list" style={{ marginTop: '8px' }}>
                      {devices.map((device) => (
                        <li key={device.deviceId} className="device-keys-list-item">
                          <div className="device-keys-list-main">
                            <strong>{device.deviceId}</strong>
                            <span>{t('permissions.kem')} {device.kemPublicKey ? t('common.yesLower') : t('common.noLower')}</span>
                            <span>{t('permissions.dsa')} {device.dsaPublicKey ? t('common.yesLower') : t('common.noLower')}</span>
                            <span>{device.deviceId === currentDeviceId ? t('permissions.currentDevice') : t('permissions.memberDevice')}</span>
                          </div>
                        </li>
                      ))}
                    </ul>
                  ) : (
                    <p>{t('permissions.noDevices')}</p>
                  )}
                </div>
              )
            })
          ) : (
            <p>{t('permissions.noAsymChannels')}</p>
          )}
        </section>
      </div>
  )
}

function PermissionsModal({ isOpen, onClose, server, channels, currentDeviceId }: PermissionsModalProps) {
  const { t } = useTranslation()
  return (
    <Modal isOpen={isOpen} onClose={onClose} title={t('permissions.title')}>
      <PermissionsContent
        isOpen={isOpen}
        server={server}
        channels={channels}
        currentDeviceId={currentDeviceId}
      />
    </Modal>
  )
}

export function PermissionsPanel({ server, channels, currentDeviceId }: PermissionsPanelProps) {
  return (
    <PermissionsContent
      isOpen={true}
      server={server}
      channels={channels}
      currentDeviceId={currentDeviceId}
    />
  )
}