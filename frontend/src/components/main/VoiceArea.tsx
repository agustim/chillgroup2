import React, { useRef, useEffect } from 'react'
import { VoiceConnection } from '../../types'

interface VoiceAreaProps {
  connection?: VoiceConnection
  onToggleMute?: () => void
  onToggleDeafen?: () => void
  onToggleCamera?: () => Promise<void>
  onLeave?: () => void
  localVideoTrack?: any
  remoteVideoTracks?: Record<string, any>
}

/** Tile d'un participant amb vídeo o avatar */
function ParticipantTile({
  participant,
  videoTrack,
  isLocal = false,
}: {
  participant: { userId: string; username: string; isSpeaking: boolean; isDeafened: boolean; isSuppressed: boolean }
  videoTrack?: any
  isLocal?: boolean
}) {
  const videoRef = useRef<HTMLVideoElement>(null)

  useEffect(() => {
    if (!videoRef.current || !videoTrack) return
    videoTrack.attach(videoRef.current)
    return () => {
      try { videoTrack.detach(videoRef.current!) } catch (_) { /* ignore */ }
    }
  }, [videoTrack])

  return (
    <div className={`participant-tile ${participant.isSpeaking ? 'speaking' : ''} ${participant.isDeafened ? 'deafened' : ''}`}>
      {videoTrack ? (
        <video
          ref={videoRef}
          autoPlay
          playsInline
          muted={isLocal}
          className="participant-video"
        />
      ) : (
        <div className="participant-avatar">
          {participant.username.charAt(0).toUpperCase()}
        </div>
      )}
      <span className="participant-name">{participant.username}</span>
      <div className="participant-status-icons">
        {participant.isSpeaking && <span title="Parlant">🗣️</span>}
        {participant.isDeafened && <span title="Sord">🔇</span>}
        {participant.isSuppressed && !participant.isSpeaking && <span title="Silenciat">🔕</span>}
      </div>
    </div>
  )
}

export function VoiceArea({
  connection,
  onToggleMute,
  onToggleDeafen,
  onToggleCamera,
  onLeave,
  localVideoTrack,
  remoteVideoTracks = {},
}: VoiceAreaProps) {
  if (!connection) return null

  const conn = connection
  const localParticipant = conn.participants[0]
  const remoteParticipants = conn.participants.slice(1)

  return (
    <div className="voice-area connected">
      {/* Header: icona + nom canal | controls 🎤🔊🎥 | botó sortir */}
      <div className="voice-header">
        <div className="voice-header-info">
          <span className="voice-icon">🔊</span>
          <h3>{conn.channelName}</h3>
        </div>

        <div className="voice-header-controls">
          <button
            className={`voice-control-btn ${conn.isMuted ? 'active-off' : 'active-on'}`}
            onClick={onToggleMute}
            title={conn.isMuted ? 'Activar micròfon' : 'Silenciar micròfon'}
          >
            🎤
          </button>
          <button
            className={`voice-control-btn ${conn.isDeafened ? 'active-off' : 'active-on'}`}
            onClick={onToggleDeafen}
            title={conn.isDeafened ? 'Activar so' : 'Desactivar so'}
          >
            🔊
          </button>
          <button
            className={`voice-control-btn ${conn.isCameraOn ? 'active-on' : 'active-off'}`}
            onClick={onToggleCamera}
            title={conn.isCameraOn ? 'Apagar càmera' : 'Activar càmera'}
          >
            🎥
          </button>
        </div>

        <button className="leave-voice-btn" onClick={onLeave} title="Surt del canal de veu">
          ✕ Surt
        </button>
      </div>

      {/* Grid de participants — creix dinàmicament */}
      <div className="voice-participants">
        <div className="participants-grid">
          {localParticipant && (
            <ParticipantTile
              participant={localParticipant}
              videoTrack={localVideoTrack}
              isLocal
            />
          )}
          {remoteParticipants.map((p) => (
            <ParticipantTile
              key={p.userId}
              participant={p}
              videoTrack={remoteVideoTracks[p.userId]}
            />
          ))}
          {/* Slots buits fins a mínim 4 participants */}
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

