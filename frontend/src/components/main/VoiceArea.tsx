import React, { useRef, useEffect, useState } from 'react'
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
        {participant.isSuppressed && <span title="Micròfon apagat">🔕</span>}
        {participant.isDeafened && <span title="Altaveu apagat">🔇</span>}
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

  const MIN_ZOOM = 0.7
  const MAX_ZOOM = 2.2
  const DEFAULT_ZOOM = 1.5

  const [participantZoom, setParticipantZoom] = useState(DEFAULT_ZOOM)

  const conn = connection
  const localParticipant = conn.participants[0]
  const remoteParticipants = conn.participants.slice(1)

  const zoomIn = () => setParticipantZoom((prev) => Math.min(MAX_ZOOM, +(prev + 0.1).toFixed(2)))
  const zoomOut = () => setParticipantZoom((prev) => Math.max(MIN_ZOOM, +(prev - 0.1).toFixed(2)))
  const resetZoom = () => setParticipantZoom(DEFAULT_ZOOM)

  const isAtMinZoom = participantZoom <= MIN_ZOOM
  const isAtMaxZoom = participantZoom >= MAX_ZOOM
  const participantsGridStyle = {
    gridTemplateColumns: `repeat(auto-fill, minmax(${(130 * participantZoom).toFixed(1)}px, 1fr))`,
    ['--participant-zoom' as '--participant-zoom']: participantZoom,
  } as React.CSSProperties & Record<'--participant-zoom', number>

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
          <button
            className="voice-control-btn"
            onClick={zoomOut}
            title="Fer més petits els participants"
            disabled={isAtMinZoom}
          >
            -
          </button>
          <button
            className="voice-control-btn"
            onClick={resetZoom}
            title="Restablir zoom"
          >
            {Math.round(participantZoom * 100)}%
          </button>
          <button
            className="voice-control-btn"
            onClick={zoomIn}
            title="Fer més grans els participants"
            disabled={isAtMaxZoom}
          >
            +
          </button>
        </div>

        <button className="leave-voice-btn" onClick={onLeave} title="Surt del canal de veu">
          ✕ Surt
        </button>
      </div>

      {/* Grid de participants — creix dinàmicament */}
      <div className="voice-participants">
        <div className="participants-grid" style={participantsGridStyle}>
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

