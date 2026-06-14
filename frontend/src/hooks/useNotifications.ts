import { useEffect, useRef, useState, useCallback } from 'react'

const PREF_KEY = 'notifications_enabled'

function isElectron(): boolean {
  return typeof window !== 'undefined' && 'electronAPI' in window
}

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

async function invokeTauri(cmd: string, args?: Record<string, unknown>): Promise<void> {
  const { invoke } = await import('@tauri-apps/api/core')
  await invoke(cmd, args)
}

export interface UseNotificationsReturn {
  notificationsEnabled: boolean
  notificationPermission: NotificationPermission | 'unsupported'
  toggleNotifications: () => Promise<void>
  fireNotification: (title: string, body: string) => void
  clearNotification: () => void
}

export function useNotifications(): UseNotificationsReturn {
  const [enabled, setEnabled] = useState<boolean>(() => {
    try { return localStorage.getItem(PREF_KEY) !== 'false' } catch { return true }
  })
  const [permission, setPermission] = useState<NotificationPermission | 'unsupported'>(() => {
    if (typeof Notification === 'undefined') return 'unsupported'
    return Notification.permission
  })

  const enabledRef = useRef(enabled)
  const permissionRef = useRef(permission)
  useEffect(() => { enabledRef.current = enabled }, [enabled])
  useEffect(() => { permissionRef.current = permission }, [permission])

  const clearNotification = useCallback(() => {
    if (isElectron()) {
      ;(window as any).electronAPI?.clearNotification?.()
    } else if (isTauri()) {
      invokeTauri('set_tray_notification', { hasNotification: false }).catch(() => {})
    }
  }, [])

  const clearRef = useRef(clearNotification)
  useEffect(() => { clearRef.current = clearNotification }, [clearNotification])

  const fireNotification = useCallback((title: string, body: string) => {
    if (!enabledRef.current) return
    if (isElectron()) {
      ;(window as any).electronAPI?.notify?.(title, body)
    } else if (isTauri()) {
      invokeTauri('notify', { title, body }).catch(() => {})
    } else if (typeof Notification !== 'undefined' && permissionRef.current === 'granted') {
      new Notification(title, { body })
    }
  }, [])

  const toggleNotifications = useCallback(async () => {
    const next = !enabled
    setEnabled(next)
    try { localStorage.setItem(PREF_KEY, String(next)) } catch {}

    if (next && typeof Notification !== 'undefined' && Notification.permission === 'default') {
      const result = await Notification.requestPermission()
      setPermission(result)
    }
    if (!next) {
      clearRef.current()
    }
  }, [enabled])

  // Clear tray notification when window gets focus
  useEffect(() => {
    const handleFocus = () => clearRef.current()
    window.addEventListener('focus', handleFocus)
    return () => window.removeEventListener('focus', handleFocus)
  }, [])

  return {
    notificationsEnabled: enabled,
    notificationPermission: permission,
    toggleNotifications,
    fireNotification,
    clearNotification,
  }
}
