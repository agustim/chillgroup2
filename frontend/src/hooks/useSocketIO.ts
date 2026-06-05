import { useEffect, useRef } from 'react'
import { Message } from '../types'
import { getSocket } from '../lib/socket'

interface SocketMessage {
  messageId: string
  channelId: string
  senderUserId: string
  senderUsername: string | null
  senderDeviceId: string
  encryptedPayload: string
  iv: string
  attachmentIds?: string[] | null
  keyVersion?: number | null
  timestamp: string
  expiresAt?: string | null
  editedAt: string | null
  deletedAt: string | null
}

interface UseSocketIOOptions {
  channelId: string | null
  onMessage: (message: Message) => void
  onUnreadUpdated?: (channelId: string, unreadCount: number) => void
  onMessagesExpired?: (channelId: string, messageIds: string[]) => void
}

export function useSocketIO({ channelId, onMessage, onUnreadUpdated, onMessagesExpired }: UseSocketIOOptions): void {
  const prevChannelIdRef = useRef<string | null>(null)
  const onMessageRef = useRef(onMessage)
  const onUnreadUpdatedRef = useRef(onUnreadUpdated)
  const onMessagesExpiredRef = useRef(onMessagesExpired)

  // Mantenir la referència actualitzada sense re-subscriure
  useEffect(() => {
    onMessageRef.current = onMessage
  })

  useEffect(() => {
    onUnreadUpdatedRef.current = onUnreadUpdated
  }, [onUnreadUpdated])

  useEffect(() => {
    onMessagesExpiredRef.current = onMessagesExpired
  }, [onMessagesExpired])

  useEffect(() => {
    const socket = getSocket()

    const handleMessage = (data: SocketMessage) => {
      const message: Message = {
        messageId: data.messageId,
        channelId: data.channelId,
        senderUserId: data.senderUserId,
        senderUsername: data.senderUsername ?? '',
        senderDeviceId: data.senderDeviceId,
        encryptedPayload: data.encryptedPayload,
        iv: data.iv,
        attachmentIds: data.attachmentIds ?? [],
        keyVersion: data.keyVersion ?? null,
        timestamp: data.timestamp,
        expiresAt: data.expiresAt ?? null,
        editedAt: data.editedAt,
        deletedAt: data.deletedAt,
      }
      onMessageRef.current(message)

      // Si estem al canal actiu, el marquem com llegit automàticament.
      if (channelId && data.channelId === channelId) {
        socket.emit('channel-read', { channelId })
      }
    }

    const handleUnreadUpdated = (data: { channelId: string; unreadCount: number }) => {
      onUnreadUpdatedRef.current?.(data.channelId, data.unreadCount)
    }

    const handleMessagesExpired = (data: { channelId: string; messageIds: string[] }) => {
      onMessagesExpiredRef.current?.(data.channelId, data.messageIds)
    }

    socket.on('message', handleMessage)
    socket.on('unread-updated', handleUnreadUpdated)
    socket.on('messages-expired', handleMessagesExpired)

    return () => {
      socket.off('message', handleMessage)
      socket.off('unread-updated', handleUnreadUpdated)
      socket.off('messages-expired', handleMessagesExpired)
    }
  }, [channelId])

  // Gestionar join/leave del canal
  useEffect(() => {
    const socket = getSocket()
    const prev = prevChannelIdRef.current

    const joinCurrentChannel = () => {
      if (channelId) {
        socket.emit('join-channel', { channelId })
      }
    }

    if (prev && prev !== channelId) {
      socket.emit('leave-channel', { channelId: prev })
    }

    // Si ja està connectat, fem join immediat; si reconnecta més tard,
    // el handler de "connect" re-farà el join automàticament.
    if (socket.connected) {
      joinCurrentChannel()
    }
    socket.on('connect', joinCurrentChannel)

    if (channelId) {
      socket.emit('channel-read', { channelId })
    }

    prevChannelIdRef.current = channelId

    return () => {
      socket.off('connect', joinCurrentChannel)
      if (channelId) {
        socket.emit('leave-channel', { channelId })
      }
    }
  }, [channelId])

}
