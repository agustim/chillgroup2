import React, { useState, useEffect, useRef } from 'react'
import { Message } from '../../types'
import { messagesList } from '../../lib/api'

interface MessageListProps {
  channelId: string
}

export function MessageList({ channelId }: MessageListProps) {
  const [messages, setMessages] = useState<Message[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const messagesEndRef = useRef<HTMLDivElement>(null)

  const loadMessages = async () => {
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
  }, [channelId])

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

  if (messages.length === 0) {
    return (
      <div className="message-list empty">
        <p>Sense missatges encara</p>
        <p className="empty-hint">Sigues el primer a enviar missatge!</p>
      </div>
    )
  }

  return (
    <div className="message-list">
      {messages.map((msg, index) => {
        const showHeader =
          index === 0 || messages[index - 1].senderUserId !== msg.senderUserId

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