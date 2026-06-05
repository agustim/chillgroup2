import React, { useState, useEffect, useRef, useCallback } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkBreaks from 'remark-breaks'
import remarkGfm from 'remark-gfm'
import { EncryptionType, Message } from '../../types'
import { attachmentGetDownload, channelsMarkRead, messagesList } from '../../lib/api'
import { decryptAttachmentToBlob, downloadAndDecryptAttachment } from '../../lib/attachments'
import { decryptMessagesForChannel } from '../../lib/channel-crypto'
import { logger } from '../../lib/logger'

interface AttachmentView {
  attachmentId: string
  fileName: string
  downloadUrl: string
  sizeBytes: number
  mimeType: string
  crypto: {
    wrappedFileKey: string
    fileIv: string
  }
  thumbnailAttachmentId?: string
  channelKeyBytes?: Uint8Array
}

interface MessageListProps {
  channelId: string
  scope?: 'server' | 'dm'
  encryptionType: EncryptionType
  refreshKey?: number
  socketMessages?: Message[]
  expiringMessageIds?: Set<string>
  unreadCount?: number
  lastReadMessageId?: string | null
}

export function MessageList({
  channelId,
  scope,
  encryptionType,
  refreshKey,
  socketMessages = [],
  expiringMessageIds,
  unreadCount = 0,
  lastReadMessageId,
}: MessageListProps) {
  const [messages, setMessages] = useState<Message[]>([])
  const [decryptedPayloads, setDecryptedPayloads] = useState<Record<string, string>>({})
  const [attachmentById, setAttachmentById] = useState<Record<string, AttachmentView>>({})
  const [thumbnailBlobUrls, setThumbnailBlobUrls] = useState<Record<string, string>>({})
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [hasPrevPage, setHasPrevPage] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [unreadDividerMessageId, setUnreadDividerMessageId] = useState<string | null>(null)

  const messagesEndRef = useRef<HTMLDivElement>(null)
  const messagesTopRef = useRef<HTMLDivElement>(null)
  const unreadDividerRef = useRef<HTMLDivElement>(null)
  const expiringMessageIdsRef = useRef<Set<string> | undefined>(expiringMessageIds)
  const atBottomRef = useRef(true)
  const isEncryptedChannel = encryptionType !== 'none'

  const renderMarkdownMessage = (text: string) => (
    <div className="message-markdown">
      <ReactMarkdown
        remarkPlugins={[remarkGfm, remarkBreaks]}
        skipHtml
        components={{
          p: ({ children }) => <p className="message-markdown-paragraph">{children}</p>,
          a: ({ href, children }) => (
            <a href={href} target="_blank" rel="noreferrer noopener" className="message-markdown-link">
              {children}
            </a>
          ),
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  )

  const formatSize = (sizeBytes: number) => {
    if (sizeBytes < 1024) return `${sizeBytes} B`
    if (sizeBytes < 1024 * 1024) return `${(sizeBytes / 1024).toFixed(1)} KB`
    return `${(sizeBytes / (1024 * 1024)).toFixed(1)} MB`
  }

  const handleAttachmentDownload = async (attachment: AttachmentView) => {
    try {
      await downloadAndDecryptAttachment({
        fileName: attachment.fileName,
        mimeType: attachment.mimeType,
        downloadUrl: attachment.downloadUrl,
        crypto: {
          wrappedFileKey: attachment.crypto.wrappedFileKey,
          fileIv: attachment.crypto.fileIv,
        },
        channelKeyBytes: attachment.channelKeyBytes,
      })
    } catch (error) {
      logger.error('[MessageList] Error descarregant adjunt:', error)
    }
  }

  useEffect(() => {
    expiringMessageIdsRef.current = expiringMessageIds
  }, [expiringMessageIds])

  const filterExpiring = (msgs: Message[]) =>
    expiringMessageIdsRef.current && expiringMessageIdsRef.current.size > 0
      ? msgs.filter((m) => !expiringMessageIdsRef.current!.has(m.messageId))
      : msgs

  // El backend retorna DESC (mes nou primer). Invertim per mostrar ASC (mes antic dalt).
  // No usem sort() per timestamps perquè missatges enviats ràpidament poden tenir el mateix timestamp.
  const fromDesc = (msgs: Message[]) => [...msgs].reverse()

  const loadMessages = async () => {
    if (!channelId) {
      setError('Canal no seleccionat')
      setLoading(false)
      return
    }

    try {
      setLoading(true)
      setError(null)
      setUnreadDividerMessageId(null)
      atBottomRef.current = true

      const hasUnread = unreadCount > 0 && !!lastReadMessageId

      if (hasUnread) {
        // Load unread messages (after lastReadMessageId)
        const unreadResult = await messagesList(channelId, 50, undefined, scope, lastReadMessageId!)
        if (!unreadResult.success || !unreadResult.data) {
          setError('No es poden carregar els missatges')
          return
        }
        const unreadMsgs = filterExpiring(unreadResult.data.data)

        let contextMsgs: Message[] = []
        let prevPage = false
        if (unreadMsgs.length > 0) {
          // Load 5 context messages just before the first unread (oldest unread = index 0 per ASC)
          const ctxResult = await messagesList(channelId, 5, unreadMsgs[0].messageId, scope)
          if (ctxResult.success && ctxResult.data) {
            contextMsgs = filterExpiring(ctxResult.data.data)
            prevPage = contextMsgs.length >= 5
          }
          setUnreadDividerMessageId(unreadMsgs[0].messageId)
          atBottomRef.current = false
        } else {
          prevPage = unreadResult.data.data.length >= 50
        }

        // contextMsgs ve en DESC (before), unreadMsgs ve en ASC (after). Invertim context i mergem.
        const merged = [...fromDesc(contextMsgs), ...unreadMsgs]
        setMessages(merged)
        setHasPrevPage(prevPage)
        const decrypted = await decryptMessagesForChannel(channelId, encryptionType, merged)
        setDecryptedPayloads(decrypted)
      } else {
        // Standard: load latest 50 (backend retorna DESC, ordenem ASC per emmagatzemar)
        const result = await messagesList(channelId, 50, undefined, scope)
        if (result.success && result.data) {
          const loaded = fromDesc(filterExpiring(result.data.data))
          setMessages(loaded)
          setHasPrevPage(loaded.length >= 50)
          const decrypted = await decryptMessagesForChannel(channelId, encryptionType, loaded)
          setDecryptedPayloads(decrypted)
        } else {
          setError('No es poden carregar els missatges')
        }
      }
    } catch {
      setError('Error de connexió')
    } finally {
      setLoading(false)
    }
  }

  const loadMoreMessages = useCallback(async () => {
    if (loadingMore || !hasPrevPage || messages.length === 0) return
    // messages[0] és sempre el més antic (estat guardat en ASC)
    const oldestId = messages[0].messageId
    setLoadingMore(true)
    try {
      const result = await messagesList(channelId, 50, oldestId, scope)
      if (result.success && result.data && result.data.data.length > 0) {
        const older = filterExpiring(result.data.data)
        setMessages((prev) => {
          const existingIds = new Set(prev.map((m) => m.messageId))
          // older ve en DESC del backend → invertim. Filtrem duplicats de older (no de prev).
          return [...fromDesc(older).filter((m) => !existingIds.has(m.messageId)), ...prev]
        })
        setHasPrevPage(result.data.data.length >= 50)
      } else {
        setHasPrevPage(false)
      }
    } catch {
      // silent fail
    } finally {
      setLoadingMore(false)
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [channelId, scope, loadingMore, hasPrevPage, messages])

  useEffect(() => {
    loadMessages()
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [channelId, scope, encryptionType, refreshKey])

  useEffect(() => {
    setAttachmentById({})
  }, [channelId])

  // Quan arriben missatges expirats, esperem la durada de l'animació i els retirem
  useEffect(() => {
    if (!expiringMessageIds || expiringMessageIds.size === 0) return
    const timer = setTimeout(() => {
      setMessages((prev) => prev.filter((m) => !expiringMessageIds.has(m.messageId)))
    }, 600)
    return () => clearTimeout(timer)
  }, [expiringMessageIds])

  useEffect(() => {
    if (!expiringMessageIds || expiringMessageIds.size === 0) return
    setMessages((prev) => prev.filter((m) => !expiringMessageIds.has(m.messageId)))
  }, [expiringMessageIds])

  // Combinar missatges carregats (ja en ordre ASC) + nous via socket (sempre al final)
  // No fem sort per timestamp: messages[] ja té l'ordre correcte establert manualment.
  // socketMessages s'afegeixen al final perquè sempre son els més nous.
  const loadedIds = new Set(messages.map((m) => m.messageId))
  const combined = [
    ...messages,
    ...socketMessages.filter((m) => !loadedIds.has(m.messageId)),
  ].filter((m) => !expiringMessageIds?.has(m.messageId))

  // Desxifrar missatges combinats en temps real
  useEffect(() => {
    if (combined.length === 0) return
    let cancelled = false
    decryptMessagesForChannel(channelId, encryptionType, combined)
      .then((decrypted) => { if (!cancelled) setDecryptedPayloads(decrypted) })
      .catch(() => {})
    return () => { cancelled = true }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [channelId, scope, encryptionType, messages, socketMessages, expiringMessageIds])

  // Scroll inicial: al divisor de no llegits o al final
  useEffect(() => {
    if (loading) return
    if (unreadDividerRef.current) {
      unreadDividerRef.current.scrollIntoView({ behavior: 'instant' as ScrollBehavior })
    } else {
      messagesEndRef.current?.scrollIntoView({ behavior: 'instant' as ScrollBehavior })
    }
  // Només en canvi de canal o fi de càrrega inicial
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loading, channelId])

  // Scroll suau al final quan arriben missatges de socket (si ja som al final)
  useEffect(() => {
    if (socketMessages.length > 0 && atBottomRef.current) {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
    }
  }, [socketMessages])

  // IntersectionObserver per carregar missatges antics (scroll cap amunt)
  useEffect(() => {
    const el = messagesTopRef.current
    if (!el) return
    const observer = new IntersectionObserver(
      ([entry]) => { if (entry.isIntersecting) void loadMoreMessages() },
      { threshold: 0.1 },
    )
    observer.observe(el)
    return () => observer.disconnect()
  }, [loadMoreMessages])

  // IntersectionObserver per marcar com llegit quan s'arriba al final
  useEffect(() => {
    const el = messagesEndRef.current
    if (!el) return
    const observer = new IntersectionObserver(
      ([entry]) => {
        atBottomRef.current = entry.isIntersecting
        if (entry.isIntersecting) {
          const lastMsg = combined[combined.length - 1]
          if (lastMsg) {
            channelsMarkRead(channelId, lastMsg.messageId).catch(() => {})
            setUnreadDividerMessageId(null)
          }
        }
      },
      { threshold: 0.5 },
    )
    observer.observe(el)
    return () => observer.disconnect()
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [channelId, combined])

  // Adjunts
  useEffect(() => {
    const attachmentIds = Array.from(
      new Set(combined.flatMap((msg) => msg.attachmentIds ?? [])),
    )
    const missingIds = attachmentIds.filter((id) => !attachmentById[id])
    if (missingIds.length === 0) return
    let cancelled = false
    Promise.all(
      missingIds.map(async (attachmentId) => {
        const response = await attachmentGetDownload(channelId, attachmentId)
        if (!response.success) return null
        const { getChannelKeyVersion } = await import('../../lib/storage')
        const channelKeyBytes = response.data.crypto.keyVersion
          ? (await getChannelKeyVersion(channelId, response.data.crypto.keyVersion)) ?? undefined
          : undefined
        return {
          attachmentId,
          fileName: response.data.fileName,
          downloadUrl: response.data.downloadUrl,
          sizeBytes: response.data.sizeBytes,
          mimeType: response.data.mimeType,
          crypto: {
            wrappedFileKey: response.data.crypto.wrappedFileKey,
            fileIv: response.data.crypto.fileIv,
          },
          thumbnailAttachmentId: response.data.thumbnail_attachment_id,
          channelKeyBytes,
        } as AttachmentView
      }),
    )
      .then((items) => {
        if (cancelled) return
        const valid = items.filter((item): item is AttachmentView => item !== null)
        if (valid.length === 0) return
        setAttachmentById((prev) => {
          const next = { ...prev }
          valid.forEach((item) => { next[item.attachmentId] = item })
          return next
        })
      })
      .catch(() => {})
    return () => { cancelled = true }
  }, [attachmentById, channelId, combined])

  // Carregar thumbnails desencriptats per imatges
  useEffect(() => {
    const pending = Object.values(attachmentById).filter(
      (a) => a.thumbnailAttachmentId && !thumbnailBlobUrls[a.attachmentId],
    )
    if (pending.length === 0) return
    let cancelled = false
    Promise.all(
      pending.map(async (attachment) => {
        const thumbId = attachment.thumbnailAttachmentId!
        const response = await attachmentGetDownload(channelId, thumbId)
        if (!response.success) {
          logger.error('[thumb] download info failed', { thumbId, error: response.error })
          return null
        }
        try {
          const blob = await decryptAttachmentToBlob({
            fileName: response.data.fileName,
            downloadUrl: response.data.downloadUrl,
            mimeType: response.data.mimeType,
            crypto: {
              wrappedFileKey: response.data.crypto.wrappedFileKey,
              fileIv: response.data.crypto.fileIv,
            },
            channelKeyBytes: attachment.channelKeyBytes,
          })
          return { attachmentId: attachment.attachmentId, blobUrl: URL.createObjectURL(blob) }
        } catch (err) {
          logger.error('[thumb] decrypt failed', { thumbId, err })
          return null
        }
      }),
    )
      .then((results) => {
        if (cancelled) return
        const valid = results.filter(
          (r): r is { attachmentId: string; blobUrl: string } => r !== null,
        )
        if (valid.length === 0) return
        setThumbnailBlobUrls((prev) => {
          const next = { ...prev }
          valid.forEach(({ attachmentId, blobUrl }) => { next[attachmentId] = blobUrl })
          return next
        })
      })
      .catch((err) => { logger.error('[thumb] effect error', err) })
    return () => { cancelled = true }
  }, [attachmentById, channelId, thumbnailBlobUrls])

  // Revocar blob URLs en canviar de canal
  useEffect(() => {
    return () => {
      setThumbnailBlobUrls((prev) => {
        Object.values(prev).forEach(URL.revokeObjectURL)
        return {}
      })
    }
  }, [channelId])

  // Early returns sempre després de tots els hooks
  if (loading) {
    return (
      <div className="message-list loading">
        <p>Carregant missatges...</p>
      </div>
    )
  }

  if (error) {
    return (
      <div className="message-list error">
        <p>{error}</p>
        <button onClick={loadMessages}>Reintentar</button>
      </div>
    )
  }

  if (combined.length === 0) {
    return (
      <div className="message-list empty">
        <p>Sense missatges encara</p>
        <p className="empty-hint">Sigues el primer a enviar missatge!</p>
      </div>
    )
  }

  return (
    <div className="message-list">
      {/* Sentinel per carregar missatges antics */}
      <div ref={messagesTopRef} className="messages-top-sentinel">
        {loadingMore && <p className="loading-more-indicator">Carregant més...</p>}
        {!loadingMore && hasPrevPage && <p className="load-more-hint">Fes scroll cap amunt per veure més</p>}
      </div>

      {combined.map((msg, index) => {
        const showDivider = unreadDividerMessageId === msg.messageId
        const showHeader =
          index === 0 || combined[index - 1].senderUserId !== msg.senderUserId || showDivider

        return (
          <React.Fragment key={msg.messageId}>
            {showDivider && (
              <div ref={unreadDividerRef} id="unread-divider" className="unread-divider">
                <span>Missatges nous</span>
              </div>
            )}
            <div
              className={`message-bubble ${msg.deletedAt ? 'deleted' : ''} ${msg.editedAt ? 'edited' : ''} ${showHeader ? 'first-in-row' : ''} ${expiringMessageIds?.has(msg.messageId) ? 'expiring' : ''}`}
            >
              {showHeader && (
                <div className="message-sender">
                  <span className="sender-avatar">
                    {msg.senderUsername.charAt(0).toUpperCase()}
                  </span>
                  <span className="sender-name">{msg.senderUsername}</span>
                  {msg.senderDeviceId && (
                    <span className="device-badge" title="Dispositiu">
                      💻
                    </span>
                  )}
                </div>
              )}
              <div className="message-content">
                {msg.deletedAt ? (
                  <p className="deleted-message">Missatge eliminat</p>
                ) : (
                  renderMarkdownMessage(decryptedPayloads[msg.messageId] ?? msg.encryptedPayload)
                )}

                {(msg.attachmentIds?.length ?? 0) > 0 && (
                  <div className="message-attachment-list">
                    {msg.attachmentIds?.map((attachmentId) => {
                      const attachment = attachmentById[attachmentId]
                      if (!attachment) {
                        return (
                          <span key={attachmentId} className="message-attachment-item loading">
                            Adjunt carregant...
                          </span>
                        )
                      }
                      const thumbUrl = thumbnailBlobUrls[attachmentId]
                      return (
                        <div key={attachmentId} className={thumbUrl ? 'message-attachment-image' : 'message-attachment-file'}>
                          {thumbUrl && (
                            <img
                              src={thumbUrl}
                              alt={attachment.fileName}
                              className="message-attachment-thumbnail"
                              title={attachment.fileName}
                            />
                          )}
                          <a
                            href={attachment.downloadUrl}
                            className="message-attachment-item"
                            download={attachment.fileName}
                            title={attachment.fileName}
                            onClick={(event) => {
                              event.preventDefault()
                              void handleAttachmentDownload(attachment)
                            }}
                          >
                            📎 {attachment.fileName} ({formatSize(attachment.sizeBytes)})
                          </a>
                        </div>
                      )
                    })}
                  </div>
                )}
              </div>
              <div className="message-timestamp">
                {new Date(msg.timestamp).toLocaleTimeString('ca-ES', {
                  hour: '2-digit',
                  minute: '2-digit',
                })}
                {msg.editedAt && <span className="edited-label">(editat)</span>}
                {msg.expiresAt && (
                  <span className="expires-label" title={msg.expiresAt}>
                    ⏰
                  </span>
                )}
                {isEncryptedChannel && msg.keyVersion != null && (
                  <span className="key-version-label" title={`Versió de clau: ${msg.keyVersion}`}>
                    🔐v{msg.keyVersion}
                  </span>
                )}
              </div>
            </div>
          </React.Fragment>
        )
      })}

      {/* Sentinel per detectar que l'usuari és al final (mark-as-read) */}
      <div ref={messagesEndRef} />
    </div>
  )
}
