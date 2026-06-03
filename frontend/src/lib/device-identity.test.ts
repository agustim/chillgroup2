import { describe, it, expect, beforeEach } from 'vitest'
import { persistDeviceId, getStoredDeviceId } from './device-identity'

describe('device-identity', () => {
  beforeEach(() => {
    localStorage.clear()
    // Netejar cookies del test
    document.cookie.split(';').forEach((c) => {
      document.cookie = c.trim().split('=')[0] + '=; Max-Age=0; Path=/'
    })
  })

  it('persistDeviceId guarda a localStorage', () => {
    persistDeviceId('dev-abc-123')
    expect(localStorage.getItem('chillgroup-device-id')).toBe('dev-abc-123')
  })

  it('getStoredDeviceId retorna el valor de localStorage', () => {
    persistDeviceId('device-xyz')
    expect(getStoredDeviceId()).toBe('device-xyz')
  })

  it('getStoredDeviceId retorna null si no hi ha res persistit', () => {
    expect(getStoredDeviceId()).toBeNull()
  })

  it('persistDeviceId sobreescriu un valor anterior', () => {
    persistDeviceId('device-v1')
    persistDeviceId('device-v2')
    expect(getStoredDeviceId()).toBe('device-v2')
  })

  it('getStoredDeviceId llegeix de cookie si localStorage no té el valor', () => {
    persistDeviceId('via-cookie')
    localStorage.removeItem('chillgroup-device-id')
    // Ara hauria de tornar via cookie
    const result = getStoredDeviceId()
    expect(result).toBe('via-cookie')
  })
})
