import React from 'react'
import { VoiceConnection } from '../../types'

interface VoiceAreaProps {
  connection?: VoiceConnection
  channel?: { channelId: string; name: string; type: 'voice' }
  joined?: boolean
  onToggleMute?: () => void
  onToggleDeafen?: () => void
  onLeave?: () => void
}

export function VoiceArea({ connection, channel, joined = false, onToggleMute, onToggleDeafen, onLeave }: VoiceAreaProps) {
  // Prefer permanent connection over temporary channel join
  const conn = connection

  if (!conn && !channel) {
    return null
  }

  // Permanent connection (user is in a voice channel)
  if (conn) {
    return (
      <div className="voice-area connected">
        {/* Header with channel name and leave button */}
        <div className="voice-header">
          <div className="voice-header-info">
            <span className="voice-icon">🔊</span>
            <h3>Unit a: {conn.channelName}</h3>
          </div>
          <button className="leave-voice-btn" onClick={onLeave} title="Surt del canal de veu">
            🚪 Surt
          </button>
        </div>

        {/* Voice Controls */}
        <div className="voice-controls">
          <button
            className={`voice-control-btn ${conn.isMuted ? 'muted' : ''}`}
            onClick={onToggleMute}
            title={conn.isMuted ? 'Activar micròfon' : 'Desactivar micròfon'}
          >
            {conn.isMuted ? '🔇' : '🎤'}
            {conn.isMuted && <span className="control-label">Silenciat</span>}
          </button>
          <button
            className={`voice-control-btn ${conn.isDeafened ? 'deafened' : ''}`}
            onClick={onToggleDeafen}
            title={conn.isDeafened ? 'Activar so' : 'Desactivar so'}
          >
            {conn.isDeafened ? '🔕' : '🔊'}
            {conn.isDeafened && <span className="control-label">Sord</span>}
          </button>
        </div>

        {/* Participants Grid */}
        <div className="voice-participants">
          <h4>Participants ({conn.participants.length})</h4>
          <div className="participants-grid">
            {conn.participants.map((p) => (
              <div
                key={p.userId}
                className={`participant-tile ${p.isSpeaking ? 'speaking' : ''} ${p.isDeafened ? 'deafened' : ''}`}
              >
                <div className="participant-avatar">
                  {p.username.charAt(0).toUpperCase()}
                </div>
                <span className="participant-name">{p.username}</span>
                <div className="participant-status-icons">
                  {p.isSpeaking && <span className="speaking-indicator" title="Parlant">🗣️</span>}
                  {p.isDeafened && <span className="deafened-indicator" title="Sord">🔕</span>}
                  {p.isSuppressed && <span className="suppressed-indicator" title="Suppressat">🔇</span>}
                </div>
              </div>
            ))}
            {/* Empty slots to fill the grid */}
            {[...Array(Math.max(0, 4 - conn.participants.length))].map((_, i) => (
              <div key={`empty-${i}`} className="participant-tile empty">
                <span className="empty-slot">+</span>
              </div>
            ))}
          </div>
        </div>
      </div>
    )
  }

  // Temporary channel view (user clicked a voice channel but hasn't joined yet)
  if (channel && !joined) {
    return (
      <div className="voice-area disconnected">
        <div className="voice-area-empty">
          <span className="voice-icon-large">🔊</span>
          <h3>Canal de veu: {channel.name}</h3>
          <p>Feu clic al canal per unir-vos</p>
        </div>
      </div>
    )
  }

  return null
}
