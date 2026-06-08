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
  onMessageDeleted?: (messageId: string, channelId: string) => void
  onMessageEdited?: (message: Message) => void
}

export function useSocketIO({ channelId, onMessage, onUnreadUpdated, onMessagesExpired, onMessageDeleted, onMessageEdited }: UseSocketIOOptions): void {
  const prevChannelIdRef = useRef<string | null>(null)
  const onMessageRef = useRef(onMessage)
  const onUnreadUpdatedRef = useRef(onUnreadUpdated)
  const onMessagesExpiredRef = useRef(onMessagesExpired)
  const onMessageDeletedRef = useRef(onMessageDeleted)
  const onMessageEditedRef = useRef(onMessageEdited)

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
    onMessageDeletedRef.current = onMessageDeleted
  }, [onMessageDeleted])

  useEffect(() => {
    onMessageEditedRef.current = onMessageEdited
  }, [onMessageEdited])

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

    const handleMessageDeleted = (data: { messageId: string; channelId: string }) => {
      onMessageDeletedRef.current?.(data.messageId, data.channelId)
    }

    const handleMessageEdited = (data: SocketMessage & { replyToMessageId?: string | null }) => {
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
      onMessageEditedRef.current?.(message)
    }

    socket.on('message', handleMessage)
    socket.on('unread-updated', handleUnreadUpdated)
    socket.on('messages-expired', handleMessagesExpired)
    socket.on('message-deleted', handleMessageDeleted)
    socket.on('message-edited', handleMessageEdited)

    return () => {
      socket.off('message', handleMessage)
      socket.off('unread-updated', handleUnreadUpdated)
      socket.off('messages-expired', handleMessagesExpired)
      socket.off('message-deleted', handleMessageDeleted)
      socket.off('message-edited', handleMessageEdited)
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
