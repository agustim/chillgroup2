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
  createLocalAudioTrack,
} from 'livekit-client'
import type { VoiceParticipant } from '../types'

const LIVEKIT_ROOM_PREFIX = 'chillgroup-'

interface UseLiveKitResult {
  /** Connexió estableerta amb LiveKit */
  isConnected: boolean
  /** Pujant el nostre àudio a la sala */
  isPublishing: boolean
  /** Estem mutejats (micròfon apagat) */
  isMuted: boolean
  /** Participants remots a la sala */
  participants: VoiceParticipant[]
  /** Connectar a un canal de veu */
  connectToChannel: (channelId: string, channelName: string) => Promise<void>
  /** Desconnectar del canal */
  disconnect: () => void
  /** Toggle mute/unmute */
  toggleMute: () => Promise<void>
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
  const audioElementsRef = useRef<Map<string, HTMLAudioElement>>(new Map())
  const [isConnected, setIsConnected] = useState(false)
  const [isPublishing, setIsPublishing] = useState(false)
  const [isMuted, setIsMuted] = useState(false)
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
    document.body.appendChild(audioEl)
    audioElementsRef.current.set(participantSid, audioEl)
    console.log('🔊 Àudio adjuntat per:', participantSid)
  }, [])

  // Eliminar l'element d'àudio d'un participant
  const detachRemoteAudio = useCallback((participantSid: string) => {
    const el = audioElementsRef.current.get(participantSid)
    if (el) {
      el.srcObject = null
      el.remove()
      audioElementsRef.current.delete(participantSid)
      console.log('🔇 Àudio eliminat per:', participantSid)
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

  // Actualitzar la llista de participants
  const updateParticipants = useCallback(() => {
    if (!roomRef.current) return
    const room = roomRef.current
    const remoteParts = room.remoteParticipants
    const parts: VoiceParticipant[] = []

    const localPart = room.localParticipant
    if (localPart) {
      const localAudioPub = (localPart as any).getTrackPublication('audio')
      const localHasAudio = !!(localAudioPub && !localAudioPub.isMuted)
      parts.push({
        userId: localPart.identity || localPart.sid,
        username: localPart.name || localPart.identity || 'Tu',
        isSpeaking: localPart.isSpeaking,
        isDeafened: false,
        isSuppressed: !localHasAudio,
        joinedAt: new Date().toISOString(),
      })
    }

    for (const p of remoteParts.values()) {
      const audioPub = (p as any).getTrackPublication('audio')
      const hasAudio = !!(audioPub && audioPub.isSubscribed)
      parts.push({
        userId: p.identity || p.sid,
        username: p.name || p.identity || 'Desconegut',
        isSpeaking: p.isSpeaking,
        isDeafened: false,
        isSuppressed: !hasAudio,
        joinedAt: new Date().toISOString(),
      })
    }
    setParticipants(parts)
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

      // Crear i connectar a la sala
      const room = new Room({
        adaptiveStream: true,
        dynacast: false,
      })

      roomRef.current = room

      // ── Esdeveniments de sala ─────────────────────

      room.on(RoomEvent.ParticipantConnected, (p: any) => {
        console.log('🎙 Participant connectat:', p.identity)
        updateParticipants()
      })

      room.on(RoomEvent.ParticipantDisconnected, (p: any) => {
        console.log('👋 Participant desconnectat:', p.identity)
        updateParticipants()
      })

      // TrackSubscribed(track, publication, participant)
      room.on(RoomEvent.TrackSubscribed, (track: any, publication: any, participant: any) => {
        console.log('📻 Track subscrit de:', participant.identity, 'kind:', track.kind)
        if (track.kind === 'audio') {
          attachRemoteAudio(track, participant.sid)
        }
        updateParticipants()
      })

      room.on(RoomEvent.TrackUnsubscribed, (track: any, publication: any, participant: any) => {
        console.log('📵 Track no subscrit de:', participant?.identity)
        if (track.kind === 'audio') {
          detachRemoteAudio(participant.sid)
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
        console.log('🔌 Desconnectat de LiveKit:', reason)
        audioElementsRef.current.forEach(el => {
          el.srcObject = null
          el.remove()
        })
        audioElementsRef.current.clear()
        setIsConnected(false)
        setIsPublishing(false)
        localAudioTrackRef.current = null
      })

      room.on(RoomEvent.ConnectionStateChanged, (state: string) => {
        console.log('📡 Connection state:', state)
        if (state === 'connected') {
          setIsConnected(true)
        }
      })

      room.on(RoomEvent.MediaDevicesError, (err: Error) => {
        console.error('❌ Error de dispositius multimèdia:', err)
        setError(`Error accedint al micròfon: ${err.message}`)
      })

      // Connectar a la sala
      console.log('🔗 Connectant a LiveKit:', room.name, 'a', livekitUrl)
      await room.connect(livekitUrl, token)
      console.log('✅ Connectat a LiveKit:', room.name)

      // Sincronitzar llista inicial (inclou participant local)
      updateParticipants()

      // Publicar àudio del micròfon (pedirà permís automàticament)
      console.log('🎤 Creant track d\'àudio...')
      const audioTrack = await createLocalAudioTrack()
      localAudioTrackRef.current = audioTrack
      await room.localParticipant?.publishTrack(audioTrack)
      setIsPublishing(true)
      console.log('🎤 Àudio publicat amb èxit')
      updateParticipants()

    } catch (e: any) {
      console.error('❌ Error connectant a LiveKit:', e)
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
  }, [fetchToken, updateParticipants, attachRemoteAudio, detachRemoteAudio])

  // Desconnectar
  const disconnect = useCallback(() => {
    if (localAudioTrackRef.current) {
      try {
        localAudioTrackRef.current.stop()
      } catch (_) { /* ignore */ }
      localAudioTrackRef.current = null
    }
    audioElementsRef.current.forEach(el => {
      el.srcObject = null
      el.remove()
    })
    audioElementsRef.current.clear()
    if (roomRef.current) {
      roomRef.current.disconnect()
      roomRef.current = null
    }
    setIsConnected(false)
    setIsPublishing(false)
    setIsMuted(false)
    setParticipants([])
  }, [])

  // Toggle mute/unmute
  const toggleMute = useCallback(async () => {
    if (!localAudioTrackRef.current) return

    try {
      const newMuted = !isMuted
      localAudioTrackRef.current.setMuted(newMuted)
      setIsMuted(newMuted)
    } catch (e: any) {
      console.error('Error toggle mute:', e)
      setError(e.message || 'Error mutejant el micròfon')
    }
  }, [isMuted])

  return {
    isConnected,
    isPublishing,
    isMuted,
    participants,
    connectToChannel,
    disconnect,
    toggleMute,
    error,
  }
}
