import React, { useState, useCallback, useEffect } from 'react'
import { Channel, Message, VoiceConnection } from '../../types'
import { messagesSend } from '../../lib/api'
import { encryptChannelMessage, ensureChannelKey, distributeChannelKey } from '../../lib/channel-crypto'
import { generateThumbnail, uploadEncryptedAttachment } from '../../lib/attachments'
import { MessageList } from './MessageList'
import { VoiceArea } from './VoiceArea'
import { MessageInput } from './MessageInput'
import { ChannelHeader } from './ChannelHeader'
import { useSocketIO } from '../../hooks/useSocketIO'
import { logger } from '../../lib/logger'

interface ComposerAttachment {
  id: string
  file: File
}

interface MainContentProps {
  channel: Channel | null
  voiceConnection: VoiceConnection | null
  currentDeviceId?: string | null
  onLeaveVoice?: () => void
  voiceAsTextMode?: boolean
  onToggleVoiceAsTextMode?: () => void
  onUnreadUpdated?: (channelId: string, unreadCount: number) => void
  localVideoTrack?: any
  localScreenTrack?: any
  remoteVideoTracks?: Record<string, any[]>
  onRepairKey?: (channel: Channel) => Promise<void>
  onRotateKey?: (channel: Channel) => Promise<void>
  onUpdateDmTTL?: (channel: Channel, ttl: number | null) => Promise<void>
  keyActionBusy?: boolean
  isChannelAdmin?: boolean
}

