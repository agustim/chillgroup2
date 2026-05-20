/**
 * Hook per connectar amb LiveKit per a canals de veu reals.
 *
 * Gestiona:
 * - Obtenció de token des del backend
 * - Connexió a la sala LiveKit
 * - Publicar/subscriure tracks d'àudio
 * - Permís de micròfon
 */

import { useRef, useState, useCallback, useEffect } from 'react'
import {
  Room,
  RoomEvent,
  Track,
  LocalTrack,
  createLocalAudioTrack,
} from 'livekit-client'
import type { Participant, TrackSource } from 'livekit-client'
import type { VoiceConnection, VoiceParticipant } from '../types'
import { getToken } from '../lib/api'

// Nom del sala LiveKit (unifiquem amb un prefix per evitar col·lisions)
const LIVEKIT_ROOM_PREFIX = 'chillgroup-'

interface UseLiveKitResult {
  /** Estem connectats a LiveKit */
  isConnected: boolean
  /** Estem pujant el nostre àudio */
  isPublishing: boolean
  /** Estem mutejats */
  isMuted: boolean
  /** Participants reals de la sala */
  participants: VoiceParticipant[]
  /** Connectar a un canal de veu */
  connectToChannel: (channelId: string, channelName: string) => Promise<void>
  /** Desconnectar del canal */
  disconnect: () => void
  /** Toggle mute */
  toggleMute: () => Promise<void>
  /** Error de connexió */
  error: string | null
}

