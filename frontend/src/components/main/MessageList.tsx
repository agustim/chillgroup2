import React, { useState, useEffect, useRef, useCallback } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkBreaks from 'remark-breaks'
import remarkGfm from 'remark-gfm'
import { EncryptionType, Message } from '../../types'
import {
  attachmentGetDownload,
  channelsMarkRead,
  messagesList,
  messagesEdit,
  messagesDelete,
  messagesReact,
  messagesUnreact,
} from '../../lib/api'
import { decryptAttachmentToBlob, downloadAndDecryptAttachment } from '../../lib/attachments'
import { decryptMessagesForChannel, encryptChannelMessage } from '../../lib/channel-crypto'
import { logger } from '../../lib/logger'
import { useAuth } from '../../contexts/AuthContext'

const REACTION_EMOJIS = ['👍', '❤️', '😂', '😮', '😢', '🔥', '👏', '🎉']

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
  permissionLevel?: number | null
  onReplyTo?: (msg: Message, plaintext: string) => void
  socketDeletedMessageIds?: Set<string>
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
  permissionLevel,
  onReplyTo,
  socketDeletedMessageIds,
}: MessageListProps) {
  const { user, currentDeviceId } = useAuth()
  const [messages, setMessages] = useState<Message[]>([])
  const [decryptedPayloads, setDecryptedPayloads] = useState<Record<string, string>>({})
  const [attachmentById, setAttachmentById] = useState<Record<string, AttachmentView>>({})
  const [thumbnailBlobUrls, setThumbnailBlobUrls] = useState<Record<string, string>>({})
  const [loading, setLoading] = useState(true)
  const [loadingMore, setLoadingMore] = useState(false)
  const [hasPrevPage, setHasPrevPage] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [unreadDividerMessageId, setUnreadDividerMessageId] = useState<string | null>(null)

  // Hover/action state
  const [hoveredMessageId, setHoveredMessageId] = useState<string | null>(null)
  const [emojiPickerMessageId, setEmojiPickerMessageId] = useState<string | null>(null)

  // Inline edit state
  const [editingMessageId, setEditingMessageId] = useState<string | null>(null)
  const [editingText, setEditingText] = useState('')
  const [editSaving, setEditSaving] = useState(false)

  // Inline delete confirm state
  const [confirmDeleteMessageId, setConfirmDeleteMessageId] = useState<string | null>(null)

  const messagesEndRef = useRef<HTMLDivElement>(null)
  const messagesTopRef = useRef<HTMLDivElement>(null)
  const unreadDividerRef = useRef<HTMLDivElement>(null)
  const expiringMessageIdsRef = useRef<Set<string> | undefined>(expiringMessageIds)
  const atBottomRef = useRef(true)
  const lastMarkedReadIdRef = useRef<string | null>(null)
  const isEncryptedChannel = encryptionType !== 'none'

  const canManage = (permissionLevel ?? 0) >= 3

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
        const unreadResult = await messagesList(channelId, 50, undefined, scope, lastReadMessageId!)
        if (!unreadResult.success || !unreadResult.data) {
          setError('No es poden carregar els missatges')
          return
        }
        const unreadMsgs = filterExpiring(unreadResult.data.data)

        let contextMsgs: Message[] = []
        let prevPage = false
        if (unreadMsgs.length > 0) {
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

        const merged = [...fromDesc(contextMsgs), ...unreadMsgs]
        setMessages(merged)
        setHasPrevPage(prevPage)
        const decrypted = await decryptMessagesForChannel(channelId, encryptionType, merged)
        setDecryptedPayloads(decrypted)
      } else {
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
    const oldestId = messages[0].messageId
    setLoadingMore(true)
    try {
      const result = await messagesList(channelId, 50, oldestId, scope)
      if (result.success && result.data && result.data.data.length > 0) {
        const older = filterExpiring(result.data.data)
        setMessages((prev) => {
          const existingIds = new Set(prev.map((m) => m.messageId))
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

  useEffect(() => {
    if (!socketDeletedMessageIds || socketDeletedMessageIds.size === 0) return
    setMessages((prev) => prev.map((m) =>
      socketDeletedMessageIds.has(m.messageId) && !m.deletedAt
        ? { ...m, deletedAt: new Date().toISOString() }
        : m,
    ))
  }, [socketDeletedMessageIds])

  const loadedIds = new Set(messages.map((m) => m.messageId))
  const combined = [
    ...messages,
    ...socketMessages.filter((m) => !loadedIds.has(m.messageId)),
  ].filter((m) => !expiringMessageIds?.has(m.messageId))

  useEffect(() => {
    if (combined.length === 0) return
    let cancelled = false
    decryptMessagesForChannel(channelId, encryptionType, combined)
      .then((decrypted) => { if (!cancelled) setDecryptedPayloads(decrypted) })
      .catch(() => {})
    return () => { cancelled = true }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [channelId, scope, encryptionType, messages, socketMessages, expiringMessageIds])

  useEffect(() => {
    if (loading) return
    if (unreadDividerRef.current) {
      unreadDividerRef.current.scrollIntoView({ behavior: 'instant' as ScrollBehavior })
    } else {
      messagesEndRef.current?.scrollIntoView({ behavior: 'instant' as ScrollBehavior })
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loading, channelId])

  useEffect(() => {
    if (socketMessages.length > 0 && atBottomRef.current) {
      messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
    }
  }, [socketMessages])

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

  useEffect(() => {
    lastMarkedReadIdRef.current = null
  }, [channelId])

  useEffect(() => {
    const el = messagesEndRef.current
    if (!el) return
    const observer = new IntersectionObserver(
      ([entry]) => {
        atBottomRef.current = entry.isIntersecting
        if (entry.isIntersecting) {
          const lastMsg = combined[combined.length - 1]
          if (lastMsg && lastMsg.messageId !== lastMarkedReadIdRef.current) {
            lastMarkedReadIdRef.current = lastMsg.messageId
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

  useEffect(() => {
    return () => {
      setThumbnailBlobUrls((prev) => {
        Object.values(prev).forEach(URL.revokeObjectURL)
        return {}
      })
    }
  }, [channelId])

  // --- Actions ---

  const handleEdit = (msg: Message) => {
    const plaintext = decryptedPayloads[msg.messageId] ?? ''
    setEditingMessageId(msg.messageId)
    setEditingText(plaintext)
  }

  const handleEditSave = async (msg: Message) => {
    if (!editingText.trim()) return
    setEditSaving(true)
    try {
      const { encryptedPayload, iv } = await encryptChannelMessage(
        channelId,
        encryptionType,
        editingText,
        currentDeviceId ?? undefined,
      )
      const result = await messagesEdit(msg.messageId, encryptedPayload, iv)
      if (result.success) {
        setMessages((prev) => prev.map((m) =>
          m.messageId === msg.messageId
            ? { ...m, editedAt: new Date().toISOString() }
            : m,
        ))
        setDecryptedPayloads((prev) => ({ ...prev, [msg.messageId]: editingText }))
        setEditingMessageId(null)
      }
    } catch (err) {
      logger.error('[MessageList] Error editant missatge', err)
    } finally {
      setEditSaving(false)
    }
  }

  const handleDeleteConfirm = async (msg: Message) => {
    setConfirmDeleteMessageId(null)
    const result = await messagesDelete(msg.messageId)
    if (result.success) {
      setMessages((prev) => prev.map((m) =>
        m.messageId === msg.messageId
          ? { ...m, deletedAt: new Date().toISOString() }
          : m,
      ))
    }
  }

  const handleReact = async (msg: Message, emoji: string) => {
    setEmojiPickerMessageId(null)
    const existing = msg.reactions?.find((r) => r.emoji === emoji)
    const alreadyReacted = existing?.userIds.includes(user?.userId ?? '')

    if (alreadyReacted) {
      await messagesUnreact(msg.messageId, emoji)
      setMessages((prev) => prev.map((m) => {
        if (m.messageId !== msg.messageId) return m
        const reactions = (m.reactions ?? []).map((r) => {
          if (r.emoji !== emoji) return r
          const userIds = r.userIds.filter((id) => id !== user?.userId)
          const usernames = r.usernames.filter((_, i) => r.userIds[i] !== user?.userId)
          return { ...r, userIds, usernames, count: userIds.length }
        }).filter((r) => r.count > 0)
        return { ...m, reactions }
      }))
    } else {
      await messagesReact(msg.messageId, emoji)
      setMessages((prev) => prev.map((m) => {
        if (m.messageId !== msg.messageId) return m
        const reactions = [...(m.reactions ?? [])]
        const idx = reactions.findIndex((r) => r.emoji === emoji)
        if (idx >= 0) {
          const r = reactions[idx]
          reactions[idx] = {
            ...r,
            userIds: [...r.userIds, user?.userId ?? ''],
            usernames: [...r.usernames, user?.username ?? ''],
            count: r.count + 1,
          }
        } else {
          reactions.push({
            emoji,
            userIds: [user?.userId ?? ''],
            usernames: [user?.username ?? ''],
            count: 1,
          })
        }
        return { ...m, reactions }
      }))
    }
  }

  const handleReply = (msg: Message) => {
    const plaintext = decryptedPayloads[msg.messageId] ?? msg.encryptedPayload
    onReplyTo?.(msg, plaintext)
  }

  // Find reply-to message for display
  const messageById = new Map(combined.map((m) => [m.messageId, m]))

  // Early returns
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
    <div className="message-list" onClick={() => setEmojiPickerMessageId(null)} onTouchStart={() => setHoveredMessageId(null)}>
      <div ref={messagesTopRef} className="messages-top-sentinel">
        {loadingMore && <p className="loading-more-indicator">Carregant més...</p>}
        {!loadingMore && hasPrevPage && <p className="load-more-hint">Fes scroll cap amunt per veure més</p>}
      </div>

      {combined.map((msg, index) => {
        const showDivider = unreadDividerMessageId === msg.messageId
        const showHeader =
          index === 0 || combined[index - 1].senderUserId !== msg.senderUserId || showDivider
        const isOwnMessage = msg.senderUserId === user?.userId
        const canDelete = isOwnMessage || canManage
        const canEdit = isOwnMessage
        const isEditing = editingMessageId === msg.messageId
        const plaintext = decryptedPayloads[msg.messageId] ?? msg.encryptedPayload
        const replyParent = msg.replyToMessageId ? messageById.get(msg.replyToMessageId) : null
        const replyParentText = replyParent
          ? (decryptedPayloads[replyParent.messageId] ?? replyParent.encryptedPayload)
          : null

        return (
          <React.Fragment key={msg.messageId}>
            {showDivider && (
              <div ref={unreadDividerRef} id="unread-divider" className="unread-divider">
                <span>Missatges nous</span>
              </div>
            )}
            <div
              className={`message-bubble ${msg.deletedAt ? 'deleted' : ''} ${msg.editedAt ? 'edited' : ''} ${showHeader ? 'first-in-row' : ''} ${expiringMessageIds?.has(msg.messageId) ? 'expiring' : ''} ${hoveredMessageId === msg.messageId ? 'hovered' : ''}`}
              onMouseEnter={() => setHoveredMessageId(msg.messageId)}
              onMouseLeave={() => setHoveredMessageId(null)}
              onTouchStart={(e) => { e.stopPropagation(); setHoveredMessageId(msg.messageId) }}
            >
              {/* Menú d'accions */}
              {hoveredMessageId === msg.messageId && !msg.deletedAt && !isEditing && (
                <div className="message-actions" onClick={(e) => e.stopPropagation()}>
                  {onReplyTo && (
                    <button
                      className="message-action-btn"
                      title="Respondre"
                      onClick={() => handleReply(msg)}
                    >↩</button>
                  )}
                  <button
                    className="message-action-btn"
                    title="Reaccionar"
                    onClick={(e) => {
                      e.stopPropagation()
                      setEmojiPickerMessageId((prev) => prev === msg.messageId ? null : msg.messageId)
                    }}
                  >😀</button>
                  {canEdit && (
                    <button
                      className="message-action-btn"
                      title="Editar"
                      onClick={() => handleEdit(msg)}
                    >✏️</button>
                  )}
                  {canDelete && (
                    <button
                      className="message-action-btn message-action-btn--danger"
                      title="Eliminar"
                      onClick={() => setConfirmDeleteMessageId(msg.messageId)}
                    >🗑</button>
                  )}
                  {emojiPickerMessageId === msg.messageId && (
                    <div className="emoji-picker">
                      {REACTION_EMOJIS.map((emoji) => (
                        <button
                          key={emoji}
                          className="emoji-picker-btn"
                          onClick={() => void handleReact(msg, emoji)}
                        >{emoji}</button>
                      ))}
                    </div>
                  )}
                </div>
              )}

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
                {/* Reply-to context */}
                {replyParent && replyParentText && !msg.deletedAt && (
                  <div className="message-reply-context">
                    <span className="message-reply-sender">{replyParent.senderUsername}</span>
                    <span className="message-reply-text">
                      {replyParentText.slice(0, 100)}{replyParentText.length > 100 ? '…' : ''}
                    </span>
                  </div>
                )}

                {msg.deletedAt ? (
                  <p className="deleted-message">Missatge eliminat</p>
                ) : isEditing ? (
                  <div className="message-edit-form">
                    <input
                      className="message-edit-input"
                      value={editingText}
                      onChange={(e) => setEditingText(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); void handleEditSave(msg) }
                        if (e.key === 'Escape') setEditingMessageId(null)
                      }}
                      autoFocus
                      disabled={editSaving}
                    />
                    <div className="message-edit-actions">
                      <button
                        className="message-edit-save"
                        onClick={() => void handleEditSave(msg)}
                        disabled={editSaving}
                      >Desar</button>
                      <button
                        className="message-edit-cancel"
                        onClick={() => setEditingMessageId(null)}
                        disabled={editSaving}
                      >Cancel·lar</button>
                    </div>
                  </div>
                ) : (
                  renderMarkdownMessage(plaintext)
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

                {/* Reactions display */}
                {(msg.reactions?.length ?? 0) > 0 && !msg.deletedAt && (
                  <div className="message-reactions">
                    {msg.reactions!.map((reaction) => {
                      const reacted = reaction.userIds.includes(user?.userId ?? '')
                      return (
                        <button
                          key={reaction.emoji}
                          className={`reaction-chip ${reacted ? 'reaction-chip--active' : ''}`}
                          title={reaction.usernames.join(', ')}
                          onClick={() => void handleReact(msg, reaction.emoji)}
                        >
                          {reaction.emoji} {reaction.count}
                        </button>
                      )
                    })}
                  </div>
                )}
              </div>

              {/* Confirmació d'esborrar inline */}
              {confirmDeleteMessageId === msg.messageId && (
                <div className="message-delete-confirm" onClick={(e) => e.stopPropagation()}>
                  <span className="message-delete-confirm-text">Eliminar aquest missatge?</span>
                  <button
                    className="message-delete-confirm-yes"
                    onClick={() => void handleDeleteConfirm(msg)}
                  >Eliminar</button>
                  <button
                    className="message-delete-confirm-no"
                    onClick={() => setConfirmDeleteMessageId(null)}
                  >Cancel·lar</button>
                </div>
              )}

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

      <div ref={messagesEndRef} />
    </div>
  )
}
