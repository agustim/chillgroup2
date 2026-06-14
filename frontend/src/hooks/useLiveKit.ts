// @ts-nocheck
/**
 * Hook per connectar amb LiveKit per a canals de veu reals.
 *
 * Funciona amb livekit-client v2.x API:
 * - Obtenció de token des del backend
 * - Connexió a la sala LiveKit
 * - Publicar/subscriure tracks d'àudio
 * - Permís de micròfon amb getUserMedia
 */

import { useRef, useState, useCallback, useEffect } from 'react'
import {
  Room,
  RoomEvent,
  ExternalE2EEKeyProvider,
  isE2EESupported,
  createLocalScreenTracks,
  createLocalVideoTrack,
  LocalVideoTrack,
  LocalAudioTrack,
} from 'livekit-client'
import E2EEWorker from 'livekit-client/e2ee-worker?worker'
import { logger } from '../lib/logger'
import { getApiBase } from '../lib/api'
import type { EncryptionType, VoiceParticipant } from '../types'

const LIVEKIT_ROOM_PREFIX = 'chillgroup-'

interface UseLiveKitResult {
  /** Connexió estableerta amb LiveKit */
  isConnected: boolean
  /** Pujant el nostre àudio a la sala */
  isPublishing: boolean
  /** Estem mutejats (micròfon apagat) */
  isMuted: boolean
  /** Escoltem els altres (speaker actiu) */
  isDeafened: boolean
  /** Càmera activa */
  isCameraOn: boolean
  /** Compartint pantalla */
  isScreenSharing: boolean
  /** Compartint fitxer de media */
  isMediaFileSharing: boolean
  /** Track de vídeo local (per previsualitzar) */
  localVideoTrack: any | null
  /** Track local de compartir pantalla */
  localScreenTrack: any | null
  /** Track local del fitxer de media (vídeo, si el fitxer és vídeo) */
  localMediaFileTrack: any | null
  /** Nom del fitxer de media que s'està compartint */
  mediaFileName: string | null
  /** Referència a l'element de media (per al reproductor) */
  mediaFileElementRef: React.MutableRefObject<HTMLVideoElement | null>
  /** Tracks de vídeo remots (identity -> track) */
  remoteVideoTracks: Record<string, any[]>
  /** Participants remots a la sala */
  participants: VoiceParticipant[]
  /** Connectar a un canal de veu */
  connectToChannel: (
    channelId: string,
    channelName: string,
    options?: { encryptionType?: EncryptionType; channelKey?: Uint8Array | null }
  ) => Promise<void>
  /** Desconnectar del canal */
  disconnect: () => void
  /** Toggle mute/unmute del micròfon */
  toggleMute: () => Promise<void>
  /** Toggle deafen (sentir o no els altres) */
  toggleDeafen: () => void
  /** Toggle càmera */
  toggleCamera: () => Promise<void>
  /** Toggle compartir pantalla */
  toggleScreenShare: () => Promise<void>
  /** Iniciar compartir fitxer de media */
  startMediaFileShare: (file: File) => Promise<void>
  /** Aturar compartir fitxer de media */
  stopMediaFileShare: () => Promise<void>
  /** Silenciar/dessilenciar localment l'àudio d'un participant remot */
  setParticipantLocalMuted: (identity: string, muted: boolean) => void
  /** Error de connexió */
  error: string | null
}

/** Obtenir token JWT des de sessionStorage */
function getJwtToken(): string | null {
  try {
    return sessionStorage.getItem('chillgroup-token')
  } catch {
    return null
  }
}

