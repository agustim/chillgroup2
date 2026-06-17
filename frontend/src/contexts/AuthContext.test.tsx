import { describe, it, expect, vi, beforeEach } from 'vitest'
import { renderHook, act } from '@testing-library/react'
import React from 'react'
import { AuthProvider, useAuth } from './AuthContext'

vi.mock('../lib/api', () => ({
  authLogin: vi.fn(),
  authRegister: vi.fn(),
  authRegisterWithInvitation: vi.fn(),
  authMe: vi.fn(),
  authRefresh: vi.fn(),
  deviceUpdatePublicKey: vi.fn(),
}))

vi.mock('../lib/socket', () => ({
  disconnectSocket: vi.fn(),
}))

vi.mock('../lib/device-identity', () => ({
  getStoredDeviceId: vi.fn(() => 'device-1'),
  persistDeviceId: vi.fn(),
}))

vi.mock('../lib/device-keys', () => ({
  generateAndStoreDeviceKeypair: vi.fn(),
  hasLocalDeviceKeypair: vi.fn(async () => false),
}))

vi.mock('../lib/local-vault', () => ({
  isLocalVaultUnlocked: vi.fn(() => false),
  lockLocalVault: vi.fn(),
}))

vi.mock('../lib/storage', () => ({
  getDevicePublicKeys: vi.fn(async () => null),
}))

import { authLogin, authMe } from '../lib/api'

const TOKEN = 'test-jwt-token'
const TOKEN_KEY = 'chillgroup-token'
const REMEMBER_ME_KEY = 'chillgroup-remember-me'

function wrapper({ children }: { children: React.ReactNode }) {
  return <AuthProvider>{children}</AuthProvider>
}

describe('AuthContext — token storage', () => {
  beforeEach(() => {
    localStorage.clear()
    sessionStorage.clear()
    vi.clearAllMocks()
    vi.mocked(authMe).mockResolvedValue({ success: false, error: { message: 'no user' } } as any)
  })

  it('login without rememberMe saves to sessionStorage only', async () => {
    vi.mocked(authLogin).mockResolvedValue({
      success: true,
      data: { token: TOKEN, deviceId: 'device-1' },
    } as any)
    vi.mocked(authMe).mockResolvedValue({ success: true, data: { userId: '1', username: 'u' } } as any)

    const { result } = renderHook(() => useAuth(), { wrapper })

    await act(async () => {
      await result.current.login('user', 'pass')
    })

    expect(sessionStorage.getItem(TOKEN_KEY)).toBe(TOKEN)
    expect(localStorage.getItem(TOKEN_KEY)).toBeNull()
    expect(localStorage.getItem(REMEMBER_ME_KEY)).toBeNull()
  })

  it('login with rememberMe=true saves to localStorage only', async () => {
    vi.mocked(authLogin).mockResolvedValue({
      success: true,
      data: { token: TOKEN, deviceId: 'device-1' },
    } as any)
    vi.mocked(authMe).mockResolvedValue({ success: true, data: { userId: '1', username: 'u' } } as any)

    const { result } = renderHook(() => useAuth(), { wrapper })

    await act(async () => {
      await result.current.login('user', 'pass', true)
    })

    expect(localStorage.getItem(TOKEN_KEY)).toBe(TOKEN)
    expect(localStorage.getItem(REMEMBER_ME_KEY)).toBe('1')
    expect(sessionStorage.getItem(TOKEN_KEY)).toBeNull()
  })

  it('logout clears token from both storages', async () => {
    vi.mocked(authLogin).mockResolvedValue({
      success: true,
      data: { token: TOKEN, deviceId: 'device-1' },
    } as any)
    vi.mocked(authMe).mockResolvedValue({ success: true, data: { userId: '1', username: 'u' } } as any)

    const { result } = renderHook(() => useAuth(), { wrapper })

    await act(async () => {
      await result.current.login('user', 'pass', true)
    })

    expect(localStorage.getItem(TOKEN_KEY)).toBe(TOKEN)

    act(() => {
      result.current.logout()
    })

    expect(localStorage.getItem(TOKEN_KEY)).toBeNull()
    expect(sessionStorage.getItem(TOKEN_KEY)).toBeNull()
    expect(localStorage.getItem(REMEMBER_ME_KEY)).toBeNull()
  })

  it('on mount prefers localStorage over sessionStorage', async () => {
    localStorage.setItem(TOKEN_KEY, 'persistent-token')
    sessionStorage.setItem(TOKEN_KEY, 'session-token')
    vi.mocked(authMe).mockResolvedValue({ success: true, data: { userId: '1', username: 'u' } } as any)

    const { result } = renderHook(() => useAuth(), { wrapper })

    await act(async () => {
      await new Promise((r) => setTimeout(r, 0))
    })

    expect(result.current.token).toBe('persistent-token')
  })

  it('on mount loads sessionStorage token when no localStorage token', async () => {
    sessionStorage.setItem(TOKEN_KEY, 'session-token')
    vi.mocked(authMe).mockResolvedValue({ success: true, data: { userId: '1', username: 'u' } } as any)

    const { result } = renderHook(() => useAuth(), { wrapper })

    await act(async () => {
      await new Promise((r) => setTimeout(r, 0))
    })

    expect(result.current.token).toBe('session-token')
  })

  it('on mount with no stored token stays unauthenticated', async () => {
    const { result } = renderHook(() => useAuth(), { wrapper })

    await act(async () => {
      await new Promise((r) => setTimeout(r, 0))
    })

    expect(result.current.token).toBeNull()
    expect(result.current.isAuthenticated).toBe(false)
  })
})
