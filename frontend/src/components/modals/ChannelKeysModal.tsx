import React, { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

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
import { useAsyncTask } from '../../hooks/useAsyncTask'
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
  const { t } = useTranslation()
  const { isBusy, error, success, run, setError, setSuccess } = useAsyncTask()
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
    return t('channelKeys.unknownChannel')
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

    setError(null)
    setSuccess(null)
    setSymImportText('')
    setAsymImportText('')
    setExportedSymmetricBundle('')
    setExportedAsymmetricBundle('')
    void refreshStateAndDirectory()
  }, [isActive, fallbackChannelLabelById])

  const handleExportSymmetric = () => void run(async () => {
    setExportedSymmetricBundle(await exportSymmetricChannelKeys())
    return t('channelKeys.successExportSym')
  }, t('channelKeys.errExportSym'))

  const handleExportAsymmetric = () => void run(async () => {
    setExportedAsymmetricBundle(await exportAsymmetricChannelKeys())
    return t('channelKeys.successExportAsym')
  }, t('channelKeys.errExportAsym'))

  const handleDeleteSymmetric = (channelId: string) => void run(async () => {
    await deleteSymmetricChannelKey(channelId)
    await refreshStateAndDirectory()
    return t('channelKeys.successDeleteSym', { label: formatChannelLabel(channelId) })
  }, t('channelKeys.errDeleteSym'))

  const handleImportSymmetric = () => {
    if (!symImportText.trim()) {
      setError(t('channelKeys.errPasteSym'))
      return
    }
    void run(async () => {
      const imported = await importSymmetricChannelKeys(symImportText)
      await refreshStateAndDirectory()
      setSymImportText('')
      return t('channelKeys.importedSym', { count: imported })
    })
  }

  const handleImportAsymmetric = () => {
    if (!asymImportText.trim()) {
      setError(t('channelKeys.errPasteAsym'))
      return
    }
    void run(async () => {
      const imported = await importAsymmetricChannelKeys(asymImportText)
      await refreshStateAndDirectory()
      setAsymImportText('')
      return t('channelKeys.importedAsym', { count: imported })
    })
  }

  return (
    <div className="device-keys-modal">
      {error && <div className="modal-error">{error}</div>}
      {success && <div className="modal-success">{success}</div>}

        <section className="device-keys-section">
          <h4>{t('channelKeys.symTitle')}</h4>
          <p>{t('channelKeys.symCount')} <strong>{symmetricKeys.length}</strong></p>
          <div className="modal-form-actions">
            <Button variant="secondary" size="sm" onClick={handleExportSymmetric} disabled={isBusy}>
              {t('channelKeys.exportSym')}
            </Button>
          </div>
          {symmetricKeys.length > 0 ? (
            <ul className="device-keys-list">
              {symmetricKeys.map((key) => (
                <li key={`${key.channelId}-${key.keyVersion}`} className="device-keys-list-item">
                  <div className="device-keys-list-main">
                    <strong title={key.channelId}>{t('channelKeys.channelVersion', { label: formatChannelLabel(key.channelId), version: key.keyVersion })}</strong>
                    <span>{t('channelKeys.keyLabel')} {key.preview}</span>
                    <span>{t('channelKeys.savedLabel')} {new Date(key.acquiredAt).toLocaleString()}</span>
                  </div>
                  <div className="device-keys-list-actions">
                    <Button variant="danger" size="sm" onClick={() => void handleDeleteSymmetric(key.channelId)} disabled={isBusy}>
                      {t('common.erase')}
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <p>{t('channelKeys.symEmpty')}</p>
          )}
          <textarea
            className="device-keys-textarea"
            value={symImportText}
            onChange={(e) => setSymImportText(e.target.value)}
            placeholder={t('channelKeys.symImportPlaceholder')}
            rows={5}
            disabled={isBusy}
          />
          <div className="modal-form-actions">
            <Button variant="secondary" size="sm" onClick={handleImportSymmetric} disabled={isBusy}>
              {t('channelKeys.importSym')}
            </Button>
          </div>
        </section>

        <section className="device-keys-section">
          <h4>{t('channelKeys.asymTitle')}</h4>
          <p>{t('channelKeys.asymCount')} <strong>{asymmetricKeys.length}</strong></p>
          <div className="modal-form-actions">
            <Button variant="secondary" size="sm" onClick={handleExportAsymmetric} disabled={isBusy}>
              {t('channelKeys.exportAsym')}
            </Button>
          </div>
          {asymmetricKeys.length > 0 ? (
            <ul className="device-keys-list">
              {asymmetricKeys.map((key) => (
                <li key={`${key.channelId}-${key.keyVersion}`} className="device-keys-list-item">
                  <div className="device-keys-list-main">
                    <strong title={key.channelId}>{t('channelKeys.channelVersion', { label: formatChannelLabel(key.channelId), version: key.keyVersion })}</strong>
                    <span>{t('channelKeys.keyVersionId')} {key.keyVersionId ?? t('channelKeys.noId')}</span>
                    <span>{t('channelKeys.savedNeuter')} {new Date(key.acquiredAt).toLocaleString()}</span>
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <p>{t('channelKeys.asymEmpty')}</p>
          )}
          <textarea
            className="device-keys-textarea"
            value={asymImportText}
            onChange={(e) => setAsymImportText(e.target.value)}
            placeholder={t('channelKeys.asymImportPlaceholder')}
            rows={5}
            disabled={isBusy}
          />
          <div className="modal-form-actions">
            <Button variant="secondary" size="sm" onClick={handleImportAsymmetric} disabled={isBusy}>
              {t('channelKeys.importAsym')}
            </Button>
          </div>
        </section>

        {exportedSymmetricBundle && (
          <section className="device-keys-section">
            <h4>{t('channelKeys.symBackupTitle')}</h4>
            <textarea className="device-keys-textarea" value={exportedSymmetricBundle} readOnly rows={6} />
          </section>
        )}

        {exportedAsymmetricBundle && (
          <section className="device-keys-section">
            <h4>{t('channelKeys.asymBackupTitle')}</h4>
            <textarea className="device-keys-textarea" value={exportedAsymmetricBundle} readOnly rows={6} />
          </section>
        )}
    </div>
  )
}

export function ChannelKeysPanel({ channels = [], serverName }: ChannelKeysPanelProps) {
  return <ChannelKeysContent isActive={true} channels={channels} serverName={serverName} />
}