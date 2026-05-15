import React, { createContext, useContext, useState, useCallback, ReactNode } from 'react'
import { User } from '../types'
import { authLogin, authRegister, authMe, authRefresh } from '../lib/api'

interface AuthContextType {
  user: User | null
  token: string | null
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
      const oldToken = sessionStorage.getItem('chillgroup-token')
      sessionStorage.setItem('chillgroup-token', tok)
      const result = await authMe()
      if (result.success && result.data) {
        setUser(result.data)
      } else {
        setUser(null)
      }
      if (oldToken) {
        sessionStorage.setItem('chillgroup-token', oldToken)
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
  }, [saveToken, fetchUser])

  const register = useCallback(async (username: string, password: string) => {
    setIsLoading(true)
    setError(null)
    try {
      const result = await authRegister(username, password)
      if (result.success && result.data) {
        saveToken(result.data.token)
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
  }, [saveToken, fetchUser])

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

  // Load token on mount and verify with timeout
  React.useEffect(() => {
    loadToken()
    
    // If there's a stored token, try to fetch user with timeout
    const stored = sessionStorage.getItem('chillgroup-token')
    if (stored) {
      let cancelled = false
      setIsLoading(true)
      
      const timeoutId = setTimeout(() => {
        // Timeout after 3 seconds - assume not authenticated
        if (!cancelled) {
          setIsLoading(false)
        }
      }, 3000)
      
      fetchUser(stored).then(() => {
        if (!cancelled) {
          clearTimeout(timeoutId)
          setIsLoading(false)
        }
      }).catch(() => {
        if (!cancelled) {
          clearTimeout(timeoutId)
          setIsLoading(false)
        }
      })
      
      return () => {
        cancelled = true
        clearTimeout(timeoutId)
      }
    }
  }, [loadToken, fetchUser])

  return (
    <AuthContext.Provider
      value={{
        user,
        token,
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