import React, { useState, useEffect, useRef } from 'react'
import { Message } from '../../types'
import { messagesList } from '../../lib/api'
import { logger } from '../../lib/logger'

interface MessageListProps {
  channelId: string
  refreshKey?: number
  socketMessages?: Message[]
  expiringMessageIds?: Set<string>
}

export function MessageList({ channelId, refreshKey, socketMessages = [], expiringMessageIds }: MessageListProps) {
  const [messages, setMessages] = useState<Message[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const expiringMessageIdsRef = useRef<Set<string> | undefined>(expiringMessageIds)

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
      const result = await messagesList(channelId, 50)
      if (result.success && result.data) {
        // Filtrar missatges que estan a expiringMessageIds
        const filtered = expiringMessageIdsRef.current && expiringMessageIdsRef.current.size > 0
          ? result.data.data.filter((m) => !expiringMessageIdsRef.current!.has(m.messageId))
          : result.data.data
        setMessages(filtered)
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
  }, [channelId, refreshKey])

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

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

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

  if (messages.length === 0 && socketMessages.length === 0) {
    return (
      <div className="message-list empty">
        <p>Sense missatges encara</p>
        <p className="empty-hint">Sigues el primer a enviar missatge!</p>
      </div>
    )
  }

  // Combinar missatges carregats + missatges rebuts via socket (sense duplicats),
  // i filtrar els que estan a expiringMessageIds per evitar el "flash" després de l'animació
  const loadedIds = new Set(messages.map((m) => m.messageId))
  const combined = [
    ...messages,
    ...socketMessages.filter((m) => !loadedIds.has(m.messageId)),
  ]
    .filter((m) => !expiringMessageIds?.has(m.messageId))
    .sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime())

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
                <p>{msg.encryptedPayload}</p>
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
            </div>
          </div>
        )
      })}
      <div ref={messagesEndRef} />
    </div>
  )
}