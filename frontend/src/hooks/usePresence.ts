import { useEffect, useState } from 'react'
import { getSocket } from '../lib/socket'
import { VoiceParticipant } from '../types'

export function usePresence(selectedServer: string | null) {
  const [voicePresenceByChannel, setVoicePresenceByChannel] = useState<Record<string, VoiceParticipant[]>>({})
  const [serverMemberPresenceById, setServerMemberPresenceById] = useState<Record<string, boolean>>({})

  useEffect(() => {
    const socket = getSocket()

    const handleVoicePresenceUpdated = (data: { channelId: string; users: VoiceParticipant[] }) => {
      setVoicePresenceByChannel((prev) => ({
        ...prev,
        [data.channelId]: data.users ?? [],
      }))
    }

    const handleVoicePresenceSnapshot = (data: {
      serverId: string
      channels: Array<{ channelId: string; users: VoiceParticipant[] }>
    }) => {
      if (!selectedServer || data.serverId !== selectedServer) return
      const next: Record<string, VoiceParticipant[]> = {}
      for (const channel of data.channels ?? []) {
        next[channel.channelId] = channel.users ?? []
      }
      setVoicePresenceByChannel(next)
    }

    socket.on('voice-presence-updated', handleVoicePresenceUpdated)
    socket.on('voice-presence-snapshot', handleVoicePresenceSnapshot)

    return () => {
      socket.off('voice-presence-updated', handleVoicePresenceUpdated)
      socket.off('voice-presence-snapshot', handleVoicePresenceSnapshot)
    }
  }, [selectedServer])

  useEffect(() => {
    const socket = getSocket()

    const handleServerMemberPresenceUpdated = (payload: { serverId: string; userId: string; status: string }) => {
      if (!selectedServer || payload.serverId !== selectedServer) return
      setServerMemberPresenceById((current) => ({
        ...current,
        [payload.userId]: payload.status === 'online',
      }))
    }

    const handleServerMemberPresenceSnapshot = (payload: {
      serverId: string
      members: Array<{ userId: string; status: string }>
    }) => {
      if (!selectedServer || payload.serverId !== selectedServer) return
      const next: Record<string, boolean> = {}
      for (const member of payload.members ?? []) {
        next[member.userId] = member.status === 'online'
      }
      setServerMemberPresenceById(next)
    }

    socket.on('server-member-presence-updated', handleServerMemberPresenceUpdated)
    socket.on('server-member-presence-snapshot', handleServerMemberPresenceSnapshot)

    return () => {
      socket.off('server-member-presence-updated', handleServerMemberPresenceUpdated)
      socket.off('server-member-presence-snapshot', handleServerMemberPresenceSnapshot)
    }
  }, [selectedServer])

  useEffect(() => {
    if (!selectedServer) {
      setVoicePresenceByChannel({})
      setServerMemberPresenceById({})
      return
    }
    const socket = getSocket()
    setVoicePresenceByChannel({})
    setServerMemberPresenceById({})
    socket.emit('join-server-presence', { serverId: selectedServer })
    socket.emit('get-server-member-presence', { serverId: selectedServer })
    socket.emit('get-voice-presence', { serverId: selectedServer })
  }, [selectedServer])

  return { voicePresenceByChannel, serverMemberPresenceById }
}
