import React, { useState } from 'react'
import { Channel } from '../../types'
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

  const handleSendMessage = () => {
    if (message.trim()) {
      // TODO: Enviar missatge via API
      setMessage('')
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
      <MessageList channelId={channel.channelId} />
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