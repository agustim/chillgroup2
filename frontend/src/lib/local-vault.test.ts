import { beforeEach, describe, expect, it } from 'vitest'
import {
  createLocalVault,
  hasLocalVault,
  isLocalVaultUnlocked,
  lockLocalVault,
  rotateLocalVaultPassphrase,
  unlockLocalVault,
} from './local-vault'

describe('local vault', () => {
  beforeEach(() => {
    localStorage.clear()
    lockLocalVault()
  })

  it('configura i desbloqueja el vault local', async () => {
    expect(hasLocalVault()).toBe(false)
    expect(isLocalVaultUnlocked()).toBe(false)

    await createLocalVault('clau-local-segura')

    expect(hasLocalVault()).toBe(true)
    expect(isLocalVaultUnlocked()).toBe(true)
  })

  it('bloqueja i desbloqueja amb la clau correcta', async () => {
    await createLocalVault('clau-local')
    lockLocalVault()

    expect(isLocalVaultUnlocked()).toBe(false)

    await unlockLocalVault('clau-local')
    expect(isLocalVaultUnlocked()).toBe(true)
  })

  it('falla si la clau local és incorrecta', async () => {
    await createLocalVault('clau-local')
    lockLocalVault()

    await expect(unlockLocalVault('incorrecta')).rejects.toThrow('Clau local incorrecta')
  })

  it('rota la clau local i invalida l antiga', async () => {
    await createLocalVault('clau-antiga')

    await rotateLocalVaultPassphrase('clau-antiga', 'clau-nova')
    lockLocalVault()

    await expect(unlockLocalVault('clau-antiga')).rejects.toThrow('Clau local incorrecta')
    await unlockLocalVault('clau-nova')
    expect(isLocalVaultUnlocked()).toBe(true)
  })
})
