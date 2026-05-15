import React, { useState } from 'react'
import { Channel } from '../../types'

interface VoiceParticipant {
  userId: string
  username: string
  isDeafened: boolean
  isSpeaking: boolean
}

interface VoiceAreaProps {
  channel: Channel
  joined: boolean
  onToggle?: () => void
}

export function VoiceArea({ channel, joined, onToggle }: VoiceAreaProps) {
  const [isDeafened, setIsDeafened] = useState(false)
  const [isMuted, setIsMuted] = useState(false)

  // Mock participants (in real app, this would come from LiveKit + presence)
  const [participants] = useState<VoiceParticipant[]>([
    { userId: '1', username: 'agusti', isDeafened: false, isSpeaking: true },
    { userId: '2', username: 'marcus', isDeafened: false, isSpeaking: false },
  ])

  if (!joined) {
    return (
      <div className="voice-area disconnected">
        <div className="voice-area-empty">
          <span className="voice-icon-large">🔊</span>
          <h3>Canal de veu: {channel.name}</h3>
          <p>Uneix-te per parlar amb altres usuaris</p>
          <button className="join-voice-btn" onClick={onToggle}>
            🎤 Unit/da al canal
          </button>
        </div>
      </div>
    )
  }

  return (
    <div className="voice-area connected">
      {/* Voice Controls */}
      <div className="voice-controls">
        <button
          className={`voice-control-btn ${isMuted ? 'muted' : ''}`}
          onClick={() => setIsMuted(!isMuted)}
          title={isMuted ? 'Activar micròfon' : 'Desactivar micròfon'}
        >
          {isMuted ? '🔇' : '🎤'}
        </button>
        <button
          className={`voice-control-btn ${isDeafened ? 'deafened' : ''}`}
          onClick={() => setIsDeafened(!isDeafened)}
          title={isDeafened ? 'Activar so' : 'Desactivar so'}
        >
          {isDeafened ? '🔕' : '🔊'}
        </button>
        <button className="voice-control-btn leave-btn" onClick={onToggle}>
          🚪 Surt
        </button>
      </div>

      {/* Participants Grid */}
      <div className="voice-participants">
        <h4>Participants ({participants.length})</h4>
        <div className="participants-grid">
          {participants.map((p) => (
            <div
              key={p.userId}
              className={`participant-tile ${p.isSpeaking ? 'speaking' : ''} ${p.isDeafened ? 'deafened' : ''}`}
            >
              <div className="participant-avatar">
                {p.username.charAt(0).toUpperCase()}
              </div>
              <span className="participant-name">{p.username}</span>
              {p.isSpeaking && <span className="speaking-indicator">🗣️</span>}
              {p.isDeafened && <span className="deafened-indicator">🔇</span>}
            </div>
          ))}
          {/* Empty slots */}
          {[...Array(Math.max(0, 4 - participants.length))].map((_, i) => (
            <div key={`empty-${i}`} className="participant-tile empty">
              <span className="empty-slot">+</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}