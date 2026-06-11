import i18n from './index'
import type { ApiError } from '../lib/api'

/**
 * Tradueix un error de l'API al idioma actiu a partir del seu codi numèric.
 *
 * El backend envia { code, message } amb el missatge en català. Aquí mapegem
 * el codi a la clau `apiErrors.<code>` del catàleg i18n. Si no hi ha traducció
 * per aquell codi, fem servir el `message` del backend com a fallback.
 *
 * `details` del backend (p.ex. { max }) s'usa per interpolació.
 */
export function translateApiError(error: ApiError['error']): string {
  const { code, message, details } = error
  return i18n.t(`apiErrors.${code}`, {
    ...(details as Record<string, unknown> | undefined),
    defaultValue: message,
  })
}
