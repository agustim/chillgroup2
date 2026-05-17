import React, { useState } from 'react'
import { Channel } from '../../types'
import { messagesSend } from '../../lib/api'
import { MessageList } from './MessageList'
import { VoiceArea } from './VoiceArea'
import { MessageInput } from './MessageInput'

interface MainContentProps {
  channel: Channel
  voiceJoined?: boolean
  onToggleVoice?: () => void
}

export function MainContent({ channel, voiceJoined = false, onToggleVoice }: MainContentProps) {
  const [message, setMessage] = useState('')
  const [refreshKey, setRefreshKey] = useState(0)
  const [sending, setSending] = useState(false)
  const [sendError, setSendError] = useState<string | null>(null)

  const handleSendMessage = async () => {
    const trimmedMessage = message.trim()
    if (!trimmedMessage || sending || channel.type === 'voice') {
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
        setSendError(response.error.message || 'No s’ha pogut enviar el missatge')
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

  // Canals de veu
  if (channel.type === 'voice') {
    return (
      <div className="main-content">
        <VoiceArea
          channel={channel}
          joined={voiceJoined}
          onToggle={onToggleVoice}
        />
      </div>
    )
  }

  // Canals de text
  return (
    <div className="main-content">
      <MessageList channelId={channel.channelId} refreshKey={refreshKey} />
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