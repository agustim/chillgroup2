import React, { useEffect, useMemo, useState } from 'react'

import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'
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

interface ChannelKeysModalProps {
  isOpen: boolean
  onClose: () => void
  channels?: Channel[]
}

export function ChannelKeysModal({ isOpen, onClose, channels = [] }: ChannelKeysModalProps) {
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

  const channelNameById = useMemo(
    () => new Map(channels.map((channel) => [channel.channelId, channel.name])),
    [channels]
  )

  const formatChannelLabel = (channelId: string) => {
    const name = channelNameById.get(channelId)
    return name ? `${name} · ${channelId}` : channelId
  }

  const refreshState = async () => {
    const [symKeys, channelKeys] = await Promise.all([listSymmetricChannelKeys(), listChannelKeys()])
    setSymmetricKeys(symKeys)
    setAsymmetricKeys(
      channelKeys
        .filter((entry) => entry.type === 'asymmetric')
        .map((entry) => ({
          channelId: entry.channelId,
          keyVersion: entry.keyVersion,
          keyVersionId: entry.keyVersionId ?? null,
          acquiredAt: entry.acquiredAt,
        }))
        .sort((a, b) => b.acquiredAt - a.acquiredAt)
    )
  }

  useEffect(() => {
    if (!isOpen) {
      return
    }

    setError('')
    setSuccess('')
    setSymImportText('')
    setAsymImportText('')
    setExportedSymmetricBundle('')
    setExportedAsymmetricBundle('')
    void refreshState()
  }, [isOpen])

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
    setError('')
    setSuccess('')
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
      await refreshState()
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
    <Modal isOpen={isOpen} onClose={onClose} title="Gestió de claus de canals">
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
                    <strong>Canal {formatChannelLabel(key.channelId)} · v{key.keyVersion}</strong>
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
                    <strong>Canal {formatChannelLabel(key.channelId)} · v{key.keyVersion}</strong>
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
    </Modal>
  )
}