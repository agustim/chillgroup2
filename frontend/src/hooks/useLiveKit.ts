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
  createLocalScreenTracks,
  createLocalVideoTrack,
} from 'livekit-client'
import { logger } from '../lib/logger'
import type { VoiceParticipant } from '../types'

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
  /** Track de vídeo local (per previsualitzar) */
  localVideoTrack: any | null
  /** Track local de compartir pantalla */
  localScreenTrack: any | null
  /** Tracks de vídeo remots (identity -> track) */
  remoteVideoTracks: Record<string, any[]>
  /** Participants remots a la sala */
  participants: VoiceParticipant[]
  /** Connectar a un canal de veu */
  connectToChannel: (channelId: string, channelName: string) => Promise<void>
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
  const roomRef = useRef<Room | null>(null)
  const localAudioTrackRef = useRef<any>(null)
  const localVideoTrackRef = useRef<any>(null)
  const localScreenTrackRef = useRef<any>(null)
  const audioElementsRef = useRef<Map<string, HTMLAudioElement>>(new Map())
  const remoteVideoTracksRef = useRef<Map<string, any[]>>(new Map())
  const isDeafenedRef = useRef(false)
  const [isConnected, setIsConnected] = useState(false)
  const [isPublishing, setIsPublishing] = useState(false)
  const [isMuted, setIsMuted] = useState(true)
  const [isDeafened, setIsDeafened] = useState(false)
  const [isCameraOn, setIsCameraOn] = useState(false)
  const [isScreenSharing, setIsScreenSharing] = useState(false)
  const [localVideoTrack, setLocalVideoTrack] = useState<any>(null)
  const [localScreenTrack, setLocalScreenTrack] = useState<any>(null)
  const [remoteVideoTracks, setRemoteVideoTracks] = useState<Record<string, any[]>>({})
  const [participants, setParticipants] = useState<VoiceParticipant[]>([])
  const [error, setError] = useState<string | null>(null)

  // Netejar quan es desmunta
  useEffect(() => {
    return () => {
      // Eliminar tots els elements d'àudio remots
      audioElementsRef.current.forEach(el => {
        el.srcObject = null
        el.remove()
      })
      audioElementsRef.current.clear()
      roomRef.current?.disconnect()
      roomRef.current = null
      localAudioTrackRef.current = null
    }
  }, [])

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
  const fetchToken = useCallback(async (channelId: string): Promise<string> => {
    const token = getJwtToken()
    if (!token) throw new Error('No estàs autenticat')

    const response = await fetch(`/api/livekit/token`, {
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
    return data.token
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
        setIsCameraOn(false)
      } else {
        // Encendre càmera
        const videoTrack = await createLocalVideoTrack()
        localVideoTrackRef.current = videoTrack
        if (roomRef.current?.localParticipant) {
          await roomRef.current.localParticipant.publishTrack(videoTrack)
        }
        setLocalVideoTrack(videoTrack)
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
        setIsScreenSharing(false)
      } else {
        const tracks = await createLocalScreenTracks({ audio: false })
        const videoTrack = tracks.find((t: any) => t.kind === 'video')
        if (!videoTrack) {
          throw new Error('No s\'ha pogut obtenir el track de pantalla')
        }
        localScreenTrackRef.current = videoTrack
        setLocalScreenTrack(videoTrack)
        await roomRef.current.localParticipant.publishTrack(videoTrack)
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
          setIsScreenSharing(false)
        }, { once: true })
      }
    } catch (e: any) {
      logger.error('Error toggle screen share:', e)
      setError(e.message || 'Error compartint pantalla')
    }
  }, [])

  // Connectar a un canal de veu
  const connectToChannel = useCallback(async (channelId: string, channelName: string) => {
    try {
      setError(null)

      // Desconnectar si ja hi som
      if (roomRef.current) {
        roomRef.current.disconnect()
        roomRef.current = null
      }
      localAudioTrackRef.current = null

      // Obtenir URL del LiveKit (carregada des del .env de l'arrel)
      const livekitUrl = import.meta.env.VITE_LIVEKIT_URL
      if (!livekitUrl) {
        throw new Error('VITE_LIVEKIT_URL no està configurada')
      }

      // Obtenir token del backend
      const token = await fetchToken(channelId)

      // Aplicar preferència actual de so abans de subscriure nous tracks
      isDeafenedRef.current = isDeafened

      // Crear i connectar a la sala
      const room = new Room({
        adaptiveStream: true,
        dynacast: false,
      })

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
        setIsConnected(false)
        setIsPublishing(false)
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
      logger.info('✅ Connectat a LiveKit:', room.name)

      // Aplicar estat per defecte/persistit del micro (OFF per defecte)
      await room.localParticipant?.setMicrophoneEnabled(!isMuted)
      setIsPublishing(!isMuted)

      // Aplicar estat per defecte/persistit de la càmera (OFF per defecte)
      if (isCameraOn) {
        const videoTrack = await createLocalVideoTrack()
        localVideoTrackRef.current = videoTrack
        await room.localParticipant?.publishTrack(videoTrack)
        setLocalVideoTrack(videoTrack)
      }

      // Aplicar estat per defecte/persistit de screen share
      if (isScreenSharing) {
        const tracks = await createLocalScreenTracks({ audio: false })
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
  }, [fetchToken, getTrackKey, isCameraOn, isDeafened, isMuted, isScreenSharing, updateParticipants, attachRemoteAudio, detachRemoteAudio])

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
    localVideoTrack,
    localScreenTrack,
    remoteVideoTracks,
    participants,
    connectToChannel,
    disconnect,
    toggleMute,
    toggleDeafen,
    toggleCamera,
    toggleScreenShare,
    error,
  }
}
