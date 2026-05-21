import React, { createContext, useContext, useState, useCallback, ReactNode } from 'react'
import { User } from '../types'
import { authLogin, authRegister, authMe, authRefresh } from '../lib/api'
import { getStoredDeviceId, persistDeviceId } from '../lib/device-identity'

interface AuthContextType {
  user: User | null
  token: string | null
  currentDeviceId: string | null
  isAuthenticated: boolean
  isLoading: boolean
  error: string | null
  login: (username: string, password: string) => Promise<void>
  register: (username: string, password: string) => Promise<void>
  logout: () => void
  refreshToken: () => Promise<void>
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [token, setToken] = useState<string | null>(null)
  const [currentDeviceId, setCurrentDeviceId] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const saveToken = useCallback((newToken: string) => {
    setToken(newToken)
    try {
      sessionStorage.setItem('chillgroup-token', newToken)
    } catch {
      // sessionStorage not available
    }
  }, [])

  const loadToken = useCallback(() => {
    try {
      const stored = sessionStorage.getItem('chillgroup-token')
      if (stored) {
        setToken(stored)
      }
    } catch {
      // sessionStorage not available
    }
  }, [])

  const saveDeviceId = useCallback((deviceId: string) => {
    setCurrentDeviceId(deviceId)
    persistDeviceId(deviceId)
  }, [])

  const loadDeviceId = useCallback(() => {
    const stored = getStoredDeviceId()
    setCurrentDeviceId(stored)
  }, [])

  const clearAuth = useCallback(() => {
    setUser(null)
    setToken(null)
    setError(null)
    try {
      sessionStorage.removeItem('chillgroup-token')
    } catch {
      // ignore
    }
  }, [])

  const fetchUser = useCallback(async (tok: string) => {
    try {
      const result = await authMe()
      if (result.success && result.data) {
        setUser(result.data)
      } else {
        setUser(null)
      }
    } catch {
      setUser(null)
    }
  }, [])

  const login = useCallback(async (username: string, password: string) => {
    setIsLoading(true)
    setError(null)
    try {
      const result = await authLogin(username, password)
      if (result.success && result.data) {
        saveToken(result.data.token)
        saveDeviceId(result.data.deviceId)
        await fetchUser(result.data.token)
      } else {
        const msg = !result.success ? result.error.message : 'Login failed'
        throw new Error(msg)
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Error desconegut'
      setError(msg)
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [saveToken, saveDeviceId, fetchUser])

  const register = useCallback(async (username: string, password: string) => {
    setIsLoading(true)
    setError(null)
    try {
      const result = await authRegister(username, password)
      if (result.success && result.data) {
        saveToken(result.data.token)
        saveDeviceId(result.data.deviceId)
        await fetchUser(result.data.token)
      } else {
        const msg = !result.success ? result.error.message : 'Register failed'
        throw new Error(msg)
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Error desconegut'
      setError(msg)
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [saveToken, saveDeviceId, fetchUser])

  const logout = useCallback(() => {
    clearAuth()
  }, [clearAuth])

  const refreshToken = useCallback(async () => {
    try {
      const result = await authRefresh()
      if (result.success && result.data) {
        saveToken(result.data.token)
        await fetchUser(result.data.token)
      }
    } catch {
      clearAuth()
    }
  }, [saveToken, fetchUser, clearAuth])

  // Load token on mount and verify (only if no user is already authenticated)
  React.useEffect(() => {
    loadToken()
    loadDeviceId()
    
    // Only verify stored token if no user is already set
    // This prevents race conditions after login
    if (user) {
      setIsLoading(false)
      return
    }
    
    const stored = sessionStorage.getItem('chillgroup-token')
    if (stored) {
      let cancelled = false
      setIsLoading(true)
      
      fetchUser(stored)
        .then(() => {
          if (!cancelled) {
            setIsLoading(false)
          }
        })
        .catch(() => {
          if (!cancelled) {
            setIsLoading(false)
          }
        })
      
      return () => {
        cancelled = true
      }
    } else {
      setIsLoading(false)
    }
  }, [loadToken, loadDeviceId, fetchUser, user])

  return (
    <AuthContext.Provider
      value={{
        user,
        token,
        currentDeviceId,
        isAuthenticated: !!user && !!token,
        isLoading,
        error,
        login,
        register,
        logout,
        refreshToken,
      }}
    >
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth(): AuthContextType {
  const context = useContext(AuthContext)
  if (context === undefined) {
    throw new Error('useAuth must be used within an AuthProvider')
  }
  return context
}