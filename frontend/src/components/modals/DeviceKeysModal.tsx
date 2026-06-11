import React, { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../shared/Button'
import {
  deleteDeviceKeypair,
  exportDeviceKeypair,
  generateAndStoreDeviceKeypair,
  getDeviceKeySummary,
  KeypairDeviceIdExistsError,
  importAndStoreDeviceKeypair,
  listDeviceKeypairs,
} from '../../lib/device-keys'
import { persistDeviceId } from '../../lib/device-identity'
import { userDevicesList, userDeviceRevoke } from '../../lib/api'

interface DeviceKeysBaseProps {
  currentDeviceId: string | null
  channels?: Array<{
    channelId: string
    name: string
  }>
  devices?: Array<{
    deviceId: string
    label: string
    revoked: boolean
    lastSeen: string
  }>
}

interface DeviceKeysPanelProps extends DeviceKeysBaseProps {}

interface DeviceKeysContentProps extends DeviceKeysBaseProps {
  isActive: boolean
}

function DeviceKeysContent({
  isActive,
  currentDeviceId,
  channels = [],
  devices = [],
}: DeviceKeysContentProps) {
  const { t } = useTranslation()
  const [isBusy, setIsBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)
  const [deviceSummary, setDeviceSummary] = useState<{
    hasKeypair: boolean
    kemPublicKeyPreview: string | null
    dsaPublicKeyPreview: string | null
    hasSigningKeypair: boolean
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
    kemPublicKey: string
    dsaPublicKey: string
    hasKemPublicKey: boolean
    hasDsaPublicKey: boolean
    createdAt?: string
    lastSeen: string
    revoked: boolean
    isCurrent: boolean
  }>>([])
  const [pendingOverwrite, setPendingOverwrite] = useState<
    { kind: 'generate' } | { kind: 'import'; text: string } | null
  >(null)

  const [deviceImportText, setDeviceImportText] = useState('')
  const [exportedDeviceBundle, setExportedDeviceBundle] = useState('')

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
          label: server?.label ?? (deviceId === currentDeviceId ? t('deviceKeys.deviceCurrentLabel') : t('deviceKeys.deviceLocalLabel')),
          local,
          server,
          isCurrent: server?.isCurrent ?? deviceId === currentDeviceId,
          hasLocalKeypair: !!local,
          isRemoteOnly: !local && !!server,
          isLocalOnly: !!local && !server,
          hasKemPublicKey: server?.hasKemPublicKey ?? false,
          hasDsaPublicKey: server?.hasDsaPublicKey ?? false,
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
    const [summary, pairs, devicesResult] = await Promise.all([
      currentDeviceId ? getDeviceKeySummary(currentDeviceId) : Promise.resolve(null),
      listDeviceKeypairs(),
      userDevicesList(),
    ])

    setDeviceSummary(summary)
    setKeypairs(pairs)

    if (devicesResult.success) {
      setServerDevices(devicesResult.data.map((device) => ({
        deviceId: device.deviceId,
        label: device.label,
        kemPublicKey: device.kemPublicKey,
        dsaPublicKey: device.dsaPublicKey,
        hasKemPublicKey: device.hasKemPublicKey ?? false,
        hasDsaPublicKey: device.hasDsaPublicKey ?? false,
        createdAt: device.createdAt,
        lastSeen: device.lastSeen,
        revoked: device.revoked,
        isCurrent: device.isCurrent ?? false,
      })))
    } else {
      setServerDevices([])
      throw new Error(devicesResult.error.message || t('deviceKeys.errLoadDevices'))
    }
  }

  useEffect(() => {
    if (!isActive) {
      return
    }

    setError(null)
    setSuccess(null)
    setPendingOverwrite(null)
    setKeypairDeviceId(currentDeviceId ?? '')
    setExportedDeviceBundle('')
    setDeviceImportText('')
    void refreshState()
  }, [isActive, currentDeviceId])

  const handleGenerateDeviceKeys = async (overwrite = false) => {
    const resolvedDeviceId = keypairDeviceId.trim()
    if (!resolvedDeviceId) {
      setError(t('deviceKeys.errDeviceIdRequired'))
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
      setSuccess(t('deviceKeys.successGenerate'))
    } catch (err) {
      if (err instanceof KeypairDeviceIdExistsError) {
        setPendingOverwrite({ kind: 'generate' })
        setError(t('deviceKeys.overwriteHint', { message: err.message }))
      } else {
        setError(t('deviceKeys.errGenerate'))
      }
    } finally {
      setIsBusy(false)
    }
  }

  const handleImportDeviceKeys = async (overwrite = false, forcedText?: string) => {
    const payload = forcedText ?? deviceImportText
    if (!payload.trim()) {
      setError(t('deviceKeys.errPasteImport'))
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
      setSuccess(t('deviceKeys.successImport', { deviceId: bundle.deviceId }))
      setDeviceImportText('')
    } catch (err) {
      if (err instanceof KeypairDeviceIdExistsError) {
        setPendingOverwrite({ kind: 'import', text: payload })
        setError(t('deviceKeys.overwriteHint', { message: err.message }))
      } else {
        const msg = err instanceof Error ? err.message : t('deviceKeys.errImport')
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
      setSuccess(t('deviceKeys.successExport', { deviceId }))
    } catch (err) {
      const msg = err instanceof Error ? err.message : t('deviceKeys.errExport')
      setError(msg)
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
      setSuccess(t('deviceKeys.successDelete', { deviceId }))
    } catch {
      setError(t('deviceKeys.errDelete'))
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
      setSuccess(t('deviceKeys.successRevoke', { deviceId }))
    } catch {
      setError(t('deviceKeys.errRevoke'))
    } finally {
      setIsBusy(false)
    }
  }

  return (
    <div className="device-keys-modal">
      {error && <div className="modal-error">{error}</div>}
      {success && <div className="modal-success">{success}</div>}

      <section className="device-keys-section">
        <h4>{t('deviceKeys.activeDevice')}</h4>
        <p>
          {t('deviceKeys.idLabel')} <strong>{currentDeviceId ?? t('deviceKeys.notAvailable')}</strong>
        </p>
        {activeDevice && (
          <p>
            {t('deviceKeys.labelLabel')} <strong>{activeDevice.label}</strong> · {t('deviceKeys.statusLabel')}{' '}
            {activeDevice.revoked ? t('deviceKeys.revoked') : t('deviceKeys.active')}
          </p>
        )}
        <p>
          {t('deviceKeys.localKeypair')}{' '}
          <strong>{deviceSummary?.hasKeypair ? t('deviceKeys.present') : t('deviceKeys.notFound')}</strong>
        </p>
        <p>
          {t('deviceKeys.localSigning')}{' '}
          <strong>{deviceSummary?.hasSigningKeypair ? t('deviceKeys.present') : t('deviceKeys.notFound')}</strong>
        </p>
        {deviceSummary?.kemPublicKeyPreview && (
          <p>{t('deviceKeys.kemPublic')} {deviceSummary.kemPublicKeyPreview}</p>
        )}
        {deviceSummary?.dsaPublicKeyPreview && (
          <p>{t('deviceKeys.dsaPublic')} {deviceSummary.dsaPublicKeyPreview}</p>
        )}
        <div className="device-keys-form-grid">
          <input
            className="device-keys-input"
            type="text"
            value={keypairDeviceId}
            onChange={(e) => setKeypairDeviceId(e.target.value)}
            placeholder={t('deviceKeys.deviceIdPlaceholder')}
            disabled={isBusy}
          />
        </div>
        <div className="modal-form-actions">
          <Button variant="primary" size="sm" onClick={() => void handleGenerateDeviceKeys(false)} disabled={isBusy}>
            {t('deviceKeys.generateKeys')}
          </Button>
          {pendingOverwrite?.kind === 'generate' && (
            <Button variant="danger" size="sm" onClick={() => void handleGenerateDeviceKeys(true)} disabled={isBusy}>
              {t('deviceKeys.overwrite')}
            </Button>
          )}
        </div>
      </section>

      <section className="device-keys-section">
        <h4>{t('deviceKeys.accountDevices')}</h4>
        {mergedDevices.length === 0 ? (
          <p>{t('deviceKeys.noDevices')}</p>
        ) : (
          <ul className="device-keys-list">
            {mergedDevices.map((device) => (
              <li key={device.deviceId} className="device-keys-list-item">
                <div className="device-keys-list-main">
                  <strong>{device.label} · {device.deviceId}</strong>
                  <span>
                    {device.isCurrent ? t('deviceKeys.current') : ''}
                    {device.isRemoteOnly ? t('deviceKeys.remoteOnly') : device.isLocalOnly ? t('deviceKeys.localOnly') : t('deviceKeys.localAndServer')}
                  </span>
                  {device.server && (
                    <span>
                      {t('deviceKeys.kem')} {device.hasKemPublicKey ? t('deviceKeys.registered') : t('deviceKeys.pending')} ·
                      {t('deviceKeys.dsa')} {device.hasDsaPublicKey ? t('deviceKeys.registered') : t('deviceKeys.pending')} ·
                      {t('deviceKeys.statusLabel')} {device.server.revoked ? t('deviceKeys.statusRevokedLower') : t('deviceKeys.statusActiveLower')}
                    </span>
                  )}
                  {device.server?.lastSeen && (
                    <span>{t('deviceKeys.lastAccess')} {new Date(device.server.lastSeen).toLocaleString()}</span>
                  )}
                  {device.local && (
                    <span>{t('deviceKeys.localKeypairUpdated')} {new Date(device.local.updatedAt).toLocaleString()}</span>
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
                      {t('deviceKeys.backupLocal')}
                    </Button>
                  )}
                  {device.local && (
                    <Button
                      variant="danger"
                      size="sm"
                      onClick={() => void handleDeleteKeypair(device.deviceId)}
                      disabled={isBusy}
                    >
                      {t('deviceKeys.eraseLocal')}
                    </Button>
                  )}
                  {device.server && !device.isCurrent && !device.server.revoked && (
                    <Button
                      variant="danger"
                      size="sm"
                      onClick={() => void handleRevokeServerDevice(device.deviceId)}
                      disabled={isBusy}
                    >
                      {t('deviceKeys.removeFromServer')}
                    </Button>
                  )}
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="device-keys-section">
        <h4>{t('deviceKeys.importTitle')}</h4>
        <textarea
          className="device-keys-textarea"
          value={deviceImportText}
          onChange={(e) => setDeviceImportText(e.target.value)}
          placeholder={t('deviceKeys.importPlaceholder')}
          rows={5}
          disabled={isBusy}
        />
        <div className="modal-form-actions">
          <Button variant="secondary" size="sm" onClick={() => void handleImportDeviceKeys(false)} disabled={isBusy}>
            {t('deviceKeys.importKeypair')}
          </Button>
          {pendingOverwrite?.kind === 'import' && (
            <Button
              variant="danger"
              size="sm"
              onClick={() => void handleImportDeviceKeys(true, pendingOverwrite.text)}
              disabled={isBusy}
            >
              {t('deviceKeys.overwrite')}
            </Button>
          )}
        </div>
      </section>

      {exportedDeviceBundle && (
        <section className="device-keys-section">
          <h4>{t('deviceKeys.backupTitle')}</h4>
          <textarea className="device-keys-textarea" value={exportedDeviceBundle} readOnly rows={6} />
        </section>
      )}

      {channels.length > 0 && (
        <div style={{ display: 'none' }} aria-hidden="true">{channels.length}</div>
      )}
    </div>
  )
}

export function DeviceKeysPanel(props: DeviceKeysPanelProps) {
  return <DeviceKeysContent isActive={true} {...props} />
}
