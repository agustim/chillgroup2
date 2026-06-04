import React, { useEffect, useState } from 'react'
import {
  channelGetExplicitPermissions,
  channelGetPermissions,
  channelSetExplicitPermission,
  channelsUpdate,
} from '../lib/api'
import { Channel } from '../types'

export type PermissionLevel = 'none' | 'read' | 'write' | 'manage'

export interface ChannelExplicitPermission {
  userId: string
  username: string
  permissionLevel: number
  permission: PermissionLevel
}

export interface ChannelPermissionRow {
  userId: string
  username: string
  effectiveLevel: number
  effectivePermission: PermissionLevel
  explicitLevel: number | null
}

interface UseChannelConfigParams {
  isActive: boolean
  channel: Channel | null
  selectedServer: string | null
  fetchChannels: (serverId: string) => Promise<void>
  setSelectedChannel: React.Dispatch<React.SetStateAction<Channel | null>>
  setFeedback: (msg: string | null) => void
}

export function useChannelConfig({
  isActive,
  channel,
  selectedServer,
  fetchChannels,
  setSelectedChannel,
  setFeedback,
}: UseChannelConfigParams) {
  const [channelConfigName, setChannelConfigName] = useState('')
  const [channelConfigMessageTTL, setChannelConfigMessageTTL] = useState('')
  const [channelConfigIsPrivate, setChannelConfigIsPrivate] = useState(false)
  const [channelExplicitPermissions, setChannelExplicitPermissions] = useState<ChannelExplicitPermission[]>([])
  const [channelExplicitPermissionsLoading, setChannelExplicitPermissionsLoading] = useState(false)
  const [canViewChannelExplicitPermissions, setCanViewChannelExplicitPermissions] = useState(false)
  const [channelPermissionRows, setChannelPermissionRows] = useState<ChannelPermissionRow[]>([])
  const [updatingChannelPermissionUserId, setUpdatingChannelPermissionUserId] = useState<string | null>(null)

  useEffect(() => {
    if (!isActive || !channel) return
    setChannelConfigName(channel.name)
    setChannelConfigMessageTTL(
      channel.messageTTL === null || channel.messageTTL === undefined
        ? ''
        : String(channel.messageTTL)
    )
    setChannelConfigIsPrivate(!!channel.isPrivate)
  }, [isActive, channel?.channelId, channel?.name, channel?.messageTTL, channel?.isPrivate])

  useEffect(() => {
    if (!isActive || !channel) {
      setChannelExplicitPermissions([])
      setCanViewChannelExplicitPermissions(false)
      setChannelExplicitPermissionsLoading(false)
      return
    }

    let cancelled = false
    const load = async () => {
      setChannelExplicitPermissionsLoading(true)
      const result = await channelGetExplicitPermissions(channel.channelId)
      if (cancelled) return

      if (result.success) {
        setCanViewChannelExplicitPermissions(true)
        setChannelExplicitPermissions(result.data)
      } else {
        setCanViewChannelExplicitPermissions(false)
        setChannelExplicitPermissions([])
      }
      setChannelExplicitPermissionsLoading(false)
    }

    void load()
    return () => { cancelled = true }
  }, [isActive, channel?.channelId])

  useEffect(() => {
    if (!isActive || !channel) {
      setChannelPermissionRows([])
      return
    }

    let cancelled = false
    const load = async () => {
      const [effectiveResult, explicitResult] = await Promise.all([
        channelGetPermissions(channel.channelId),
        channelGetExplicitPermissions(channel.channelId),
      ])

      if (cancelled) return
      if (!effectiveResult.success) {
        setChannelPermissionRows([])
        return
      }

      const explicitMap = new Map<string, number>()
      if (explicitResult.success) {
        for (const entry of explicitResult.data) {
          explicitMap.set(entry.userId, entry.permissionLevel)
        }
      }

      setChannelPermissionRows(
        effectiveResult.data.map((entry) => ({
          userId: entry.userId,
          username: entry.username,
          effectiveLevel: entry.permissionLevel,
          effectivePermission: entry.permission,
          explicitLevel: explicitMap.get(entry.userId) ?? null,
        }))
      )
    }

    void load()
    return () => { cancelled = true }
  }, [isActive, channel?.channelId, channelExplicitPermissions])

  const handleUpdateChannelExplicitPermission = async (userId: string, value: string) => {
    if (!channel) return

    setUpdatingChannelPermissionUserId(userId)
    const nextLevel = value === 'inherited' ? null : Number(value)
    const result = await channelSetExplicitPermission(channel.channelId, userId, nextLevel)
    setUpdatingChannelPermissionUserId(null)

    if (!result.success) {
      setFeedback(result.error.message)
      return
    }

    const explicitResult = await channelGetExplicitPermissions(channel.channelId)
    if (!explicitResult.success) {
      setFeedback(explicitResult.error.message)
      return
    }
    setChannelExplicitPermissions(explicitResult.data)
    setFeedback('Permís del canal actualitzat')
  }

  const handleChannelConfigSave = async (event: React.FormEvent) => {
    event.preventDefault()
    if (!channel) return

    const trimmedName = channelConfigName.trim()
    if (!trimmedName) {
      setFeedback('El nom del canal és obligatori')
      return
    }

    const ttlRaw = channelConfigMessageTTL.trim()
    let parsedTtl: number | null = null
    if (ttlRaw) {
      const value = Number(ttlRaw)
      if (Number.isNaN(value) || value < 0) {
        setFeedback('TTL ha de ser un número positiu o buit')
        return
      }
      parsedTtl = value
    }

    const result = await channelsUpdate(channel.channelId, trimmedName, parsedTtl, channelConfigIsPrivate)
    if (result.success) {
      if (selectedServer) await fetchChannels(selectedServer)
      setSelectedChannel((current) =>
        current ? { ...current, name: trimmedName, messageTTL: parsedTtl, isPrivate: channelConfigIsPrivate } : current
      )
      setFeedback(`Canal "${trimmedName}" actualitzat`)
    } else {
      setFeedback(result.error.message)
    }
  }

  return {
    channelConfigName,
    setChannelConfigName,
    channelConfigMessageTTL,
    setChannelConfigMessageTTL,
    channelConfigIsPrivate,
    setChannelConfigIsPrivate,
    channelExplicitPermissions,
    channelExplicitPermissionsLoading,
    canViewChannelExplicitPermissions,
    channelPermissionRows,
    updatingChannelPermissionUserId,
    handleUpdateChannelExplicitPermission,
    handleChannelConfigSave,
  }
}
