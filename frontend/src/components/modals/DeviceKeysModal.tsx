import React, { useEffect, useMemo, useState } from 'react'

import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'
import {
  deleteDeviceKeypair,
  deleteSymmetricChannelKey,
  exportDeviceKeypair,
  exportSymmetricChannelKeys,
  generateAndStoreDeviceKeypair,
  getDeviceKeySummary,
  KeypairDeviceIdExistsError,
  importAndStoreDeviceKeypair,
  importSymmetricChannelKeys,
  listDeviceKeypairs,
  listSymmetricChannelKeys,
} from '../../lib/device-keys'
import { persistDeviceId } from '../../lib/device-identity'
import { userDevicesList, userDeviceRevoke } from '../../lib/api'

interface DeviceKeysModalProps {
  isOpen: boolean
  onClose: () => void
  currentDeviceId: string | null
  devices?: Array<{
    deviceId: string
    label: string
    revoked: boolean
    lastSeen: string
  }>
}

export function DeviceKeysModal({
  isOpen,
  onClose,
  currentDeviceId,
  devices = [],
}: DeviceKeysModalProps) {
  const [activeTab, setActiveTab] = useState<'device' | 'channels'>('device')
  const [isBusy, setIsBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)
  const [deviceSummary, setDeviceSummary] = useState<{
    hasKeypair: boolean
    publicKeyPreview: string | null
  } | null>(null)
  const [keypairDeviceId, setKeypairDeviceId] = useState('')
  const [keypairs, setKeypairs] = useState<Array<{
    deviceId: string
    createdAt: number
    updatedAt: number
  }>>([])
  const [serverDevices, setServerDevices] = useState<Array<{
    deviceId: string
    label: string
    publicKey: string
    hasPublicKey: boolean
    createdAt?: string
    lastSeen: string
    revoked: boolean
    isCurrent: boolean
  }>>([])
  const [symmetricKeys, setSymmetricKeys] = useState<Array<{
    channelId: string
    keyVersion: number
    acquiredAt: number
    preview: string
  }>>([])
  const [pendingOverwrite, setPendingOverwrite] = useState<
    { kind: 'generate' } | { kind: 'import'; text: string } | null
  >(null)

  const [deviceImportText, setDeviceImportText] = useState('')
  const [symImportText, setSymImportText] = useState('')
  const [exportedDeviceBundle, setExportedDeviceBundle] = useState('')
  const [exportedSymmetricBundle, setExportedSymmetricBundle] = useState('')

  const activeDevice = useMemo(
    () => serverDevices.find((item) => item.deviceId === currentDeviceId)
      ?? devices.find((item) => item.deviceId === currentDeviceId)
      ?? null,
    [serverDevices, devices, currentDeviceId]
  )

  const mergedDevices = useMemo(() => {
    const localMap = new Map(keypairs.map((pair) => [pair.deviceId, pair]))
    const serverMap = new Map(serverDevices.map((device) => [device.deviceId, device]))
    const ids = new Set([...localMap.keys(), ...serverMap.keys()])

    return Array.from(ids)
      .map((deviceId) => {
        const local = localMap.get(deviceId) ?? null
        const server = serverMap.get(deviceId) ?? null
        return {
          deviceId,
          label: server?.label ?? (deviceId === currentDeviceId ? 'Dispositiu actual' : 'Dispositiu local'),
          local,
          server,
          isCurrent: server?.isCurrent ?? deviceId === currentDeviceId,
          hasLocalKeypair: !!local,
          isRemoteOnly: !local && !!server,
          isLocalOnly: !!local && !server,
          hasPublicKey: server?.hasPublicKey ?? false,
        }
      })
      .sort((a, b) => {
        if (a.isCurrent) return -1
        if (b.isCurrent) return 1
        if (a.server && !b.server) return -1
        if (!a.server && b.server) return 1
        return a.deviceId.localeCompare(b.deviceId)
      })
  }, [keypairs, serverDevices, currentDeviceId])

  const refreshState = async () => {
    const [summary, pairs, symKeys, devicesResult] = await Promise.all([
      currentDeviceId ? getDeviceKeySummary(currentDeviceId) : Promise.resolve(null),
      listDeviceKeypairs(),
      listSymmetricChannelKeys(),
      userDevicesList(),
    ])

    setDeviceSummary(summary)
    setKeypairs(pairs)
    setSymmetricKeys(symKeys)
    if (devicesResult.success) {
      setServerDevices(devicesResult.data.map((device) => ({
        deviceId: device.deviceId,
        label: device.label,
        publicKey: device.publicKey,
        hasPublicKey: device.hasPublicKey ?? false,
        createdAt: device.createdAt,
        lastSeen: device.lastSeen,
        revoked: device.revoked,
        isCurrent: device.isCurrent ?? false,
      })))
    } else {
      setServerDevices([])
      throw new Error(devicesResult.error.message || 'No s\'ha pogut carregar la llista de dispositius del servidor')
    }
  }

  useEffect(() => {
    if (!isOpen) {
      return
    }

    setError(null)
    setSuccess(null)
    setPendingOverwrite(null)
    setActiveTab('device')
    setKeypairDeviceId(currentDeviceId ?? '')
    setExportedDeviceBundle('')
    setExportedSymmetricBundle('')
    void refreshState()
  }, [isOpen, currentDeviceId])

  const handleGenerateDeviceKeys = async (overwrite = false) => {
    const resolvedDeviceId = keypairDeviceId.trim()
    if (!resolvedDeviceId) {
      setError('Has d\'indicar un deviceId pel parell de claus')
      return
    }

    setIsBusy(true)
    setError(null)
    setSuccess(null)
    setPendingOverwrite(null)

    try {
      await generateAndStoreDeviceKeypair(resolvedDeviceId, overwrite)
      persistDeviceId(resolvedDeviceId)
      await refreshState()
      setSuccess('Parell de claus ML-KEM-1024 generat i guardat localment')
    } catch (err) {
      if (err instanceof KeypairDeviceIdExistsError) {
        setPendingOverwrite({ kind: 'generate' })
        setError(`${err.message}. Prem "Sobrescriure" si ho vols substituir.`)
      } else {
        setError('No s\'han pogut generar les claus del dispositiu')
      }
    } finally {
      setIsBusy(false)
    }
  }

  const handleImportDeviceKeys = async (overwrite = false, forcedText?: string) => {
    const payload = forcedText ?? deviceImportText
    if (!payload.trim()) {
      setError('Enganxa el JSON del backup del dispositiu')
      return
    }

    setIsBusy(true)
    setError(null)
    setSuccess(null)
    setPendingOverwrite(null)

    try {
      const bundle = await importAndStoreDeviceKeypair(payload, overwrite)
      persistDeviceId(bundle.deviceId)
      await refreshState()
      setSuccess(`Backup de dispositiu importat (${bundle.deviceId})`)
      setDeviceImportText('')
    } catch (err) {
      if (err instanceof KeypairDeviceIdExistsError) {
        setPendingOverwrite({ kind: 'import', text: payload })
        setError(`${err.message}. Prem "Sobrescriure" si ho vols substituir.`)
      } else {
        const msg = err instanceof Error ? err.message : 'No s\'ha pogut importar el backup'
        setError(msg)
      }
    } finally {
      setIsBusy(false)
    }
  }

  const handleExportDeviceKeys = async (deviceId: string) => {
    setIsBusy(true)
    setError(null)
    setSuccess(null)

    try {
      const json = await exportDeviceKeypair(deviceId)
      setExportedDeviceBundle(json)
      setSuccess(`Backup de claus preparat (${deviceId})`)
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'No s\'ha pogut exportar el keypair'
      setError(msg)
    } finally {
      setIsBusy(false)
    }
  }

  const handleExportSymmetric = async () => {
    setIsBusy(true)
    setError(null)
    setSuccess(null)

    try {
      const json = await exportSymmetricChannelKeys()
      setExportedSymmetricBundle(json)
      setSuccess('Exportació de claus simètriques preparada')
    } catch {
      setError('No s\'han pogut exportar les claus simètriques')
    } finally {
      setIsBusy(false)
    }
  }

  const handleDeleteKeypair = async (deviceId: string) => {
    setIsBusy(true)
    setError(null)
    setSuccess(null)
    try {
      await deleteDeviceKeypair(deviceId)
      await refreshState()
      setSuccess(`Parell de claus "${deviceId}" esborrat`)
    } catch {
      setError('No s\'ha pogut esborrar el parell de claus')
    } finally {
      setIsBusy(false)
    }
  }

  const handleRevokeServerDevice = async (deviceId: string) => {
    setIsBusy(true)
    setError(null)
    setSuccess(null)
    try {
      const result = await userDeviceRevoke(deviceId)
      if (!result.success) {
        setError(result.error.message)
        return
      }
      await refreshState()
      setSuccess(`Dispositiu remot "${deviceId}" eliminat del servidor`)
    } catch {
      setError('No s\'ha pogut eliminar el dispositiu remot del servidor')
    } finally {
      setIsBusy(false)
    }
  }

  const handleDeleteSymmetric = async (channelId: string) => {
    setIsBusy(true)
    setError(null)
    setSuccess(null)
    try {
      await deleteSymmetricChannelKey(channelId)
      await refreshState()
      setSuccess(`Clau simètrica del canal ${channelId} eliminada`)
    } catch {
      setError('No s\'ha pogut eliminar la clau simètrica')
    } finally {
      setIsBusy(false)
    }
  }

  const handleImportSymmetric = async () => {
    if (!symImportText.trim()) {
      setError('Enganxa el JSON de claus simètriques')
      return
    }

    setIsBusy(true)
    setError(null)
    setSuccess(null)

    try {
      const imported = await importSymmetricChannelKeys(symImportText)
      await refreshState()
      setSuccess(`Importades ${imported} claus simètriques de canals`)
      setSymImportText('')
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'No s\'han pogut importar les claus'
      setError(msg)
    } finally {
      setIsBusy(false)
    }
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Gestió de claus de dispositiu i canals">
      <div className="device-keys-modal">
        {error && <div className="modal-error">{error}</div>}
        {success && <div className="modal-success">{success}</div>}

        <div className="device-keys-tabs" role="tablist" aria-label="Gestió de claus">
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'device'}
            className={`device-keys-tab ${activeTab === 'device' ? 'active' : ''}`}
            onClick={() => setActiveTab('device')}
          >
            Gestió dispositiu
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={activeTab === 'channels'}
            className={`device-keys-tab ${activeTab === 'channels' ? 'active' : ''}`}
            onClick={() => setActiveTab('channels')}
          >
            Gestió canals
          </button>
        </div>

        {activeTab === 'device' && (
        <>
        <section className="device-keys-section">
          <h4>Dispositiu actiu</h4>
          <p>
            ID: <strong>{currentDeviceId ?? 'No disponible'}</strong>
          </p>
          {activeDevice && (
            <p>
              Etiqueta: <strong>{activeDevice.label}</strong> · Estat:{' '}
              {activeDevice.revoked ? 'Revocat' : 'Actiu'}
            </p>
          )}
          <p>
            Keypair local:{' '}
            <strong>{deviceSummary?.hasKeypair ? 'Present' : 'No trobat'}</strong>
          </p>
          {deviceSummary?.publicKeyPreview && (
            <p>Public key (preview): {deviceSummary.publicKeyPreview}</p>
          )}
          <div className="device-keys-form-grid">
            <input
              className="device-keys-input"
              type="text"
              value={keypairDeviceId}
              onChange={(e) => setKeypairDeviceId(e.target.value)}
              placeholder="Device ID del parell de claus"
              disabled={isBusy}
            />
          </div>
          <div className="modal-form-actions">
            <Button variant="primary" size="sm" onClick={() => void handleGenerateDeviceKeys(false)} disabled={isBusy}>
              Generar claus noves
            </Button>
            {pendingOverwrite?.kind === 'generate' && (
              <Button variant="danger" size="sm" onClick={() => void handleGenerateDeviceKeys(true)} disabled={isBusy}>
                Sobrescriure
              </Button>
            )}
          </div>
        </section>

        <section className="device-keys-section">
          <h4>Dispositius del compte</h4>
          {mergedDevices.length === 0 ? (
            <p>Encara no hi ha dispositius disponibles.</p>
          ) : (
            <ul className="device-keys-list">
              {mergedDevices.map((device) => (
                <li key={device.deviceId} className="device-keys-list-item">
                  <div className="device-keys-list-main">
                    <strong>{device.label} · {device.deviceId}</strong>
                    <span>
                      {device.isCurrent ? 'Actual · ' : ''}
                      {device.isRemoteOnly ? 'Només servidor' : device.isLocalOnly ? 'Només local' : 'Local + servidor'}
                    </span>
                    {device.server && (
                      <span>
                        Clau pública: {device.hasPublicKey ? 'registrada' : 'pendent'} · 
                        Estat: {device.server.revoked ? 'revocat' : 'actiu'}
                      </span>
                    )}
                    {device.server?.lastSeen && (
                      <span>Darrer accés: {new Date(device.server.lastSeen).toLocaleString()}</span>
                    )}
                    {device.local && (
                      <span>Keypair local actualitzat: {new Date(device.local.updatedAt).toLocaleString()}</span>
                    )}
                  </div>
                  <div className="device-keys-list-actions">
                    {device.local && (
                      <Button
                        variant="secondary"
                        size="sm"
                        onClick={() => void handleExportDeviceKeys(device.deviceId)}
                        disabled={isBusy}
                      >
                        Backup local
                      </Button>
                    )}
                    {device.local && (
                      <Button
                        variant="danger"
                        size="sm"
                        onClick={() => void handleDeleteKeypair(device.deviceId)}
                        disabled={isBusy}
                      >
                        Esborrar local
                      </Button>
                    )}
                    {device.server && !device.isCurrent && !device.server.revoked && (
                      <Button
                        variant="danger"
                        size="sm"
                        onClick={() => void handleRevokeServerDevice(device.deviceId)}
                        disabled={isBusy}
                      >
                        Eliminar del servidor
                      </Button>
                    )}
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>

        <section className="device-keys-section">
          <h4>Importar keypair de dispositiu</h4>
          <textarea
            className="device-keys-textarea"
            value={deviceImportText}
            onChange={(e) => setDeviceImportText(e.target.value)}
            placeholder="Enganxa aquí el JSON exportat del keypair"
            rows={5}
            disabled={isBusy}
          />
          <div className="modal-form-actions">
            <Button variant="secondary" size="sm" onClick={() => void handleImportDeviceKeys(false)} disabled={isBusy}>
              Importar keypair
            </Button>
            {pendingOverwrite?.kind === 'import' && (
              <Button
                variant="danger"
                size="sm"
                onClick={() => void handleImportDeviceKeys(true, pendingOverwrite.text)}
                disabled={isBusy}
              >
                Sobrescriure
              </Button>
            )}
          </div>
        </section>

        {exportedDeviceBundle && (
          <section className="device-keys-section">
            <h4>Backup del dispositiu (JSON)</h4>
            <textarea className="device-keys-textarea" value={exportedDeviceBundle} readOnly rows={6} />
          </section>
        )}
        </>
        )}

        {activeTab === 'channels' && (
        <>
        <section className="device-keys-section">
          <h4>Claus simètriques de canals</h4>
          <p>Claus simètriques guardades localment: <strong>{symmetricKeys.length}</strong></p>
          <div className="modal-form-actions">
            <Button variant="secondary" size="sm" onClick={handleExportSymmetric} disabled={isBusy}>
              Exportar simètriques
            </Button>
          </div>
          {symmetricKeys.length > 0 && (
            <ul className="device-keys-list">
              {symmetricKeys.map((key) => (
                <li key={`${key.channelId}-${key.keyVersion}`} className="device-keys-list-item">
                  <div className="device-keys-list-main">
                    <strong>Canal {key.channelId} · v{key.keyVersion}</strong>
                    <span>Clau: {key.preview}</span>
                    <span>Guardada: {new Date(key.acquiredAt).toLocaleString()}</span>
                  </div>
                  <div className="device-keys-list-actions">
                    <Button
                      variant="danger"
                      size="sm"
                      onClick={() => void handleDeleteSymmetric(key.channelId)}
                      disabled={isBusy}
                    >
                      Esborrar
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          )}
          <textarea
            className="device-keys-textarea"
            value={symImportText}
            onChange={(e) => setSymImportText(e.target.value)}
            placeholder="Enganxa aquí JSON de claus simètriques"
            rows={5}
            disabled={isBusy}
          />
          <div className="modal-form-actions">
            <Button variant="secondary" size="sm" onClick={handleImportSymmetric} disabled={isBusy}>
              Importar simètriques
            </Button>
          </div>
        </section>

        {exportedSymmetricBundle && (
          <section className="device-keys-section">
            <h4>Backup de claus simètriques (JSON)</h4>
            <textarea className="device-keys-textarea" value={exportedSymmetricBundle} readOnly rows={6} />
          </section>
        )}
        </>
        )}
      </div>
    </Modal>
  )
}
