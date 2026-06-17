import React, { createContext, useContext, useState, useCallback, ReactNode } from 'react'
import { ml_kem1024 } from '@noble/post-quantum/ml-kem.js'
import { User } from '../types'
import { authLogin, authRegister, authRegisterWithInvitation, authMe, authRefresh, deviceUpdatePublicKey } from '../lib/api'
import { disconnectSocket } from '../lib/socket'
import { getStoredDeviceId, persistDeviceId } from '../lib/device-identity'
import { generateAndStoreDeviceKeypair, hasLocalDeviceKeypair } from '../lib/device-keys'
import { isLocalVaultUnlocked, lockLocalVault } from '../lib/local-vault'

const ML_KEM_1024_PUBLIC_KEY_BYTES = 1568

function uint8ArrayToBase64(data: Uint8Array): string {
  let binary = ''
  for (let i = 0; i < data.length; i++) binary += String.fromCharCode(data[i])
  return btoa(binary)
}

function isValidKemPublicKey(key: Uint8Array): boolean {
  if (key.length !== ML_KEM_1024_PUBLIC_KEY_BYTES) {
    return false
  }

  try {
    ml_kem1024.encapsulate(key)
    return true
  } catch {
    return false
  }
}

interface AuthContextType {
  user: User | null
  token: string | null
  currentDeviceId: string | null
  isAuthenticated: boolean
  isLoading: boolean
  error: string | null
  login: (username: string, password: string, rememberMe?: boolean) => Promise<void>
  register: (username: string, password: string) => Promise<void>
  registerWithInvitation: (code: string, username: string, password: string) => Promise<void>
  logout: () => void
  refreshToken: () => Promise<void>
  ensureCurrentDeviceKeypair: () => Promise<void>
}

