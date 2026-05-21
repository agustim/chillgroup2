const DEVICE_ID_STORAGE_KEY = 'chillgroup-device-id'
const DEVICE_ID_COOKIE_NAME = 'chillgroup_device_id'
const DEVICE_ID_COOKIE_MAX_AGE_SECONDS = 60 * 60 * 24 * 365

function setCookie(name: string, value: string, maxAgeSeconds: number): void {
  document.cookie = `${name}=${encodeURIComponent(value)}; Max-Age=${maxAgeSeconds}; Path=/; SameSite=Lax`
}

function getCookie(name: string): string | null {
  const chunks = document.cookie ? document.cookie.split('; ') : []
  for (const chunk of chunks) {
    const [key, ...rest] = chunk.split('=')
    if (key === name) {
      return decodeURIComponent(rest.join('='))
    }
  }
  return null
}

function clearCookie(name: string): void {
  document.cookie = `${name}=; Max-Age=0; Path=/; SameSite=Lax`
}

export function persistDeviceId(deviceId: string): void {
  try {
    localStorage.setItem(DEVICE_ID_STORAGE_KEY, deviceId)
  } catch {
    // Ignore storage failures in restricted environments.
  }
  setCookie(DEVICE_ID_COOKIE_NAME, deviceId, DEVICE_ID_COOKIE_MAX_AGE_SECONDS)
}

export function getStoredDeviceId(): string | null {
  try {
    const fromStorage = localStorage.getItem(DEVICE_ID_STORAGE_KEY)
    if (fromStorage) {
      return fromStorage
    }
  } catch {
    // Ignore storage failures in restricted environments.
  }

  return getCookie(DEVICE_ID_COOKIE_NAME)
}

export function clearStoredDeviceId(): void {
  try {
    localStorage.removeItem(DEVICE_ID_STORAGE_KEY)
  } catch {
    // Ignore storage failures in restricted environments.
  }
  clearCookie(DEVICE_ID_COOKIE_NAME)
}