export function MainContent({
  channel,
  voiceConnection,
  currentDeviceId,
  onLeaveVoice,
  voiceAsTextMode = false,
  onToggleVoiceAsTextMode,
  onUnreadUpdated,
  localVideoTrack,
  localScreenTrack,
  remoteVideoTracks = {},
  onRepairKey,
  onRotateKey,
  onUpdateDmTTL,
  keyActionBusy = false,
  isChannelAdmin = false,
}: MainContentProps) {
  const [message, setMessage] = useState('')
  const [refreshKey, setRefreshKey] = useState(0)
  const [sending, setSending] = useState(false)
  const [sendError, setSendError] = useState<string | null>(null)
  const [uploadingAttachmentNames, setUploadingAttachmentNames] = useState<string[]>([])
  const [pendingAttachments, setPendingAttachments] = useState<ComposerAttachment[]>([])
  const [socketMessages, setSocketMessages] = useState<Message[]>([])
  const [expiringMessageIds, setExpiringMessageIds] = useState<Set<string>>(new Set())
  const [focusTrigger, setFocusTrigger] = useState(0)

  useEffect(() => {
    setFocusTrigger((n) => n + 1)
  }, [channel?.channelId])

  const handleSocketMessage = useCallback((msg: Message) => {
    setSocketMessages((prev) => {
      // Evitar duplicats
      if (prev.some((m) => m.messageId === msg.messageId)) return prev
      return [...prev, msg]
    })
  }, [])

  const handleMessagesExpired = useCallback((_channelId: string, messageIds: string[]) => {
    setExpiringMessageIds((prev) => new Set([...prev, ...messageIds]))
    // Passats 600ms (durada de l'animació), retirem els missatges de l'estat
    setTimeout(() => {
      setSocketMessages((prev) => prev.filter((m) => !messageIds.includes(m.messageId)))
      setExpiringMessageIds((prev) => {
        const next = new Set(prev)
        messageIds.forEach((id) => next.delete(id))
        return next
      })
    }, 600)
  }, [])

  useSocketIO({
    channelId: channel?.type === 'text' ? (channel?.channelId ?? null) : null,
    onMessage: handleSocketMessage,
    onUnreadUpdated,
    onMessagesExpired: handleMessagesExpired,
  })

  // Netejar missatges de socket quan canviem de canal
  const channelId = channel?.channelId
  React.useEffect(() => {
    setSocketMessages([])
  }, [channelId])

  // Quan obrim un canal encriptat, intentar obtenir la clau del servidor si no la tenim
  React.useEffect(() => {
    if (!channel || channel.encryptionType === 'none' || !currentDeviceId) return

    let cancelled = false

    ensureChannelKey(channel.channelId, channel.encryptionType, currentDeviceId)
      .then(async (channelKey) => {
        if (cancelled || !channelKey || channel.encryptionType !== 'asymmetric') return

        const { getLatestChannelKey } = await import('../../lib/storage')
        const latest = await getLatestChannelKey(channel.channelId)
        const keyVersion = latest?.keyVersion ?? channel.keyVersion ?? 1
        const keyVersionId = latest?.keyVersionId ?? channel.keyVersionId ?? null

        // En canals asimètrics cal keyVersionId per signar bundles.
        if (!keyVersionId) {
          logger.warn('[E2EE] Redistribució automàtica omesa: falta keyVersionId', {
            channelId: channel.channelId,
            currentDeviceId,
          })
          return
        }

        // Si tenim la clau local, tornem a distribuir-la als dispositius membres.
        // Això cobreix membres/dispositius nous que encara no tenien bundle.
        distributeChannelKey(
          channel.channelId,
          channelKey,
          keyVersion,
          keyVersionId,
          currentDeviceId,
        ).catch((err) => {
          const msg = err instanceof Error ? err.message : 'Error desconegut redistribuint clau'
          logger.error('[E2EE] Redistribució automàtica de clau fallida en obrir canal', {
            channelId: channel.channelId,
            currentDeviceId,
            error: msg,
          })
        })
      })
      .catch((err) => {
        const msg = err instanceof Error ? err.message : 'Error desconegut obtenint clau de canal'
        logger.error('[E2EE] Error obtenint clau de canal en obrir-lo', {
          channelId: channel.channelId,
          currentDeviceId,
          error: msg,
        })
      })

    return () => {
      cancelled = true
    }
  }, [channel?.channelId, channel?.encryptionType, channel?.keyVersion, channel?.keyVersionId, currentDeviceId])

  const handleAddAttachments = useCallback((files: FileList | null) => {
    if (!files || files.length === 0) return

    setPendingAttachments((previous) => {
      const existingKey = new Set(previous.map((item) => `${item.file.name}:${item.file.size}:${item.file.lastModified}`))
      const next = [...previous]

      Array.from(files).forEach((file) => {
        const fingerprint = `${file.name}:${file.size}:${file.lastModified}`
        if (existingKey.has(fingerprint)) return
        next.push({
          id: crypto.randomUUID(),
          file,
        })
        existingKey.add(fingerprint)
      })

      return next
    })
  }, [])

  const handleRemoveAttachment = useCallback((attachmentId: string) => {
    setPendingAttachments((previous) => previous.filter((item) => item.id !== attachmentId))
  }, [])

  const handleSendMessage = async () => {
    const trimmedMessage = message.trim()
    if ((!trimmedMessage && pendingAttachments.length === 0) || sending || !channel || channel.type === 'voice') {
      return
    }

    setSending(true)
    setSendError(null)

    try {
      const { encryptedPayload, iv, keyVersion } = await encryptChannelMessage(
        channel.channelId,
        channel.encryptionType,
        trimmedMessage,
        currentDeviceId ?? undefined
      )

      const attachmentIds: string[] = []
      if (pendingAttachments.length > 0) {
        const { getLatestChannelKey } = await import('../../lib/storage')
        const latest = await getLatestChannelKey(channel.channelId)
        const keyVersionId = channel.keyVersionId ?? latest?.keyVersionId ?? crypto.randomUUID()
        const resolvedKeyVersion = channel.keyVersion ?? latest?.keyVersion ?? keyVersion ?? 1
        const channelKeyBytes = latest?.keyBytes ?? undefined

        for (const attachment of pendingAttachments) {
          setUploadingAttachmentNames([attachment.file.name])

          let thumbnailAttachmentId: string | undefined
          if (attachment.file.type.startsWith('image/')) {
            const thumbnailBlob = await generateThumbnail(attachment.file)
            if (thumbnailBlob) {
              const thumbnailFile = new File(
                [thumbnailBlob],
                `thumb_${attachment.file.name}`,
                { type: 'image/jpeg' },
              )
              const thumbUploaded = await uploadEncryptedAttachment({
                channelId: channel.channelId,
                file: thumbnailFile,
                keyVersionId,
                keyVersion: resolvedKeyVersion,
                channelKeyBytes,
              })
              thumbnailAttachmentId = thumbUploaded.attachmentId
            }
          }

          const uploaded = await uploadEncryptedAttachment({
            channelId: channel.channelId,
            file: attachment.file,
            keyVersionId,
            keyVersion: resolvedKeyVersion,
            thumbnailAttachmentId,
            channelKeyBytes,
          })
          attachmentIds.push(uploaded.attachmentId)
        }
      }

      const response = await messagesSend(
        channel.channelId,
        encryptedPayload,
        iv,
        keyVersion ?? undefined,
        undefined,
        channel.scope,
        attachmentIds,
      )
      if (response.success) {
        setMessage('')
        setPendingAttachments([])
        setRefreshKey((current) => current + 1)
        setFocusTrigger((n) => n + 1)
      } else {
        setSendError(response.error.message || "No s'ha pogut enviar el missatge")
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Error en enviar el missatge'
      setSendError(message)
    } finally {
      setUploadingAttachmentNames([])
      setSending(false)
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault()
      handleSendMessage()
    }
  }

  // Validar que el canal existeix
  if (!channel && !voiceConnection) {
    return (
      <div className="main-content">
        <div className="empty-state">
          <p>Selecciona un canal per començar</p>
        </div>
      </div>
    )
  }

  const isTextChannel = channel?.type === 'text'
  const isVoiceChannel = channel?.type === 'voice'
  const showVoicePanel = !!voiceConnection && (!voiceAsTextMode || isVoiceChannel || !channel)
  const handleRepairKey = async () => {
    if (!channel || !onRepairKey) return
    await onRepairKey(channel)
  }

  const handleRotateKey = async () => {
    if (!channel || !onRotateKey) return
    await onRotateKey(channel)
  }

  const handleUpdateTTL = async (ttl: number | null) => {
    if (!channel || !onUpdateDmTTL) return
    await onUpdateDmTTL(channel, ttl)
  }

  // Layout: voice-panel a l'esquerra (si connectat) + text-area a la dreta (si canal de text)
  return (
    <div className={`main-content ${showVoicePanel ? 'voice-active-layout' : ''}`}>
      {/* Panell de veu (sempre visible si connectat, independentment del canal de text) */}
      {showVoicePanel && (
        <div className="voice-panel">
          <VoiceArea
            connection={voiceConnection}
            onLeave={onLeaveVoice}
            voiceAsTextMode={voiceAsTextMode}
            onToggleVoiceAsTextMode={onToggleVoiceAsTextMode}
            localVideoTrack={localVideoTrack}
            localScreenTrack={localScreenTrack}
            remoteVideoTracks={remoteVideoTracks}
          />
        </div>
      )}

      {/* Àrea de text: channel-header + missatges + input */}
      {isTextChannel && channel && (
        <div className="text-area">
          <ChannelHeader
            channel={channel}
            onRepairKey={handleRepairKey}
            onRotateKey={handleRotateKey}
            onUpdateTTL={channel.scope === 'dm' && onUpdateDmTTL ? handleUpdateTTL : undefined}
            keyActionBusy={keyActionBusy}
            isChannelAdmin={isChannelAdmin}
          />
          <div className="text-panel">
            <MessageList
              channelId={channel.channelId}
              scope={channel.scope}
              encryptionType={channel.encryptionType}
              refreshKey={refreshKey}
              socketMessages={socketMessages}
              expiringMessageIds={expiringMessageIds}
              unreadCount={channel.unreadCount ?? 0}
              lastReadMessageId={channel.lastReadMessageId}
            />
            {sendError && <div className="message-send-error">{sendError}</div>}
            <MessageInput
              value={message}
              onChange={setMessage}
              onKeyDown={handleKeyDown}
              onSubmit={handleSendMessage}
              onAddAttachments={handleAddAttachments}
              onRemoveAttachment={handleRemoveAttachment}
              pendingAttachments={pendingAttachments.map((item) => ({
                id: item.id,
                name: item.file.name,
                size: item.file.size,
              }))}
              placeholder={`Missatjar a #${channel.name}`}
              encryptionType={channel.encryptionType}
              isBusy={sending}
              focusKey={String(focusTrigger)}
            />
            {uploadingAttachmentNames.length > 0 && (
              <div className="message-send-info">
                Pujant adjunts: {uploadingAttachmentNames.join(', ')}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Si en veu però sense canal de text seleccionat */}
      {voiceConnection && !isTextChannel && !voiceAsTextMode && (
        <div className="text-area empty-text-area">
          <div className="empty-state">
            <p>Selecciona un canal de text per xatejar</p>
          </div>
        </div>
      )}
    </div>
  )
}
