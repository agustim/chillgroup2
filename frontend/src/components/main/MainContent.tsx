import React, { useState, useCallback } from 'react'
import { Channel, Message, VoiceConnection } from '../../types'
import { messagesSend } from '../../lib/api'
import { MessageList } from './MessageList'
import { VoiceArea } from './VoiceArea'
import { MessageInput } from './MessageInput'
import { ChannelHeader } from './ChannelHeader'
import { useSocketIO } from '../../hooks/useSocketIO'

interface MainContentProps {
  channel: Channel | null
  voiceConnection: VoiceConnection | null
  onLeaveVoice?: () => void
  onUnreadUpdated?: (channelId: string, unreadCount: number) => void
  onConfigureChannel?: () => void
  onInviteChannel?: () => void
  canManageChannel?: boolean
  localVideoTrack?: any
  remoteVideoTracks?: Record<string, any>
}

export function MainContent({
  channel,
  voiceConnection,
  onLeaveVoice,
  onUnreadUpdated,
  onConfigureChannel,
  onInviteChannel,
  canManageChannel = false,
  localVideoTrack,
  remoteVideoTracks = {},
}: MainContentProps) {
  const [message, setMessage] = useState('')
  const [refreshKey, setRefreshKey] = useState(0)
  const [sending, setSending] = useState(false)
  const [sendError, setSendError] = useState<string | null>(null)
  const [socketMessages, setSocketMessages] = useState<Message[]>([])
  const [expiringMessageIds, setExpiringMessageIds] = useState<Set<string>>(new Set())

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

  const isTextChannel = channel?.type === 'text'

  // Layout: voice-panel a l'esquerra (si connectat) + text-area a la dreta (si canal de text)
  return (
    <div className={`main-content ${voiceConnection ? 'voice-active-layout' : ''}`}>
      {/* Panell de veu (sempre visible si connectat, independentment del canal de text) */}
      {voiceConnection && (
        <div className="voice-panel">
          <VoiceArea
            connection={voiceConnection}
            onLeave={onLeaveVoice}
            localVideoTrack={localVideoTrack}
            remoteVideoTracks={remoteVideoTracks}
          />
        </div>
      )}

      {/* Àrea de text: channel-header + missatges + input */}
      {isTextChannel && channel && (
        <div className="text-area">
          <ChannelHeader
            channel={channel}
            canManageChannel={canManageChannel}
            onConfigureChannel={onConfigureChannel}
            onInviteChannel={onInviteChannel}
          />
          <div className="text-panel">
            <MessageList
              channelId={channel.channelId}
              refreshKey={refreshKey}
              socketMessages={socketMessages}
              expiringMessageIds={expiringMessageIds}
            />
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
        </div>
      )}

      {/* Si en veu però sense canal de text seleccionat */}
      {voiceConnection && !isTextChannel && (
        <div className="text-area empty-text-area">
          <div className="empty-state">
            <p>Selecciona un canal de text per xatejar</p>
          </div>
        </div>
      )}
    </div>
  )
}
