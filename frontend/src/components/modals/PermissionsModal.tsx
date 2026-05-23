import React, { useEffect, useMemo, useState } from 'react'

import { Modal } from '../ui/Modal'
import type { Channel, ServerFullInfo } from '../../types'
import { channelGetMemberDevices } from '../../lib/api'

interface PermissionsModalProps {
  isOpen: boolean
  onClose: () => void
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

export function PermissionsModal({ isOpen, onClose, server, channels, currentDeviceId }: PermissionsModalProps) {
  const [memberDevicesByChannel, setMemberDevicesByChannel] = useState<Record<string, ChannelMemberDevice[]>>({})
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

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Permisos i accessos">
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
    </Modal>
  )
}