const AuthContext = createContext<AuthContextType | undefined>(undefined)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<User | null>(null)
  const [token, setToken] = useState<string | null>(null)
  const [currentDeviceId, setCurrentDeviceId] = useState<string | null>(null)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const TOKEN_KEY = 'chillgroup-token'
  const REMEMBER_ME_KEY = 'chillgroup-remember-me'

  const saveToken = useCallback((newToken: string, rememberMe?: boolean) => {
    setToken(newToken)
    try {
      if (rememberMe) {
        localStorage.setItem(TOKEN_KEY, newToken)
        localStorage.setItem(REMEMBER_ME_KEY, '1')
        sessionStorage.removeItem(TOKEN_KEY)
      } else {
        sessionStorage.setItem(TOKEN_KEY, newToken)
        localStorage.removeItem(TOKEN_KEY)
        localStorage.removeItem(REMEMBER_ME_KEY)
      }
    } catch {
      // storage not available
    }
  }, [])

  const loadToken = useCallback(() => {
    try {
      const persistent = localStorage.getItem(TOKEN_KEY)
      if (persistent) {
        setToken(persistent)
        return
      }
      const session = sessionStorage.getItem(TOKEN_KEY)
      if (session) {
        setToken(session)
      }
    } catch {
      // storage not available
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
    lockLocalVault()
    disconnectSocket()
    try {
      sessionStorage.removeItem(TOKEN_KEY)
      localStorage.removeItem(TOKEN_KEY)
      localStorage.removeItem(REMEMBER_ME_KEY)
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

  // Assegura que el dispositiu té un keypair ML-KEM generat i pujat al servidor.
  const ensureDeviceKeypairUploaded = useCallback(async (deviceId: string) => {
    if (!isLocalVaultUnlocked()) {
      // Vault bloquejat — no podem llegir ni generar claus xifrades.
      // Intentem pujar les claus públiques existents (no necessiten vault).
      const { getDevicePublicKeys } = await import('../lib/storage')
      const existing = await getDevicePublicKeys(deviceId)
      if (existing?.kemPublicKey && existing.dsaPublicKey) {
        const kemPublicKey = uint8ArrayToBase64(existing.kemPublicKey)
        const dsaPublicKey = uint8ArrayToBase64(existing.dsaPublicKey)
        await deviceUpdatePublicKey(kemPublicKey, dsaPublicKey)
      }
      // Si no hi ha claus, diferim fins que el vault estigui desbloquejat.
      return
    }

    const alreadyHas = await hasLocalDeviceKeypair(deviceId)
    let kemPublicKey: string
    let dsaPublicKey: string
    if (alreadyHas) {
      const { getDevicePublicKeys } = await import('../lib/storage')
      const keypair = await getDevicePublicKeys(deviceId)
      if (
        !keypair?.kemPublicKey ||
        !keypair.dsaPublicKey ||
        !isValidKemPublicKey(keypair.kemPublicKey)
      ) {
        const repaired = await generateAndStoreDeviceKeypair(deviceId, true)
        kemPublicKey = repaired.kemPublicKey
        dsaPublicKey = repaired.dsaPublicKey
      } else {
        kemPublicKey = uint8ArrayToBase64(keypair.kemPublicKey)
        dsaPublicKey = uint8ArrayToBase64(keypair.dsaPublicKey)
      }
    } else {
      const result = await generateAndStoreDeviceKeypair(deviceId, true)
      kemPublicKey = result.kemPublicKey
      dsaPublicKey = result.dsaPublicKey
    }

    const uploadResult = await deviceUpdatePublicKey(kemPublicKey, dsaPublicKey)
    if (!uploadResult.success) {
      throw new Error(uploadResult.error.message || 'No s\'ha pogut registrar la clau pública del dispositiu')
    }
  }, [])

  const ensureCurrentDeviceKeypair = useCallback(async () => {
    const deviceId = getStoredDeviceId()
    if (deviceId) {
      await ensureDeviceKeypairUploaded(deviceId)
    }
  }, [ensureDeviceKeypairUploaded])

  const login = useCallback(async (username: string, password: string, rememberMe?: boolean) => {
    setIsLoading(true)
    setError(null)
    try {
      const result = await authLogin(username, password)
      if (result.success && result.data) {
        saveToken(result.data.token, rememberMe)
        saveDeviceId(result.data.deviceId)
        await ensureDeviceKeypairUploaded(result.data.deviceId)
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
  }, [saveToken, saveDeviceId, fetchUser, ensureDeviceKeypairUploaded])

  const register = useCallback(async (username: string, password: string) => {
    setIsLoading(true)
    setError(null)
    try {
      const result = await authRegister(username, password)
      if (result.success && result.data) {
        saveToken(result.data.token)
        saveDeviceId(result.data.deviceId)
        await ensureDeviceKeypairUploaded(result.data.deviceId)
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
  }, [saveToken, saveDeviceId, fetchUser, ensureDeviceKeypairUploaded])

  const registerWithInvitation = useCallback(async (code: string, username: string, password: string) => {
    setIsLoading(true)
    setError(null)
    try {
      const result = await authRegisterWithInvitation(code, username, password)
      if (result.success && result.data) {
        saveToken(result.data.token)
        saveDeviceId(result.data.deviceId)
        await ensureDeviceKeypairUploaded(result.data.deviceId)
        await fetchUser(result.data.token)
      } else {
        const msg = !result.success ? result.error.message : 'Register with invitation failed'
        throw new Error(msg)
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Error desconegut'
      setError(msg)
      throw err
    } finally {
      setIsLoading(false)
    }
  }, [saveToken, saveDeviceId, fetchUser, ensureDeviceKeypairUploaded])

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
    
    const stored = localStorage.getItem(TOKEN_KEY) || sessionStorage.getItem(TOKEN_KEY)
    if (stored) {
      let cancelled = false
      setIsLoading(true)
      const storedDeviceId = getStoredDeviceId()
      
      fetchUser(stored)
        .then(async () => {
          if (storedDeviceId) {
            try {
              await ensureDeviceKeypairUploaded(storedDeviceId)
            } catch {
              // El login explícit ja mostrarà errors; aquí fem best-effort de reparació.
            }
          }
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
  }, [loadToken, loadDeviceId, fetchUser, ensureDeviceKeypairUploaded, user])

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
        registerWithInvitation,
        logout,
        refreshToken,
        ensureCurrentDeviceKeypair,
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