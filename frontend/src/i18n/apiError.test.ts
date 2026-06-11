import { describe, it, expect, beforeEach } from 'vitest'
import i18n from './index'
import { translateApiError } from './apiError'

describe('translateApiError', () => {
  beforeEach(async () => {
    await i18n.changeLanguage('ca')
  })

  it('tradueix un codi conegut a català', () => {
    const msg = translateApiError({ code: 1002, message: 'qualsevol' })
    expect(msg).toBe('Credencials incorrectes')
  })

  it('tradueix el mateix codi a anglès en canviar idioma', async () => {
    await i18n.changeLanguage('en')
    const msg = translateApiError({ code: 1002, message: 'qualsevol' })
    expect(msg).toBe('Incorrect credentials')
  })

  it('interpola details (4001 → max)', () => {
    const msg = translateApiError({ code: 4001, message: 'x', details: { max: '4096' } })
    expect(msg).toBe('El missatge és massa llarg (màxim 4096 caràcters)')
  })

  it('fa fallback al missatge del backend si el codi no té traducció', () => {
    const msg = translateApiError({ code: 99999, message: 'Error del backend' })
    expect(msg).toBe('Error del backend')
  })
})