export function useLiveKit(): UseLiveKitResult {
  const roomRef = useRef<Room | null>(null)
  const [isConnected, setIsConnected] = useState(false)
  const [isPublishing, setIsPublishing] = useState(false)
  const [isMuted, setIsMuted] = useState(false)
  const [participants, setParticipants] = useState<VoiceParticipant[]>([])
  const [error, setError] = useState<string | null>(null)
  const localAudioTrackRef = useRef<LocalTrack | null>(null)

  // Netejar quan es desmunta
  useEffect(() => {
    return () => {
      roomRef.current?.disconnect()
      roomRef.current = null
    }
  }, [])

  // Obtener token del backend
  const fetchToken = useCallback(async (channelId: string): Promise<string> => {
    const token = getToken()
    if (!token) throw new Error('No estàs autenticat')

    const response = await fetch(`/api/livekit/token`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({
        room: LIVEKIT_ROOM_PREFIX + channelId,
        participant: token.substring(0, 20), // ID simplificat com a participant
      }),
    })

    if (!response.ok) {
      const text = await response.text()
      throw new Error(`Error obtenint token LiveKit: ${response.status} ${text}`)
    }

    const data = await response.json()
    return data.token
  }, [])

  // Mapejar participants de LiveKit als nostres VoiceParticipant
  const mapParticipants = useCallback((pkParticipants: Participant[]): VoiceParticipant[] => {
    return Array.from(pkParticipants)
      .filter(p => !p.isLocal)
      .map(p => {
        // Trobar track d'àudio
        const audioTrack = p.getTrack(TrackSource.SOURCE_MICROPHONE)
        return {
          userId: p.identity || p.sid,
          username: p.identity || 'Desconegut',
          isSpeaking: audioTrack?.isMuted !== true && p.isSpeaking,
          isDeafened: false,
          isSuppressed: false,
          joinedAt: new Date().toISOString(),
        }
      })
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

      // Obtenir URL del LiveKit (del backend o configurar)
      const livekitUrl = import.meta.env.VITE_LIVEKIT_URL
      if (!livekitUrl) {
        throw new Error('VITE_LIVEKIT_URL no està configurada')
      }

      // Obtenir token
      const token = await fetchToken(channelId)

      // Crear i connectar a la sala
      const room = new Room({
        // Si l'àudio falla, no reconnectar automàticament
        reconnect: true,
        // Publish tracks després de reconectar
        publishOnJoin: { audio: true, video: false },
      })

      // Esdeveniments de sala
      room.on(RoomEvent.ParticipantConnected, (participant: Participant) => {
        console.log('🎙 Participant connectat:', participant.identity)
        setParticipants(prev => {
          const existing = prev.find(p => p.userId === (participant.identity || participant.sid))
          if (existing) return prev
          return [...prev, {
            userId: participant.identity || participant.sid,
            username: participant.identity || 'Desconegut',
            isSpeaking: false,
            isDeafened: false,
            isSuppressed: false,
            joinedAt: new Date().toISOString(),
          }]
        })
      })

      room.on(RoomEvent.ParticipantDisconnected, (participant: Participant) => {
        console.log('👋 Participant desconnectat:', participant.identity)
        setParticipants(prev =>
          prev.filter(p => p.userId !== (participant.identity || participant.sid))
        )
      })

      room.on(RoomEvent.TrackSubscribed, (_track: Track, publication: any, participant: Participant) => {
        console.log('📻 Track subscrit de:', participant.identity)
        // Actualitzar isSpeaking basat en el track
        setParticipants(prev =>
          prev.map(p =>
            p.userId === (participant.identity || participant.sid)
              ? { ...p, isSpeaking: !publication.trackMuted }
              : p
          )
        )
      })

      room.on(RoomEvent.TrackMuted, (_track: Track, participant: Participant) => {
        if (participant.isLocal) {
          setIsMuted(true)
        }
        setParticipants(prev =>
          prev.map(p =>
            p.userId === (participant.identity || participant.sid)
              ? { ...p, isSpeaking: false }
              : p
          )
        )
      })

      room.on(RoomEvent.TrackUnmuted, (_track: Track, participant: Participant) => {
        if (participant.isLocal) {
          setIsMuted(false)
        }
      })

      room.on(RoomEvent.SpeakingChanged, (speaking: Participant) => {
        setParticipants(prev =>
          prev.map(p =>
            p.userId === (speaking.identity || speaking.sid)
              ? { ...p, isSpeaking: speaking.isSpeaking }
              : p
          )
        )
      })

      room.on(RoomEvent.Disconnected, (reason?: string) => {
        console.log('🔌 Desconnectat de LiveKit:', reason)
        setIsConnected(false)
        setIsPublishing(false)
        localAudioTrackRef.current?.stop()
        localAudioTrackRef.current = null
      })

      room.on(RoomEvent.RoomConnectionFailed, (error) => {
        console.error('❌ Error de connexió LiveKit:', error)
        setError(`No s'ha pogut connectar al canal de veu: ${error.message || error}`)
      })

      room.on(RoomEvent.AudioPlaying, () => {
        console.log('🔊 Àudio en reproducció')
      })

      room.on(RoomEvent.VideoPlaying, () => {
        console.log('📹 Vídeo en reproducció')
      })

      roomRef.current = room

      // Connectar a la sala
      await room.connect(livekitUrl, token)
      console.log('✅ Connectat a LiveKit:', room.name)
      setIsConnected(true)

      // Publicar àudio del micròfon
      const audioTrack = await createLocalAudioTrack()
      await room.localParticipant?.publishTrack(audioTrack)
      localAudioTrackRef.current = audioTrack
      setIsPublishing(true)
      console.log('🎤 Àudio publicat')

    } catch (e: any) {
      console.error('Error connectant a LiveKit:', e)
      setError(e.message || 'Error connectant al canal de veu')
      // Netejar
      if (roomRef.current) {
        roomRef.current.disconnect()
        roomRef.current = null
      }
      setIsConnected(false)
      setIsPublishing(false)
    }
  }, [fetchToken])

  // Desconnectar
  const disconnect = useCallback(() => {
    if (localAudioTrackRef.current) {
      localAudioTrackRef.current.stop()
      localAudioTrackRef.current = null
    }
    if (roomRef.current) {
      roomRef.current.disconnect()
      roomRef.current = null
    }
    setIsConnected(false)
    setIsPublishing(false)
    setIsMuted(false)
    setParticipants([])
  }, [])

  // Toggle mute
  const toggleMute = useCallback(async () => {
    if (!roomRef.current || !localAudioTrackRef.current) return

    try {
      if (isMuted) {
        await localAudioTrackRef.current.enable()
      } else {
        await localAudioTrackRef.current.disable()
      }
      setIsMuted(!isMuted)
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
