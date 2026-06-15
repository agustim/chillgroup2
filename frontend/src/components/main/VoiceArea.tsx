import React, { useRef, useEffect, useState, MutableRefObject } from 'react'
import { useTranslation } from 'react-i18next'
import { VoiceConnection } from '../../types'
import { MediaFilePlayer } from './MediaFilePlayer'

type ViewMode = 'mosaic' | 'focus'

type ParticipantRenderItem = {
  id: string
  participant: { userId: string; username: string; isSpeaking: boolean; isDeafened: boolean; isSuppressed: boolean }
  videoTrack?: any
  streamBadge?: string
  isLocal?: boolean
  isScreen?: boolean
}

interface VoiceAreaProps {
  connection?: VoiceConnection
  onLeave?: () => void
  voiceAsTextMode?: boolean
  onToggleVoiceAsTextMode?: () => void
  localVideoTrack?: any
  localScreenTrack?: any
  localMediaFileTrack?: any
  mediaFileName?: string | null
  mediaFileElementRef?: MutableRefObject<HTMLVideoElement | null>
  onStopMediaFileShare?: () => void
  onSetParticipantLocalMuted?: (identity: string, muted: boolean) => void
  isMediaFileSharing?: boolean
  remoteVideoTracks?: Record<string, any[]>
}

