import React, { useState, useEffect, useRef } from 'react'
import { Message } from '../../types'
import { messagesList } from '../../lib/api'

interface MessageListProps {
  channelId: string
  refreshKey?: number
  socketMessages?: Message[]
}

export function MessageList({ channelId, refreshKey, socketMessages = [] }: MessageListProps) {
  const [messages, setMessages] = useState<Message[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const messagesEndRef = useRef<HTMLDivElement>(null)

  const loadMessages = async () => {
    // Debug: veure què arriba
    console.log('[MessageList] channelId prop:', channelId)
    
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
        setMessages(result.data.data)
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

  // Combinar missatges carregats + missatges rebuts via socket (sense duplicats)
  const loadedIds = new Set(messages.map((m) => m.messageId))
  const combined = [
    ...messages,
    ...socketMessages.filter((m) => !loadedIds.has(m.messageId)),
  ].sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime())

  return (
    <div className="message-list">
      {combined.map((msg, index) => {
        const showHeader =
          index === 0 || combined[index - 1].senderUserId !== msg.senderUserId

        return (
          <div
            key={msg.messageId}
            className={`message-bubble ${msg.deletedAt ? 'deleted' : ''} ${msg.editedAt ? 'edited' : ''} ${showHeader ? 'first-in-row' : ''}`}
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