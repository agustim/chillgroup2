import React, { useEffect, useMemo, useState } from 'react'

import { Button } from '../shared/Button'
import { channelsList, serversList } from '../../lib/api'
import {
  deleteSymmetricChannelKey,
  exportAsymmetricChannelKeys,
  exportSymmetricChannelKeys,
  importAsymmetricChannelKeys,
  importSymmetricChannelKeys,
  listChannelKeys,
  listSymmetricChannelKeys,
} from '../../lib/device-keys'
import type { Channel } from '../../types'

interface ChannelKeysPanelProps {
  channels?: Channel[]
  serverName?: string
}

interface ChannelKeysContentProps {
  isActive: boolean
  channels?: Channel[]
  serverName?: string
}

function ChannelKeysContent({ isActive, channels = [], serverName }: ChannelKeysContentProps) {
  const [isBusy, setIsBusy] = useState(false)
  const [error, setError] = useState('')
  const [success, setSuccess] = useState('')
  const [symmetricKeys, setSymmetricKeys] = useState<Array<{
    channelId: string
    keyVersion: number
    acquiredAt: number
    preview: string
  }>>([])
  const [asymmetricKeys, setAsymmetricKeys] = useState<Array<{
    channelId: string
    keyVersion: number
    keyVersionId: string | null
    acquiredAt: number
  }>>([])
  const [symImportText, setSymImportText] = useState('')
  const [asymImportText, setAsymImportText] = useState('')
  const [exportedSymmetricBundle, setExportedSymmetricBundle] = useState('')
  const [exportedAsymmetricBundle, setExportedAsymmetricBundle] = useState('')
  const [channelLabelById, setChannelLabelById] = useState<Record<string, string>>({})

  const channelNameById = useMemo(
    () => new Map(channels.map((channel) => [channel.channelId, channel.name])),
    [channels]
  )

  const fallbackChannelLabelById = useMemo(() => {
    const map: Record<string, string> = {}
    for (const channel of channels) {
      map[channel.channelId] = serverName ? `${serverName} · #${channel.name}` : `#${channel.name}`
    }
    return map
  }, [channels, serverName])

  const formatChannelLabel = (channelId: string) => {
    const knownLabel = channelLabelById[channelId] ?? fallbackChannelLabelById[channelId]
    if (knownLabel) {
      return knownLabel
    }

    const name = channelNameById.get(channelId)
    if (name && serverName) {
      return `${serverName} · #${name}`
    }
    if (name) {
      return `#${name}`
    }
    return 'Canal desconegut'
  }

  const getRequiredChannelIds = (
    symKeys: Array<{ channelId: string }>,
    asymKeys: Array<{ channelId: string }>
  ) => Array.from(new Set([...symKeys.map((key) => key.channelId), ...asymKeys.map((key) => key.channelId)]))

  const refreshState = async () => {
    const [symKeys, channelKeys] = await Promise.all([listSymmetricChannelKeys(), listChannelKeys()])
    const mappedAsymmetricKeys =
      channelKeys
        .filter((entry) => entry.type === 'asymmetric')
        .map((entry) => ({
          channelId: entry.channelId,
          keyVersion: entry.keyVersion,
          keyVersionId: entry.keyVersionId ?? null,
          acquiredAt: entry.acquiredAt,
        }))
        .sort((a, b) => b.acquiredAt - a.acquiredAt)

    setSymmetricKeys(symKeys)
    setAsymmetricKeys(mappedAsymmetricKeys)

    return getRequiredChannelIds(symKeys, mappedAsymmetricKeys)
  }

  const refreshChannelDirectory = async (requiredChannelIds: string[]) => {
    const nextLabels: Record<string, string> = {
      ...channelLabelById,
      ...fallbackChannelLabelById,
    }
    const unresolved = new Set(requiredChannelIds.filter((channelId) => !nextLabels[channelId]))

    if (unresolved.size === 0) {
      setChannelLabelById(nextLabels)
      return
    }

    const serversResult = await serversList()
    if (!serversResult.success) {
      setChannelLabelById(nextLabels)
      return
    }

    for (const server of serversResult.data) {
      const channelsResult = await channelsList(server.serverId)
      if (!channelsResult.success) {
        continue
      }

      for (const channel of channelsResult.data) {
        if (!unresolved.has(channel.channelId)) {
          continue
        }
        nextLabels[channel.channelId] = `${server.name} · #${channel.name}`
        unresolved.delete(channel.channelId)
      }

      if (unresolved.size === 0) {
        break
      }
    }

    setChannelLabelById(nextLabels)
  }

  const refreshStateAndDirectory = async () => {
    const requiredChannelIds = await refreshState()
    await refreshChannelDirectory(requiredChannelIds)
  }

  useEffect(() => {
    if (!isActive) {
      return
    }

    setError('')
    setSuccess('')
    setSymImportText('')
    setAsymImportText('')
    setExportedSymmetricBundle('')
    setExportedAsymmetricBundle('')
    void refreshStateAndDirectory()
  }, [isActive, fallbackChannelLabelById])

  const handleExportSymmetric = async () => {
    setIsBusy(true)
    setError('')
    setSuccess('')
    try {
      setExportedSymmetricBundle(await exportSymmetricChannelKeys())
      setSuccess('Exportació de claus simètriques preparada')
    } catch {
      setError('No s\'han pogut exportar les claus simètriques')
    } finally {
      setIsBusy(false)
    }
  }

  const handleExportAsymmetric = async () => {
    setIsBusy(true)
    setError('')
    setSuccess('')
    try {
      setExportedAsymmetricBundle(await exportAsymmetricChannelKeys())
      setSuccess('Exportació de claus asimètriques preparada')
    } catch {
      setError('No s\'han pogut exportar les claus asimètriques')
    } finally {
      setIsBusy(false)
    }
  }

  const handleDeleteSymmetric = async (channelId: string) => {
    setIsBusy(true)
    setError('')
    setSuccess('')
    try {
      await deleteSymmetricChannelKey(channelId)
      await refreshStateAndDirectory()
      setSuccess(`Clau simètrica de ${formatChannelLabel(channelId)} eliminada`)
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
    setError('')
    setSuccess('')
    try {
      const imported = await importSymmetricChannelKeys(symImportText)
      await refreshStateAndDirectory()
      setSuccess(`Importades ${imported} claus simètriques de canals`)
      setSymImportText('')
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'No s\'han pogut importar les claus'
      setError(msg)
    } finally {
      setIsBusy(false)
    }
  }

  const handleImportAsymmetric = async () => {
    if (!asymImportText.trim()) {
      setError('Enganxa el JSON de claus asimètriques')
      return
    }

    setIsBusy(true)
    setError('')
    setSuccess('')
    try {
      const imported = await importAsymmetricChannelKeys(asymImportText)
      await refreshStateAndDirectory()
      setSuccess(`Importades ${imported} claus asimètriques de canals`)
      setAsymImportText('')
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'No s\'han pogut importar les claus'
      setError(msg)
    } finally {
      setIsBusy(false)
    }
  }

  return (
    <div className="device-keys-modal">
      {error && <div className="modal-error">{error}</div>}
      {success && <div className="modal-success">{success}</div>}

        <section className="device-keys-section">
          <h4>Claus simètriques</h4>
          <p>Claus simètriques guardades localment: <strong>{symmetricKeys.length}</strong></p>
          <div className="modal-form-actions">
            <Button variant="secondary" size="sm" onClick={handleExportSymmetric} disabled={isBusy}>
              Exportar simètriques
            </Button>
          </div>
          {symmetricKeys.length > 0 ? (
            <ul className="device-keys-list">
              {symmetricKeys.map((key) => (
                <li key={`${key.channelId}-${key.keyVersion}`} className="device-keys-list-item">
                  <div className="device-keys-list-main">
                    <strong title={key.channelId}>Canal {formatChannelLabel(key.channelId)} · v{key.keyVersion}</strong>
                    <span>Clau: {key.preview}</span>
                    <span>Guardada: {new Date(key.acquiredAt).toLocaleString()}</span>
                  </div>
                  <div className="device-keys-list-actions">
                    <Button variant="danger" size="sm" onClick={() => void handleDeleteSymmetric(key.channelId)} disabled={isBusy}>
                      Esborrar
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <p>No hi ha claus simètriques guardades localment.</p>
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

        <section className="device-keys-section">
          <h4>Bundles asimètrics</h4>
          <p>Bundles asimètrics guardats localment: <strong>{asymmetricKeys.length}</strong></p>
          <div className="modal-form-actions">
            <Button variant="secondary" size="sm" onClick={handleExportAsymmetric} disabled={isBusy}>
              Exportar asimètriques
            </Button>
          </div>
          {asymmetricKeys.length > 0 ? (
            <ul className="device-keys-list">
              {asymmetricKeys.map((key) => (
                <li key={`${key.channelId}-${key.keyVersion}`} className="device-keys-list-item">
                  <div className="device-keys-list-main">
                    <strong title={key.channelId}>Canal {formatChannelLabel(key.channelId)} · v{key.keyVersion}</strong>
                    <span>KeyVersionId: {key.keyVersionId ?? 'sense id'}</span>
                    <span>Guardat: {new Date(key.acquiredAt).toLocaleString()}</span>
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <p>No hi ha bundles asimètrics guardats localment.</p>
          )}
          <textarea
            className="device-keys-textarea"
            value={asymImportText}
            onChange={(e) => setAsymImportText(e.target.value)}
            placeholder="Enganxa aquí JSON de bundles asimètrics"
            rows={5}
            disabled={isBusy}
          />
          <div className="modal-form-actions">
            <Button variant="secondary" size="sm" onClick={handleImportAsymmetric} disabled={isBusy}>
              Importar asimètriques
            </Button>
          </div>
        </section>

        {exportedSymmetricBundle && (
          <section className="device-keys-section">
            <h4>Backup de claus simètriques (JSON)</h4>
            <textarea className="device-keys-textarea" value={exportedSymmetricBundle} readOnly rows={6} />
          </section>
        )}

        {exportedAsymmetricBundle && (
          <section className="device-keys-section">
            <h4>Backup de claus asimètriques (JSON)</h4>
            <textarea className="device-keys-textarea" value={exportedAsymmetricBundle} readOnly rows={6} />
          </section>
        )}
    </div>
  )
}

export function ChannelKeysPanel({ channels = [], serverName }: ChannelKeysPanelProps) {
  return <ChannelKeysContent isActive={true} channels={channels} serverName={serverName} />
}