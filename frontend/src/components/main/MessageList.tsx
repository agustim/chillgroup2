import React, { useState, useEffect, useRef } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkBreaks from 'remark-breaks'
import remarkGfm from 'remark-gfm'
import { EncryptionType, Message } from '../../types'
import { attachmentGetDownload, messagesList } from '../../lib/api'
import { downloadAndDecryptAttachment } from '../../lib/attachments'
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
}

interface MessageListProps {
  channelId: string
  scope?: 'server' | 'dm'
  encryptionType: EncryptionType
  refreshKey?: number
  socketMessages?: Message[]
  expiringMessageIds?: Set<string>
}

export function MessageList({
  channelId,
  scope,
  encryptionType,
  refreshKey,
  socketMessages = [],
  expiringMessageIds,
}: MessageListProps) {
  const [messages, setMessages] = useState<Message[]>([])
  const [decryptedPayloads, setDecryptedPayloads] = useState<Record<string, string>>({})
  const [attachmentById, setAttachmentById] = useState<Record<string, AttachmentView>>({})
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const expiringMessageIdsRef = useRef<Set<string> | undefined>(expiringMessageIds)
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
      })
    } catch (error) {
      logger.error('[MessageList] Error descarregant adjunt:', error)
    }
  }

  // Mantenir la ref actualitzada amb el valor actual de expiringMessageIds
  useEffect(() => {
    expiringMessageIdsRef.current = expiringMessageIds
  }, [expiringMessageIds])

  const loadMessages = async () => {
    // Debug: veure què arriba
    logger.debug('[MessageList] channelId prop:', channelId)
    // Validar que el channelId existeix
    if (!channelId) {
      setError('Canal no seleccionat')
      setLoading(false)
      return
    }

    try {
      setLoading(true)
      setError(null)
      const result = await messagesList(channelId, 50, undefined, scope)
      if (result.success && result.data) {
        // Filtrar missatges que estan a expiringMessageIds
        const filtered = expiringMessageIdsRef.current && expiringMessageIdsRef.current.size > 0
          ? result.data.data.filter((m) => !expiringMessageIdsRef.current!.has(m.messageId))
          : result.data.data
        setMessages(filtered)
        const decrypted = await decryptMessagesForChannel(channelId, encryptionType, filtered)
        setDecryptedPayloads(decrypted)
      } else {
        setError('No es poden carregar els missatges')
      }
    } catch {
      setError('Error de connexió')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    loadMessages()
  }, [channelId, scope, encryptionType, refreshKey])

  useEffect(() => {
    setAttachmentById({})
  }, [channelId])

  // Quan arriben missatges expirats, esperem la durada de l'animació i els retirem de l'estat local
  useEffect(() => {
    if (!expiringMessageIds || expiringMessageIds.size === 0) return
    const timer = setTimeout(() => {
      setMessages((prev) => prev.filter((m) => !expiringMessageIds.has(m.messageId)))
    }, 600)
    return () => clearTimeout(timer)
  }, [expiringMessageIds])

  // Quan expiringMessageIds es reactualitza, filtrar immediatament els missatges carregats
  useEffect(() => {
    if (!expiringMessageIds || expiringMessageIds.size === 0) return
    setMessages((prev) => prev.filter((m) => !expiringMessageIds.has(m.messageId)))
  }, [expiringMessageIds])

  // Combinar missatges carregats + missatges rebuts via socket (sense duplicats),
  // i filtrar els que estan a expiringMessageIds per evitar el "flash" després de l'animació.
  // Cal calcular-ho AQUÍ perquè el useEffect de desxifrat el necessita i tots els hooks
  // s'han d'invocar abans de qualsevol early return.
  const loadedIds = new Set(messages.map((m) => m.messageId))
  const combined = [
    ...messages,
    ...socketMessages.filter((m) => !loadedIds.has(m.messageId)),
  ]
    .filter((m) => !expiringMessageIds?.has(m.messageId))
    .sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime())

  // Desxifrar missatges combinats en temps real (socket + historial)
  useEffect(() => {
    if (combined.length === 0) {
      return
    }

    let cancelled = false
    decryptMessagesForChannel(channelId, encryptionType, combined)
      .then((decrypted) => {
        if (!cancelled) {
          setDecryptedPayloads(decrypted)
        }
      })
      .catch(() => {
        // Best effort: si falta clau, el missatge mostrarà el payload cru.
      })

    return () => {
      cancelled = true
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [channelId, scope, encryptionType, messages, socketMessages, expiringMessageIds])

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView?.({ behavior: 'smooth' })
  }, [combined])

  useEffect(() => {
    const attachmentIds = Array.from(
      new Set(
        combined.flatMap((msg) => msg.attachmentIds ?? []),
      ),
    )

    const missingIds = attachmentIds.filter((attachmentId) => !attachmentById[attachmentId])
    if (missingIds.length === 0) return

    let cancelled = false

    Promise.all(
      missingIds.map(async (attachmentId) => {
        const response = await attachmentGetDownload(channelId, attachmentId)
        if (!response.success) {
          return null
        }

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
        } satisfies AttachmentView
      }),
    )
      .then((items) => {
        if (cancelled) return
        const validItems = items.filter((item): item is AttachmentView => item !== null)
        if (validItems.length === 0) return

        setAttachmentById((previous) => {
          const next = { ...previous }
          validItems.forEach((item) => {
            next[item.attachmentId] = item
          })
          return next
        })
      })
      .catch(() => {
        // Ignore per-message attachment metadata failures.
      })

    return () => {
      cancelled = true
    }
  }, [attachmentById, channelId, combined])

  // Early returns SEMPRE després de tots els hooks
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
      {combined.map((msg, index) => {
        const showHeader =
          index === 0 || combined[index - 1].senderUserId !== msg.senderUserId

        return (
          <div
            key={msg.messageId}
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

                    return (
                      <a
                        key={attachmentId}
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
        )
      })}
      <div ref={messagesEndRef} />
    </div>
  )
}