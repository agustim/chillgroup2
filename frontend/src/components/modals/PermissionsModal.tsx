import React, { useEffect, useMemo, useState } from 'react'

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
        setError(err instanceof Error ? err.message : 'No s\'han pogut carregar els permisos')
        setLoading(false)
      }
    })

    return () => {
      cancelled = true
    }
  }, [isOpen, server, asymmetricChannels])

  const roleLabel = server?.myRole === 'owner' ? 'Propietari' : server?.myRole === 'admin' ? 'Administrador' : 'Membre'
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
        setError(err instanceof Error ? err.message : 'No s\'han pogut carregar els permisos per usuari')
      }
    })

    return () => {
      cancelled = true
    }
  }, [isOpen, server, canSeeManagement, serverScopedChannels])

  return (
    <div className="modal-inline-stack">
        <section className="device-keys-section">
          <h4>Servidor</h4>
          {server ? (
            <>
              <p><strong>{server.name}</strong></p>
              <p>El teu rol: <strong>{roleLabel}</strong></p>
              <p>Servidor: <strong>{canSeeManagement ? 'gestió visible' : 'vista restringida'}</strong></p>
            </>
          ) : (
            <p>No hi ha servidor seleccionat.</p>
          )}
        </section>

        <section className="device-keys-section">
          <h4>Canals</h4>
          {channels.length > 0 ? (
            <ul className="device-keys-list">
              {channels.map((channel) => (
                <li key={channel.channelId} className="device-keys-list-item">
                  <div className="device-keys-list-main">
                    <strong>{channel.type === 'voice' ? '🔊' : '#'} {channel.name}</strong>
                    <span>Privat: {channel.isPrivate ? 'sí' : 'no'}</span>
                    <span>Encriptació: {channel.encryptionType}</span>
                    <span>KeyVersion: {channel.keyVersion ?? 'sense versió'}</span>
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <p>No hi ha canals carregats.</p>
          )}
        </section>

        {canSeeManagement && (
          <section className="device-keys-section">
            <h4>Permisos per usuari i canal</h4>
            {serverScopedChannels.length > 0 ? (
              serverScopedChannels.map((channel) => {
                const rows = permissionsByChannel[channel.channelId] ?? []
                return (
                  <div key={channel.channelId} style={{ marginBottom: '12px' }}>
                    <strong>{channel.type === 'voice' ? '🔊' : '#'} {channel.name}</strong>
                    <span style={{ display: 'block', marginTop: '4px', color: 'var(--text-muted)', fontSize: '12px' }}>
                      {channel.isPrivate ? 'Canal privat (permís explícit)' : 'Canal públic (permís per rol de servidor)'}
                    </span>

                    {rows.length > 0 ? (
                      <div style={{ marginTop: '8px', overflowX: 'auto' }}>
                        <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '13px' }}>
                          <thead>
                            <tr>
                              <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>Usuari</th>
                              <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>Nivell</th>
                              <th style={{ textAlign: 'left', borderBottom: '1px solid var(--bg-active)', padding: '6px 4px' }}>Permís</th>
                            </tr>
                          </thead>
                          <tbody>
                            {rows.map((row) => (
                              <tr key={`${channel.channelId}-${row.userId}`}>
                                <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>{row.username}</td>
                                <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>{row.permissionLevel}</td>
                                <td style={{ padding: '6px 4px', borderBottom: '1px solid var(--bg-active)' }}>
                                  {channel.type === 'voice'
                                    ? row.permission === 'read' ? 'escoltar-veure'
                                      : row.permission === 'write' ? 'parlar-mostrar'
                                      : row.permission === 'manage' ? 'manager'
                                      : row.permission
                                    : row.permission}
                                </td>
                              </tr>
                            ))}
                          </tbody>
                        </table>
                      </div>
                    ) : (
                      <p style={{ marginTop: '8px' }}>Sense dades de permisos per aquest canal.</p>
                    )}
                  </div>
                )
              })
            ) : (
              <p>No hi ha canals de servidor per mostrar permisos.</p>
            )}
          </section>
        )}

        <section className="device-keys-section">
          <h4>Claus asimètriques per canal</h4>
          {loading && <p>Carregant claus...</p>}
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
                            <span>KEM: {device.kemPublicKey ? 'sí' : 'no'}</span>
                            <span>DSA: {device.dsaPublicKey ? 'sí' : 'no'}</span>
                            <span>{device.deviceId === currentDeviceId ? 'Dispositiu actual' : 'Dispositiu membre'}</span>
                          </div>
                        </li>
                      ))}
                    </ul>
                  ) : (
                    <p>No hi ha dispositius membres visibles per aquest canal.</p>
                  )}
                </div>
              )
            })
          ) : (
            <p>No hi ha canals asimètrics.</p>
          )}
        </section>
      </div>
  )
}

function PermissionsModal({ isOpen, onClose, server, channels, currentDeviceId }: PermissionsModalProps) {
  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Permisos i accessos">
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