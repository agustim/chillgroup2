import React, { useRef, useEffect, useState, MutableRefObject } from 'react'

interface MediaFilePlayerProps {
  mediaFileElementRef: MutableRefObject<HTMLVideoElement | null>
  fileName: string
  onStop: () => void
}

export function MediaFilePlayer({ mediaFileElementRef, fileName, onStop }: MediaFilePlayerProps) {
  const containerRef = useRef<HTMLDivElement>(null)
  const [currentTime, setCurrentTime] = useState(0)
  const [duration, setDuration] = useState(0)
  const [isPlaying, setIsPlaying] = useState(true)
  const [isVideo, setIsVideo] = useState(false)
  const [isLocallyMuted, setIsLocallyMuted] = useState(false)

  useEffect(() => {
    const el = mediaFileElementRef.current
    const container = containerRef.current
    if (!el || !container) return

    const checkVideo = () => {
      setIsVideo(el.videoWidth > 0 || el.videoHeight > 0)
    }
    const onTimeUpdate = () => setCurrentTime(el.currentTime)
    const onDurationChange = () => setDuration(el.duration)
    const onPlay = () => setIsPlaying(true)
    const onPause = () => setIsPlaying(false)
    const onMetadata = () => {
      setDuration(el.duration)
      checkVideo()
    }

    el.addEventListener('timeupdate', onTimeUpdate)
    el.addEventListener('durationchange', onDurationChange)
    el.addEventListener('loadedmetadata', onMetadata)
    el.addEventListener('play', onPlay)
    el.addEventListener('pause', onPause)

    if (el.readyState >= 1) onMetadata()
    setCurrentTime(el.currentTime)
    setIsPlaying(!el.paused)

    // Move element into container for display
    el.style.cssText = 'width:100%;max-height:140px;object-fit:contain;display:block;background:#000;'
    container.appendChild(el)

    return () => {
      el.removeEventListener('timeupdate', onTimeUpdate)
      el.removeEventListener('durationchange', onDurationChange)
      el.removeEventListener('loadedmetadata', onMetadata)
      el.removeEventListener('play', onPlay)
      el.removeEventListener('pause', onPause)

      // Return element to hidden body position (hook will clean up)
      el.style.cssText = 'position:absolute;left:-9999px;top:-9999px;width:1px;height:1px;'
      document.body.appendChild(el)
    }
  }, [mediaFileElementRef])

  const formatTime = (secs: number) => {
    if (!isFinite(secs) || isNaN(secs)) return '0:00'
    const m = Math.floor(secs / 60)
    const s = Math.floor(secs % 60)
    return `${m}:${s.toString().padStart(2, '0')}`
  }

  const handleSeek = (e: React.ChangeEvent<HTMLInputElement>) => {
    const el = mediaFileElementRef.current
    if (!el) return
    el.currentTime = parseFloat(e.target.value)
  }

  const togglePlayPause = () => {
    const el = mediaFileElementRef.current
    if (!el) return
    if (el.paused) {
      void el.play()
    } else {
      el.pause()
    }
  }

  const toggleLocalMute = () => {
    const el = mediaFileElementRef.current
    if (!el) return
    const newMuted = !isLocallyMuted
    el.muted = newMuted
    setIsLocallyMuted(newMuted)
  }

  const displayName = fileName.length > 28 ? fileName.slice(0, 25) + '…' : fileName

  return (
    <div className="media-file-player">
      <div className="media-file-player-header">
        <span className="media-file-player-name" title={fileName}>
          🎬 {displayName}
        </span>
        <button className="media-file-player-close" onClick={onStop} title="Aturar i tancar">
          ✕
        </button>
      </div>
      <div
        ref={containerRef}
        className="media-file-player-video-container"
        style={isVideo ? undefined : { display: 'none' }}
      />
      <div className="media-file-player-controls">
        <button className="media-file-player-btn" onClick={togglePlayPause} title={isPlaying ? 'Pausar' : 'Reproduir'}>
          {isPlaying ? '⏸' : '▶'}
        </button>
        <button className="media-file-player-btn" onClick={toggleLocalMute} title={isLocallyMuted ? 'Activar so local' : 'Silenciar localment'}>
          {isLocallyMuted ? '🔇' : '🔊'}
        </button>
        <input
          className="media-file-player-seek"
          type="range"
          min={0}
          max={duration || 0}
          step={0.5}
          value={currentTime}
          onChange={handleSeek}
        />
        <span className="media-file-player-time">
          {formatTime(currentTime)} / {formatTime(duration)}
        </span>
      </div>
    </div>
  )
}
