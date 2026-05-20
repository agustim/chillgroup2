import React, { useState, useCallback } from 'react'
import { Channel, Message, VoiceConnection } from '../../types'
import { messagesSend } from '../../lib/api'
import { MessageList } from './MessageList'
import { VoiceArea } from './VoiceArea'
import { MessageInput } from './MessageInput'
import { useSocketIO } from '../../hooks/useSocketIO'

interface MainContentProps {
  channel: Channel | null
  voiceConnection: VoiceConnection | null
  onToggleMute?: () => void
  onToggleDeafen?: () => void
  onLeaveVoice?: () => void
  onUnreadUpdated?: (channelId: string, unreadCount: number) => void
}

export function MainContent({ channel, voiceConnection, onToggleMute, onToggleDeafen, onLeaveVoice, onUnreadUpdated }: MainContentProps) {
  const [message, setMessage] = useState('')
  const [refreshKey, setRefreshKey] = useState(0)
  const [sending, setSending] = useState(false)
  const [sendError, setSendError] = useState<string | null>(null)
  const [socketMessages, setSocketMessages] = useState<Message[]>([])

  const handleSocketMessage = useCallback((msg: Message) => {
    setSocketMessages((prev) => {
      // Evitar duplicats
      if (prev.some((m) => m.messageId === msg.messageId)) return prev
      return [...prev, msg]
    })
  }, [])

  useSocketIO({
    channelId: channel?.type === 'text' ? (channel?.channelId ?? null) : null,
    onMessage: handleSocketMessage,
    onUnreadUpdated,
  })

  // Netejar missatges de socket quan canviem de canal
  const channelId = channel?.channelId
  React.useEffect(() => {
    setSocketMessages([])
  }, [channelId])

  const handleSendMessage = async () => {
    const trimmedMessage = message.trim()
    if (!trimmedMessage || sending || !channel || channel.type === 'voice') {
      return
    }

    setSending(true)
    setSendError(null)

    try {
      const response = await messagesSend(channel.channelId, trimmedMessage, '')
      if (response.success) {
        setMessage('')
        setRefreshKey((current) => current + 1)
      } else {
        setSendError(response.error.message || "No s'ha pogut enviar el missatge")
      }
    } catch {
      setSendError('Error en enviar el missatge')
    } finally {
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

  // Quan tens veu connectada, mostra VoiceArea a la part superior i el canal de text a sota
  if (voiceConnection) {
    return (
      <div className="main-content voice-split-layout">
        {/* Voice panel at the top */}
        <div className="voice-panel">
          <VoiceArea
            connection={voiceConnection}
            onToggleMute={onToggleMute}
            onToggleDeafen={onToggleDeafen}
            onLeave={onLeaveVoice}
          />
        </div>

        {/* Text chat below */}
        {channel && channel.type === 'text' ? (
          <div className="text-panel">
            <MessageList channelId={channel.channelId} refreshKey={refreshKey} socketMessages={socketMessages} />
            {sendError && <div className="message-send-error">{sendError}</div>}
            <MessageInput
              value={message}
              onChange={setMessage}
              onKeyDown={handleKeyDown}
              onSubmit={handleSendMessage}
              placeholder={`Missatjar a #${channel.name}`}
              encryptionType={channel.encryptionType}
            />
          </div>
        ) : (
          <div className="text-panel">
            <div className="empty-state">
              <p>Selecciona un canal de text per parlar</p>
            </div>
          </div>
        )}
      </div>
    )
  }

  // Sense veu connectada: mostra el canal seleccionat
  if (!channel) {
    return (
      <div className="main-content">
        <div className="empty-state">
          <p>Selecciona un canal per començar</p>
        </div>
      </div>
    )
  }

  // Canals de veu (seleccionats directament, sense connexió prèvia)
  if (channel.type === 'voice') {
    return (
      <div className="main-content">
        <VoiceArea
          channel={{ channelId: channel.channelId, name: channel.name, type: 'voice' as const }}
          joined={false}
        />
      </div>
    )
  }

  // Canals de text
  return (
    <div className="main-content">
      <MessageList channelId={channel.channelId} refreshKey={refreshKey} socketMessages={socketMessages} />
      {sendError && <div className="message-send-error">{sendError}</div>}
      <MessageInput
        value={message}
        onChange={setMessage}
        onKeyDown={handleKeyDown}
        onSubmit={handleSendMessage}
        placeholder={`Missatjar a #${channel.name}`}
        encryptionType={channel.encryptionType}
      />
    </div>
  )
}