export function useLiveKit(): UseLiveKitResult {
    const toArrayBuffer = useCallback((bytes: Uint8Array): ArrayBuffer => {
      if (bytes.byteOffset === 0 && bytes.byteLength === bytes.buffer.byteLength) {
        return bytes.buffer
      }
      return bytes.slice().buffer
    }, [])

  const roomRef = useRef<Room | null>(null)
  const localAudioTrackRef = useRef<any>(null)
  const localVideoTrackRef = useRef<any>(null)
  const localScreenTrackRef = useRef<any>(null)
  const audioElementsRef = useRef<Map<string, HTMLAudioElement>>(new Map())
  const remoteVideoTracksRef = useRef<Map<string, any[]>>(new Map())
  const isDeafenedRef = useRef(false)
  const isCameraOnRef = useRef(false)
  const isScreenSharingRef = useRef(false)
  const mediaFileElementRef = useRef<HTMLVideoElement | null>(null)
  const mediaFileObjectUrlRef = useRef<string | null>(null)
  const localMediaFileVideoTrackRef = useRef<any>(null)
  const localMediaFileAudioTrackRef = useRef<any>(null)
  const [isConnected, setIsConnected] = useState(false)
  const [isPublishing, setIsPublishing] = useState(false)
  const [isMuted, setIsMuted] = useState(true)
  const [isDeafened, setIsDeafened] = useState(false)
  const [isCameraOn, setIsCameraOn] = useState(false)
  const [isScreenSharing, setIsScreenSharing] = useState(false)
  const [isMediaFileSharing, setIsMediaFileSharing] = useState(false)
  const [localVideoTrack, setLocalVideoTrack] = useState<any>(null)
  const [localScreenTrack, setLocalScreenTrack] = useState<any>(null)
  const [localMediaFileTrack, setLocalMediaFileTrack] = useState<any>(null)
  const [mediaFileName, setMediaFileName] = useState<string | null>(null)
  const [remoteVideoTracks, setRemoteVideoTracks] = useState<Record<string, any[]>>({})
  const [participants, setParticipants] = useState<VoiceParticipant[]>([])
  const [error, setError] = useState<string | null>(null)

  // Netejar quan es desmunta
  useEffect(() => {
    return () => {
      audioElementsRef.current.forEach(el => {
        el.srcObject = null
        el.remove()
      })
      audioElementsRef.current.clear()
      if (mediaFileElementRef.current) {
        mediaFileElementRef.current.pause()
        mediaFileElementRef.current.src = ''
        try { mediaFileElementRef.current.remove() } catch (_) {}
        mediaFileElementRef.current = null
      }
      if (mediaFileObjectUrlRef.current) {
        URL.revokeObjectURL(mediaFileObjectUrlRef.current)
        mediaFileObjectUrlRef.current = null
      }
      roomRef.current?.disconnect()
      roomRef.current = null
      localAudioTrackRef.current = null
    }
  }, [])

  // Firefox rejects getDisplayMedia with frameRate constraints → retry unconstrained.
  // width/height 0 causes livekit to skip the resolution block entirely.
  const captureScreenTracks = async () => {
    try {
      return await createLocalScreenTracks({ audio: false })
    } catch (e: unknown) {
      if (e instanceof Error && e.name === 'NotSupportedError') {
        return createLocalScreenTracks({ audio: false, resolution: { width: 0, height: 0, frameRate: 0 } as any })
      }
      throw e
    }
  }

  // Adjuntar un track d'àudio remot al DOM perquè es pugui sentir
  const attachRemoteAudio = useCallback((track: any, participantSid: string) => {
    // Evitar duplicats
    const existing = audioElementsRef.current.get(participantSid)
    if (existing) {
      existing.srcObject = null
      existing.remove()
      audioElementsRef.current.delete(participantSid)
    }
    const audioEl = track.attach() as HTMLAudioElement
    audioEl.setAttribute('data-participant', participantSid)
    audioEl.autoplay = true
    audioEl.muted = isDeafenedRef.current
    document.body.appendChild(audioEl)
    audioElementsRef.current.set(participantSid, audioEl)
    logger.debug('🔊 Àudio adjuntat per:', participantSid)
  }, [])

  // Eliminar l'element d'àudio d'un participant
  const detachRemoteAudio = useCallback((participantSid: string) => {
    const el = audioElementsRef.current.get(participantSid)
    if (el) {
      el.srcObject = null
      el.remove()
      audioElementsRef.current.delete(participantSid)
      logger.debug('🔇 Àudio eliminat per:', participantSid)
    }
  }, [])

  // Obtenir token del backend
  const fetchToken = useCallback(async (channelId: string): Promise<{ token: string; url: string }> => {
    const token = getJwtToken()
    if (!token) throw new Error('No estàs autenticat')

    const response = await fetch(`${getApiBase()}/api/livekit/token`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({
        room: LIVEKIT_ROOM_PREFIX + channelId,
      }),
    })

    if (!response.ok) {
      const text = await response.text()
      throw new Error(`Error obtenint token LiveKit: ${response.status} ${text}`)
    }

    const data = await response.json()
    return {
      token: data.token,
      url: data.url || __LIVEKIT_HOST__,
    }
  }, [])

  const hasMicrophoneEnabled = useCallback((participant: any): boolean => {
    if (!participant) return false

    const getPub = participant.getTrackPublication?.bind(participant)
    const directPub = getPub?.('microphone') ?? getPub?.('audio')
    if (directPub) return !directPub.isMuted

    const pubs = participant.trackPublications
      ? Array.from(participant.trackPublications.values())
      : []
    const micPub = pubs.find((pub: any) => pub?.source === 'microphone' || pub?.kind === 'audio')
    return !!(micPub && !micPub.isMuted)
  }, [])

  const getTrackKey = useCallback((track: any): string => {
    return track?.sid || track?.mediaStreamTrack?.id || String(track)
  }, [])

  // Actualitzar la llista de participants
  const updateParticipants = useCallback(() => {
    if (!roomRef.current) return
    const room = roomRef.current
    const remoteParts = room.remoteParticipants
    const parts: VoiceParticipant[] = []

    const localPart = room.localParticipant
    if (localPart) {
      const localHasAudio = hasMicrophoneEnabled(localPart)
      parts.push({
        userId: localPart.identity || localPart.sid,
        username: localPart.name || localPart.identity || 'Tu',
        isSpeaking: localPart.isSpeaking,
        isDeafened: isDeafenedRef.current,
        isSuppressed: !localHasAudio,
        joinedAt: new Date().toISOString(),
        videoTrack: localVideoTrackRef.current ?? undefined,
      })
    }

    for (const p of remoteParts.values()) {
      const hasAudio = hasMicrophoneEnabled(p)
      const userId = p.identity || p.sid
      const remoteTracks = remoteVideoTracksRef.current.get(userId) ?? []
      parts.push({
        userId,
        username: p.name || p.identity || 'Desconegut',
        isSpeaking: p.isSpeaking,
        isDeafened: false,
        isSuppressed: !hasAudio,
        joinedAt: new Date().toISOString(),
        videoTrack: remoteTracks[0] ?? undefined,
      })
    }
    setParticipants(parts)
  }, [hasMicrophoneEnabled])

  // Toggle deafen: silencia / activa tots els elements d'àudio remots
  const toggleDeafen = useCallback(() => {
    const newDeafened = !isDeafenedRef.current
    isDeafenedRef.current = newDeafened
    audioElementsRef.current.forEach(el => {
      el.muted = newDeafened
    })
    setIsDeafened(newDeafened)
    updateParticipants()
  }, [updateParticipants])

  // Toggle càmera
  const toggleCamera = useCallback(async () => {
    try {
      if (localVideoTrackRef.current) {
        // Apagar càmera
        if (roomRef.current?.localParticipant) {
          await roomRef.current.localParticipant.unpublishTrack(localVideoTrackRef.current)
        }
        localVideoTrackRef.current.stop()
        localVideoTrackRef.current = null
        setLocalVideoTrack(null)
        isCameraOnRef.current = false
        setIsCameraOn(false)
      } else {
        // Encendre càmera
        const videoTrack = await createLocalVideoTrack()
        localVideoTrackRef.current = videoTrack
        if (roomRef.current?.localParticipant) {
          await roomRef.current.localParticipant.publishTrack(videoTrack)
        }
        setLocalVideoTrack(videoTrack)
        isCameraOnRef.current = true
        setIsCameraOn(true)
      }
      updateParticipants()
    } catch (e: any) {
      logger.error('Error toggle càmera:', e)
      setError(e.message || 'Error accedint a la càmera')
    }
  }, [updateParticipants])

  const toggleScreenShare = useCallback(async () => {
    try {
      if (!roomRef.current?.localParticipant) return

      if (localScreenTrackRef.current) {
        await roomRef.current.localParticipant.unpublishTrack(localScreenTrackRef.current)
        localScreenTrackRef.current.stop()
        localScreenTrackRef.current = null
        setLocalScreenTrack(null)
        isScreenSharingRef.current = false
        setIsScreenSharing(false)
      } else {
        const tracks = await captureScreenTracks()
        const videoTrack = tracks.find((t: any) => t.kind === 'video')
        if (!videoTrack) {
          throw new Error('No s\'ha pogut obtenir el track de pantalla')
        }
        localScreenTrackRef.current = videoTrack
        setLocalScreenTrack(videoTrack)
        await roomRef.current.localParticipant.publishTrack(videoTrack)
        isScreenSharingRef.current = true
        setIsScreenSharing(true)

        // Si l'usuari atura el share des del navegador, sincronitzem estat.
        videoTrack.mediaStreamTrack?.addEventListener('ended', async () => {
          try {
            if (roomRef.current?.localParticipant && localScreenTrackRef.current) {
              await roomRef.current.localParticipant.unpublishTrack(localScreenTrackRef.current)
            }
          } catch (_) {
            // ignore
          }
          try {
            localScreenTrackRef.current?.stop()
          } catch (_) {
            // ignore
          }
          localScreenTrackRef.current = null
          setLocalScreenTrack(null)
          isScreenSharingRef.current = false
          setIsScreenSharing(false)
        }, { once: true })
      }
    } catch (e: any) {
      logger.error('Error toggle screen share:', e)
      setError(e.message || 'Error compartint pantalla')
    }
  }, [])

  const setParticipantLocalMuted = useCallback((identity: string, muted: boolean) => {
    if (!roomRef.current) return
    for (const participant of roomRef.current.remoteParticipants.values()) {
      if (participant.identity === identity) {
        const el = audioElementsRef.current.get(participant.sid)
        if (el) el.muted = muted
        break
      }
    }
  }, [])

  const stopMediaFileShare = useCallback(async () => {
    try {
      if (localMediaFileVideoTrackRef.current) {
        if (roomRef.current?.localParticipant) {
          try { await roomRef.current.localParticipant.unpublishTrack(localMediaFileVideoTrackRef.current) } catch (_) {}
        }
        try { localMediaFileVideoTrackRef.current.stop() } catch (_) {}
        localMediaFileVideoTrackRef.current = null
        setLocalMediaFileTrack(null)
      }
      if (localMediaFileAudioTrackRef.current) {
        if (roomRef.current?.localParticipant) {
          try { await roomRef.current.localParticipant.unpublishTrack(localMediaFileAudioTrackRef.current) } catch (_) {}
        }
        try { localMediaFileAudioTrackRef.current.stop() } catch (_) {}
        localMediaFileAudioTrackRef.current = null
      }
      if (mediaFileElementRef.current) {
        mediaFileElementRef.current.pause()
        mediaFileElementRef.current.src = ''
        try { mediaFileElementRef.current.remove() } catch (_) {}
        mediaFileElementRef.current = null
      }
      if (mediaFileObjectUrlRef.current) {
        URL.revokeObjectURL(mediaFileObjectUrlRef.current)
        mediaFileObjectUrlRef.current = null
      }
    } catch (e: any) {
      logger.error('Error aturant media file share:', e)
    } finally {
      setIsMediaFileSharing(false)
      setMediaFileName(null)
    }
  }, [])

  const startMediaFileShare = useCallback(async (file: File) => {
    if (!roomRef.current?.localParticipant) return

    if (localMediaFileVideoTrackRef.current || mediaFileElementRef.current) {
      await stopMediaFileShare()
    }

    try {
      const objectUrl = URL.createObjectURL(file)
      mediaFileObjectUrlRef.current = objectUrl

      const el = document.createElement('video')
      el.src = objectUrl
      el.style.cssText = 'position:absolute;left:-9999px;top:-9999px;width:1px;height:1px;'
      el.muted = false
      document.body.appendChild(el)
      mediaFileElementRef.current = el

      await el.play()

      const stream = (el as any).captureStream() as MediaStream
      // In Electron the video track may appear asynchronously after play().
      const videoMediaTrack: MediaStreamTrack | undefined =
        stream.getVideoTracks()[0] ??
        await new Promise<MediaStreamTrack | undefined>(resolve => {
          const timer = setTimeout(() => resolve(undefined), 2000)
          stream.addEventListener('addtrack', (e: any) => {
            if (e.track.kind === 'video') { clearTimeout(timer); resolve(e.track) }
          })
        })
      const audioMediaTrack = stream.getAudioTracks()[0]

      if (videoMediaTrack) {
        const lvTrack = new LocalVideoTrack(videoMediaTrack, undefined, true)
        await roomRef.current.localParticipant.publishTrack(lvTrack)
        localMediaFileVideoTrackRef.current = lvTrack
        setLocalMediaFileTrack(lvTrack)
      }

      if (audioMediaTrack) {
        const laTrack = new LocalAudioTrack(audioMediaTrack, undefined, true)
        await roomRef.current.localParticipant.publishTrack(laTrack)
        localMediaFileAudioTrackRef.current = laTrack
      }

      setMediaFileName(file.name)
      setIsMediaFileSharing(true)

      el.addEventListener('ended', () => { void stopMediaFileShare() }, { once: true })
    } catch (e: any) {
      logger.error('Error iniciant media file share:', e)
      if (mediaFileElementRef.current) {
        try { mediaFileElementRef.current.pause() } catch (_) {}
        try { mediaFileElementRef.current.remove() } catch (_) {}
        mediaFileElementRef.current = null
      }
      if (mediaFileObjectUrlRef.current) {
        URL.revokeObjectURL(mediaFileObjectUrlRef.current)
        mediaFileObjectUrlRef.current = null
      }
    }
  }, [stopMediaFileShare])

  // Connectar a un canal de veu
  const connectToChannel = useCallback(async (
    channelId: string,
    channelName: string,
    options?: { encryptionType?: EncryptionType; channelKey?: Uint8Array | null }
  ) => {
    try {
      setError(null)

      // Evita re-publicar càmera/pantalla per estat antic quan fem switch de canal.
      let shouldRestoreCamera = isCameraOnRef.current
      let shouldRestoreScreenShare = isScreenSharingRef.current

      // Desconnectar si ja hi som
      if (roomRef.current) {
        shouldRestoreCamera = false
        shouldRestoreScreenShare = false
        isCameraOnRef.current = false
        isScreenSharingRef.current = false
        setIsCameraOn(false)
        setIsScreenSharing(false)
        roomRef.current.disconnect()
        roomRef.current = null
      }
      localAudioTrackRef.current = null

      // Obtenir token i URL efectiva del LiveKit des del backend
      const { token, url } = await fetchToken(channelId)
      const livekitUrl = url || __LIVEKIT_HOST__
      if (!livekitUrl) {
        throw new Error('LIVEKIT_HOST no està configurada')
      }

      // Aplicar preferència actual de so abans de subscriure nous tracks
      isDeafenedRef.current = isDeafened

      const wantsE2EE = options?.encryptionType && options.encryptionType !== 'none'
      if (wantsE2EE && !isE2EESupported()) {
        const inTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
        if (inTauri) {
          throw new Error(
            'E2EE de veu no és compatible amb l\'app d\'escriptori Linux (WebKitGTK no implementa RTCRtpScriptTransform). ' +
            'Usa el navegador web per a canals de veu xifrats.'
          )
        }
        throw new Error('El navegador no suporta E2EE de LiveKit')
      }
      const shouldEnableE2EE = !!wantsE2EE

      // Crear i connectar a la sala
      const roomOptions: any = {
        adaptiveStream: true,
        dynacast: false,
      }

      if (shouldEnableE2EE) {
        if (!options?.channelKey) {
          throw new Error('Falta la clau E2EE del canal de veu')
        }

        const keyProvider = new ExternalE2EEKeyProvider()
        await keyProvider.setKey(toArrayBuffer(options.channelKey))

        roomOptions.encryption = {
          keyProvider,
          worker: new E2EEWorker(),
        }
      }

      const room = new Room(roomOptions)

      roomRef.current = room

      // ── Esdeveniments de sala ─────────────────────

      room.on(RoomEvent.ParticipantConnected, (p: any) => {
        logger.debug('🎙 Participant connectat:', p.identity)
        updateParticipants()
      })

      room.on(RoomEvent.ParticipantDisconnected, (p: any) => {
        logger.debug('👋 Participant desconnectat:', p.identity)
        updateParticipants()
      })

      // TrackSubscribed(track, publication, participant)
      room.on(RoomEvent.TrackSubscribed, (track: any, publication: any, participant: any) => {
        logger.debug('📻 Track subscrit de:', participant.identity, 'kind:', track.kind)
        if (track.kind === 'audio') {
          attachRemoteAudio(track, participant.sid)
        } else if (track.kind === 'video') {
          const userId = participant.identity || participant.sid
          const prevTracks = remoteVideoTracksRef.current.get(userId) ?? []
          const nextTracks = prevTracks.some((t: any) => getTrackKey(t) === getTrackKey(track))
            ? prevTracks
            : [...prevTracks, track]

          remoteVideoTracksRef.current.set(userId, nextTracks)
          setRemoteVideoTracks(prev => ({ ...prev, [userId]: nextTracks }))
        }
        updateParticipants()
      })

      room.on(RoomEvent.TrackUnsubscribed, (track: any, publication: any, participant: any) => {
        logger.debug('📵 Track no subscrit de:', participant?.identity)
        if (track.kind === 'audio') {
          detachRemoteAudio(participant.sid)
        } else if (track.kind === 'video') {
          const userId = participant.identity || participant.sid
          const prevTracks = remoteVideoTracksRef.current.get(userId) ?? []
          const nextTracks = prevTracks.filter((t: any) => getTrackKey(t) !== getTrackKey(track))

          if (nextTracks.length === 0) {
            remoteVideoTracksRef.current.delete(userId)
            setRemoteVideoTracks(prev => {
              const next = { ...prev }
              delete next[userId]
              return next
            })
          } else {
            remoteVideoTracksRef.current.set(userId, nextTracks)
            setRemoteVideoTracks(prev => ({ ...prev, [userId]: nextTracks }))
          }
        }
        updateParticipants()
      })

      // TrackMuted(publication, participant)
      room.on(RoomEvent.TrackMuted, (publication: any, participant: any) => {
        if (participant.isLocal) {
          setIsMuted(true)
        }
        updateParticipants()
      })

      // TrackUnmuted(publication, participant)
      room.on(RoomEvent.TrackUnmuted, (publication: any, participant: any) => {
        if (participant.isLocal) {
          setIsMuted(false)
        }
      })

      // ActiveSpeakersChanged(speakers)
      room.on(RoomEvent.ActiveSpeakersChanged, (speakers: any[]) => {
        const speakingIds = new Set(speakers.map((s: any) => s.identity || s.sid))
        setParticipants(prev =>
          prev.map(p => ({
            ...p,
            isSpeaking: speakingIds.has(p.userId),
          }))
        )
      })

      room.on(RoomEvent.Disconnected, (reason: any) => {
        logger.info('🔌 Desconnectat de LiveKit:', reason)
        audioElementsRef.current.forEach(el => {
          el.srcObject = null
          el.remove()
        })
        audioElementsRef.current.clear()
        remoteVideoTracksRef.current.clear()
        setRemoteVideoTracks({})
        if (localVideoTrackRef.current) {
          localVideoTrackRef.current.stop()
          localVideoTrackRef.current = null
          setLocalVideoTrack(null)
        }
        if (localScreenTrackRef.current) {
          localScreenTrackRef.current.stop()
          localScreenTrackRef.current = null
          setLocalScreenTrack(null)
        }
        if (localMediaFileVideoTrackRef.current) {
          try { localMediaFileVideoTrackRef.current.stop() } catch (_) {}
          localMediaFileVideoTrackRef.current = null
          setLocalMediaFileTrack(null)
        }
        if (localMediaFileAudioTrackRef.current) {
          try { localMediaFileAudioTrackRef.current.stop() } catch (_) {}
          localMediaFileAudioTrackRef.current = null
        }
        if (mediaFileElementRef.current) {
          mediaFileElementRef.current.pause()
          mediaFileElementRef.current.src = ''
          try { mediaFileElementRef.current.remove() } catch (_) {}
          mediaFileElementRef.current = null
        }
        if (mediaFileObjectUrlRef.current) {
          URL.revokeObjectURL(mediaFileObjectUrlRef.current)
          mediaFileObjectUrlRef.current = null
        }
        setIsConnected(false)
        setIsPublishing(false)
        isCameraOnRef.current = false
        isScreenSharingRef.current = false
        setIsCameraOn(false)
        setIsScreenSharing(false)
        setIsMediaFileSharing(false)
        setMediaFileName(null)
        localAudioTrackRef.current = null
      })

      room.on(RoomEvent.ConnectionStateChanged, (state: string) => {
        logger.debug('📡 Connection state:', state)
        if (state === 'connected') {
          setIsConnected(true)
        }
      })

      room.on(RoomEvent.MediaDevicesError, (err: Error) => {
        logger.error('❌ Error de dispositius multimèdia:', err)
        setError(`Error accedint al micròfon: ${err.message}`)
      })

      // Connectar a la sala
      logger.info('🔗 Connectant a LiveKit:', room.name, 'a', livekitUrl)
      await room.connect(livekitUrl, token)

      if (shouldEnableE2EE) {
        await room.setE2EEEnabled(true)
      }

      logger.info('✅ Connectat a LiveKit:', room.name)

      // Aplicar estat per defecte/persistit del micro (OFF per defecte)
      await room.localParticipant?.setMicrophoneEnabled(!isMuted)
      setIsPublishing(!isMuted)

      // Aplicar estat per defecte/persistit de la càmera (OFF per defecte)
      if (shouldRestoreCamera) {
        const videoTrack = await createLocalVideoTrack()
        localVideoTrackRef.current = videoTrack
        await room.localParticipant?.publishTrack(videoTrack)
        setLocalVideoTrack(videoTrack)
      }

      // Aplicar estat per defecte/persistit de screen share
      if (shouldRestoreScreenShare) {
        const tracks = await captureScreenTracks()
        const screenTrack = tracks.find((t: any) => t.kind === 'video')
        if (screenTrack) {
          localScreenTrackRef.current = screenTrack
          setLocalScreenTrack(screenTrack)
          await room.localParticipant?.publishTrack(screenTrack)
        }
      }

      // Sincronitzar llista inicial (inclou participant local)
      updateParticipants()

    } catch (e: any) {
      logger.error('❌ Error connectant a LiveKit:', e)
      setError(e.message || 'Error connectant al canal de veu')

      // Netejar en cas d'error
      if (localAudioTrackRef.current) {
        try {
          localAudioTrackRef.current.stop()
        } catch (_) { /* ignore */ }
        localAudioTrackRef.current = null
      }
      if (roomRef.current) {
        roomRef.current.disconnect()
        roomRef.current = null
      }
      setIsConnected(false)
      setIsPublishing(false)
    }
  }, [fetchToken, getTrackKey, isCameraOn, isDeafened, isMuted, isScreenSharing, toArrayBuffer, updateParticipants, attachRemoteAudio, detachRemoteAudio])

  // Desconnectar
  const disconnect = useCallback(() => {
    if (localAudioTrackRef.current) {
      try {
        localAudioTrackRef.current.stop()
      } catch (_) { /* ignore */ }
      localAudioTrackRef.current = null
    }
    if (localVideoTrackRef.current) {
      try {
        localVideoTrackRef.current.stop()
      } catch (_) { /* ignore */ }
      localVideoTrackRef.current = null
      setLocalVideoTrack(null)
    }
    if (localScreenTrackRef.current) {
      try {
        localScreenTrackRef.current.stop()
      } catch (_) { /* ignore */ }
      localScreenTrackRef.current = null
      setLocalScreenTrack(null)
    }
    if (localMediaFileVideoTrackRef.current) {
      try { localMediaFileVideoTrackRef.current.stop() } catch (_) {}
      localMediaFileVideoTrackRef.current = null
      setLocalMediaFileTrack(null)
    }
    if (localMediaFileAudioTrackRef.current) {
      try { localMediaFileAudioTrackRef.current.stop() } catch (_) {}
      localMediaFileAudioTrackRef.current = null
    }
    if (mediaFileElementRef.current) {
      mediaFileElementRef.current.pause()
      mediaFileElementRef.current.src = ''
      try { mediaFileElementRef.current.remove() } catch (_) {}
      mediaFileElementRef.current = null
    }
    if (mediaFileObjectUrlRef.current) {
      URL.revokeObjectURL(mediaFileObjectUrlRef.current)
      mediaFileObjectUrlRef.current = null
    }
    audioElementsRef.current.forEach(el => {
      el.srcObject = null
      el.remove()
    })
    audioElementsRef.current.clear()
    remoteVideoTracksRef.current.clear()
    setRemoteVideoTracks({})
    if (roomRef.current) {
      roomRef.current.disconnect()
      roomRef.current = null
    }
    setIsConnected(false)
    setIsPublishing(false)
    isCameraOnRef.current = false
    isScreenSharingRef.current = false
    setIsCameraOn(false)
    setIsScreenSharing(false)
    setIsMediaFileSharing(false)
    setMediaFileName(null)
    setParticipants([])
  }, [])

  // Toggle mute/unmute
  const toggleMute = useCallback(async () => {
    if (!roomRef.current?.localParticipant) return

    try {
      const newMuted = !isMuted
      // LiveKit v2: setMicrophoneEnabled(true) = micro actiu, (false) = mutat
      await roomRef.current.localParticipant.setMicrophoneEnabled(!newMuted)
      setIsMuted(newMuted)
    } catch (e: any) {
      logger.error('Error toggle mute:', e)
      setError(e.message || 'Error mutejant el micròfon')
    }
  }, [isMuted])

  return {
    isConnected,
    isPublishing,
    isMuted,
    isDeafened,
    isCameraOn,
    isScreenSharing,
    isMediaFileSharing,
    localVideoTrack,
    localScreenTrack,
    localMediaFileTrack,
    mediaFileName,
    mediaFileElementRef,
    remoteVideoTracks,
    participants,
    connectToChannel,
    disconnect,
    toggleMute,
    toggleDeafen,
    toggleCamera,
    toggleScreenShare,
    startMediaFileShare,
    stopMediaFileShare,
    setParticipantLocalMuted,
    error,
  }
}
