import React, { useRef, useEffect, useState } from 'react'
import { VoiceConnection } from '../../types'

interface VoiceAreaProps {
  connection?: VoiceConnection
  onLeave?: () => void
  voiceAsTextMode?: boolean
  onToggleVoiceAsTextMode?: () => void
  localVideoTrack?: any
  localScreenTrack?: any
  remoteVideoTracks?: Record<string, any[]>
}

/** Tile d'un participant amb vídeo o avatar */
function ParticipantTile({
  participant,
  videoTrack,
  streamBadge,
  isLocal = false,
}: {
  participant: { userId: string; username: string; isSpeaking: boolean; isDeafened: boolean; isSuppressed: boolean }
  videoTrack?: any
  streamBadge?: string
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
      {streamBadge && <span className="participant-stream-badge">{streamBadge}</span>}
      <div className="participant-status-icons">
        {participant.isSuppressed && <span title="Micròfon apagat">🔕</span>}
        {participant.isDeafened && <span title="Altaveu apagat">🔇</span>}
      </div>
    </div>
  )
}

export function VoiceArea({
  connection,
  onLeave,
  voiceAsTextMode = false,
  onToggleVoiceAsTextMode,
  localVideoTrack,
  localScreenTrack,
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
            className={`voice-control-btn voice-mode-btn ${voiceAsTextMode ? 'active-on' : 'active-off'}`}
            onClick={onToggleVoiceAsTextMode}
            title={voiceAsTextMode ? 'Mode veu com text activat' : 'Mode fixat activat'}
          >
            {voiceAsTextMode ? 'TAB' : 'FIX'}
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


      </div>

      {/* Grid de participants — creix dinàmicament */}
      <div className="voice-participants">
        <div className="participants-grid" style={participantsGridStyle}>
          {localParticipant && (
            <ParticipantTile
              participant={localParticipant}
              videoTrack={localVideoTrack}
              streamBadge={localVideoTrack ? 'CAM' : undefined}
              isLocal
            />
          )}

          {localParticipant && localScreenTrack && (
            <ParticipantTile
              key={`${localParticipant.userId}-screen`}
              participant={localParticipant}
              videoTrack={localScreenTrack}
              streamBadge="SCREEN"
              isLocal
            />
          )}

          {remoteParticipants.map((p) => {
            const tracks = remoteVideoTracks[p.userId] ?? []
            if (tracks.length === 0) {
              return <ParticipantTile key={p.userId} participant={p} />
            }

            return tracks.map((track, idx) => {
              const source = String(track?.source ?? '').toLowerCase()
              const isScreen = source.includes('screen')
              return (
                <ParticipantTile
                  key={`${p.userId}-${track?.sid || idx}`}
                  participant={p}
                  videoTrack={track}
                  streamBadge={isScreen ? 'SCREEN' : 'CAM'}
                />
              )
            })
          })}
        </div>
      </div>
    </div>
  )
}