/** Tile d'un participant amb vídeo o avatar */
function ParticipantTile({
  participant,
  videoTrack,
  streamBadge,
  isLocal = false,
  isPinned = false,
  isLocallyMuted = false,
  showMuteBtn = false,
  onTogglePin,
  onOpenPopout,
  onToggleLocalMute,
}: {
  participant: { userId: string; username: string; isSpeaking: boolean; isDeafened: boolean; isSuppressed: boolean }
  videoTrack?: any
  streamBadge?: string
  isLocal?: boolean
  isPinned?: boolean
  isLocallyMuted?: boolean
  showMuteBtn?: boolean
  onTogglePin?: () => void
  onOpenPopout?: () => void
  onToggleLocalMute?: () => void
}) {
  const { t } = useTranslation()
  const videoRef = useRef<HTMLVideoElement>(null)

  useEffect(() => {
    if (!videoRef.current || !videoTrack) return
    videoTrack.attach(videoRef.current)
    return () => {
      try { videoTrack.detach(videoRef.current!) } catch (_) { /* ignore */ }
    }
  }, [videoTrack])

  return (
    <div className={`participant-tile ${participant.isSpeaking ? 'speaking' : ''} ${participant.isDeafened ? 'deafened' : ''} ${isPinned ? 'pinned' : ''}`}>
      <div className="participant-tile-actions">
        <button
          className={`participant-action-btn ${isPinned ? 'active-on' : ''}`}
          onClick={onTogglePin}
          title={isPinned ? t('voiceArea.unpin') : t('voiceArea.pin')}
        >
          📌
        </button>
        <button
          className="participant-action-btn"
          onClick={onOpenPopout}
          title={t('voiceArea.popout')}
        >
          ⛶
        </button>
        {showMuteBtn && (
          <button
            className={`participant-action-btn ${isLocallyMuted ? 'active-off' : ''}`}
            onClick={onToggleLocalMute}
            title={isLocallyMuted ? t('voiceArea.unmuteLocal') : t('voiceArea.muteLocal')}
          >
            {isLocallyMuted ? '🔇' : '🔊'}
          </button>
        )}
      </div>
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
        {participant.isSuppressed && <span title={t('channels.micOff')}>🔕</span>}
        {participant.isDeafened && <span title={t('channels.speakerOff')}>🔇</span>}
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
  localMediaFileTrack,
  mediaFileName,
  mediaFileElementRef,
  onStopMediaFileShare,
  onSetParticipantLocalMuted,
  isMediaFileSharing = false,
  remoteVideoTracks = {},
}: VoiceAreaProps) {
  if (!connection) return null

  const MIN_ZOOM = 0.7
  const MAX_ZOOM = 2.2
  const DEFAULT_ZOOM = 1.5

  const { t } = useTranslation()
  const [participantZoom, setParticipantZoom] = useState(DEFAULT_ZOOM)
  const [viewMode, setViewMode] = useState<ViewMode>('mosaic')
  const [pinnedParticipantId, setPinnedParticipantId] = useState<string | null>(null)
  const [localMutedStreamIds, setLocalMutedStreamIds] = useState<Set<string>>(new Set())

  const toggleLocalMute = (item: ParticipantRenderItem) => {
    const wasMuted = localMutedStreamIds.has(item.id)
    const newMuted = !wasMuted
    if (item.isLocal && item.id.endsWith('-file')) {
      if (mediaFileElementRef?.current) mediaFileElementRef.current.muted = newMuted
    } else if (!item.isLocal) {
      onSetParticipantLocalMuted?.(item.participant.userId, newMuted)
    }
    setLocalMutedStreamIds(prev => {
      const next = new Set(prev)
      if (newMuted) next.add(item.id)
      else next.delete(item.id)
      return next
    })
  }

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

  const participantsContainerStyle = {
    ['--participant-zoom' as '--participant-zoom']: participantZoom,
  } as React.CSSProperties & Record<'--participant-zoom', number>

  const renderParticipants: ParticipantRenderItem[] = []

  if (localParticipant) {
    renderParticipants.push({
      id: localParticipant.userId,
      participant: localParticipant,
      videoTrack: localVideoTrack,
      streamBadge: localVideoTrack ? 'CAM' : undefined,
      isLocal: true,
      isScreen: false,
    })

    if (localScreenTrack) {
      renderParticipants.push({
        id: `${localParticipant.userId}-screen`,
        participant: localParticipant,
        videoTrack: localScreenTrack,
        streamBadge: 'SCREEN',
        isLocal: true,
        isScreen: true,
      })
    }

    if (isMediaFileSharing) {
      renderParticipants.push({
        id: `${localParticipant.userId}-file`,
        participant: localParticipant,
        videoTrack: localMediaFileTrack ?? undefined,
        streamBadge: 'FILE',
        isLocal: true,
        isScreen: false,
      })
    }
  }

  for (const p of remoteParticipants) {
    const tracks = remoteVideoTracks[p.userId] ?? []

    if (tracks.length === 0) {
      renderParticipants.push({
        id: p.userId,
        participant: p,
        isScreen: false,
      })
      continue
    }

    tracks.forEach((track, idx) => {
      const source = String(track?.source ?? '').toLowerCase()
      const isScreen = source.includes('screen')
      renderParticipants.push({
        id: `${p.userId}-${track?.sid || idx}`,
        participant: p,
        videoTrack: track,
        streamBadge: isScreen ? 'SCREEN' : 'CAM',
        isScreen,
      })
    })
  }

  const pinnedParticipant = pinnedParticipantId
    ? renderParticipants.find((item) => item.id === pinnedParticipantId)
    : undefined

  useEffect(() => {
    if (!pinnedParticipantId) return
    if (!renderParticipants.some((item) => item.id === pinnedParticipantId)) {
      setPinnedParticipantId(null)
    }
  }, [pinnedParticipantId, renderParticipants])

  const automaticFocusParticipant =
    renderParticipants.find((item) => item.isScreen && item.participant.isSpeaking) ||
    renderParticipants.find((item) => item.isScreen) ||
    renderParticipants.find((item) => item.participant.isSpeaking) ||
    renderParticipants[0]

  const focusedParticipant = pinnedParticipant || automaticFocusParticipant
  const sideParticipants = renderParticipants.filter((item) => item.id !== focusedParticipant?.id)

  const togglePinParticipant = (participantId: string) => {
    setPinnedParticipantId((current) => (current === participantId ? null : participantId))
    setViewMode('focus')
  }

  const openParticipantPopout = (item: ParticipantRenderItem) => {
    const popup = window.open('', '_blank', 'popup,width=1280,height=720')
    if (!popup) return

    const safeName = item.participant.username.replace(/[&<>"']/g, (char) => {
      if (char === '&') return '&amp;'
      if (char === '<') return '&lt;'
      if (char === '>') return '&gt;'
      if (char === '"') return '&quot;'
      return '&#39;'
    })

    popup.document.title = `${item.participant.username} - ${t('voiceArea.liveView')}`
    popup.document.body.innerHTML = `
      <style>
        html, body { margin: 0; width: 100%; height: 100%; background: #000; overflow: hidden; }
        .popout-wrap { position: relative; width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; }
        .popout-video { width: 100%; height: 100%; object-fit: contain; background: #000; }
        .popout-avatar { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; font: 800 28vh/1 sans-serif; color: #fff; }
        .popout-label { position: absolute; left: 16px; bottom: 16px; background: rgba(0,0,0,0.55); color: #fff; padding: 8px 12px; border-radius: 10px; font: 600 14px/1.2 sans-serif; }
        .popout-btn { position: absolute; top: 16px; right: 16px; border: 1px solid rgba(255,255,255,0.35); background: rgba(0,0,0,0.45); color: #fff; border-radius: 8px; padding: 8px 10px; cursor: pointer; }
      </style>
      <div class="popout-wrap">
        <button class="popout-btn" id="fs-btn">${t('voiceArea.fullscreen')}</button>
        <div class="popout-label">${safeName}${item.streamBadge ? ` · ${item.streamBadge}` : ''}</div>
      </div>
    `

    const wrap = popup.document.querySelector('.popout-wrap') as HTMLDivElement | null
    if (!wrap) return

    if (item.videoTrack) {
      const video = popup.document.createElement('video')
      video.className = 'popout-video'
      video.autoplay = true
      video.playsInline = true
      video.muted = !!item.isLocal
      wrap.insertBefore(video, wrap.firstChild)
      item.videoTrack.attach(video)
      popup.addEventListener('beforeunload', () => {
        try { item.videoTrack.detach(video) } catch (_) { /* ignore */ }
      })
    } else {
      const avatar = popup.document.createElement('div')
      avatar.className = 'popout-avatar'
      avatar.textContent = item.participant.username.charAt(0).toUpperCase()
      wrap.insertBefore(avatar, wrap.firstChild)
    }

    const requestFullscreen = () => {
      const root = popup.document.documentElement
      if (root.requestFullscreen) {
        root.requestFullscreen().catch(() => {
          // Ignore if browser blocks automatic fullscreen request.
        })
      }
    }

    const fsBtn = popup.document.getElementById('fs-btn')
    fsBtn?.addEventListener('click', requestFullscreen)
    requestFullscreen()
  }

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
            className={`voice-control-btn voice-mode-btn ${viewMode === 'mosaic' ? 'active-on' : ''}`}
            onClick={() => setViewMode('mosaic')}
            title={t('voiceArea.modeMosaic')}
          >
            MOS
          </button>
          <button
            className={`voice-control-btn voice-mode-btn ${viewMode === 'focus' ? 'active-on' : ''}`}
            onClick={() => setViewMode('focus')}
            title={t('voiceArea.modeFocus')}
          >
            FCS
          </button>
          <button
            className={`voice-control-btn voice-mode-btn ${voiceAsTextMode ? 'active-on' : 'active-off'}`}
            onClick={onToggleVoiceAsTextMode}
            title={voiceAsTextMode ? t('voiceArea.modeFixOn') : t('voiceArea.modeTabOn')}
          >
            {voiceAsTextMode ? 'FIX' : 'TAB'}
          </button>
          <button
            className="voice-control-btn"
            onClick={zoomOut}
            title={t('voiceArea.zoomOut')}
            disabled={isAtMinZoom}
          >
            -
          </button>
          <button
            className="voice-control-btn"
            onClick={resetZoom}
            title={t('voiceArea.zoomReset')}
          >
            {Math.round(participantZoom * 100)}%
          </button>
          <button
            className="voice-control-btn"
            onClick={zoomIn}
            title={t('voiceArea.zoomIn')}
            disabled={isAtMaxZoom}
          >
            +
          </button>
        </div>


      </div>

      {/* Grid de participants — creix dinàmicament */}
      <div className="voice-participants" style={participantsContainerStyle}>
        {viewMode === 'focus' && focusedParticipant ? (
          <div className="participants-focus-layout">
            <div className="participants-focus-main">
              <ParticipantTile
                participant={focusedParticipant.participant}
                videoTrack={focusedParticipant.videoTrack}
                streamBadge={focusedParticipant.streamBadge}
                isLocal={focusedParticipant.isLocal}
                isPinned={pinnedParticipantId === focusedParticipant.id}
                isLocallyMuted={localMutedStreamIds.has(focusedParticipant.id)}
                showMuteBtn={!focusedParticipant.isLocal || focusedParticipant.id.endsWith('-file')}
                onTogglePin={() => togglePinParticipant(focusedParticipant.id)}
                onOpenPopout={() => openParticipantPopout(focusedParticipant)}
                onToggleLocalMute={() => toggleLocalMute(focusedParticipant)}
              />
            </div>
            <div className="participants-focus-strip">
              {sideParticipants.map((item) => (
                <ParticipantTile
                  key={item.id}
                  participant={item.participant}
                  videoTrack={item.videoTrack}
                  streamBadge={item.streamBadge}
                  isLocal={item.isLocal}
                  isPinned={pinnedParticipantId === item.id}
                  isLocallyMuted={localMutedStreamIds.has(item.id)}
                  showMuteBtn={!item.isLocal || item.id.endsWith('-file')}
                  onTogglePin={() => togglePinParticipant(item.id)}
                  onOpenPopout={() => openParticipantPopout(item)}
                  onToggleLocalMute={() => toggleLocalMute(item)}
                />
              ))}
            </div>
          </div>
        ) : (
          <div className="participants-grid" style={participantsGridStyle}>
            {renderParticipants.map((item) => (
              <ParticipantTile
                key={item.id}
                participant={item.participant}
                videoTrack={item.videoTrack}
                streamBadge={item.streamBadge}
                isLocal={item.isLocal}
                isPinned={pinnedParticipantId === item.id}
                isLocallyMuted={localMutedStreamIds.has(item.id)}
                showMuteBtn={!item.isLocal || item.id.endsWith('-file')}
                onTogglePin={() => togglePinParticipant(item.id)}
                onOpenPopout={() => openParticipantPopout(item)}
                onToggleLocalMute={() => toggleLocalMute(item)}
              />
            ))}
          </div>
        )}
      </div>

      {mediaFileElementRef?.current && mediaFileName && (
        <div className="media-file-player-overlay">
          <MediaFilePlayer
            mediaFileElementRef={mediaFileElementRef}
            fileName={mediaFileName}
            onStop={onStopMediaFileShare ?? (() => {})}
          />
        </div>
      )}
    </div>
  )
}

