import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import {
  encryptBackup,
  decryptBackup,
  isEncryptedBackup,
  generateAndStoreDeviceKeypair,
  hasLocalDeviceKeypair,
  exportDeviceKeypair,
  deleteDeviceKeypair,
  listDeviceKeypairs,
} from './device-keys'
import { createLocalVault, lockLocalVault } from './local-vault'

describe('encryptBackup / decryptBackup', () => {
  it('round-trip: encripta i desxifra correctament', async () => {
    const original = JSON.stringify({ key: 'value', num: 42 })
    const encrypted = await encryptBackup(original, 'contrasenya-segura')
    const decrypted = await decryptBackup(encrypted, 'contrasenya-segura')
    expect(decrypted).toBe(original)
  })

  it('encryptBackup genera JSON vàlid amb camps esperats', async () => {
    const enc = await encryptBackup('test', 'pass')
    const parsed = JSON.parse(enc)
    expect(parsed.encrypted).toBe(true)
    expect(parsed.algorithm).toBe('AES-GCM')
    expect(parsed.kdf).toBe('PBKDF2')
    expect(typeof parsed.salt).toBe('string')
    expect(typeof parsed.iv).toBe('string')
    expect(typeof parsed.ciphertext).toBe('string')
  })

  it('dues encriptacions del mateix text donen resultats diferents (IV aleatori)', async () => {
    const enc1 = await encryptBackup('missatge', 'pass')
    const enc2 = await encryptBackup('missatge', 'pass')
    const p1 = JSON.parse(enc1)
    const p2 = JSON.parse(enc2)
    expect(p1.ciphertext).not.toBe(p2.ciphertext)
    expect(p1.iv).not.toBe(p2.iv)
  })

  it('decryptBackup llança error amb contrasenya incorrecta', async () => {
    const enc = await encryptBackup('secret', 'correcta')
    await expect(decryptBackup(enc, 'incorrecta')).rejects.toThrow()
  })

  it('decryptBackup retorna el text original si no estava xifrat', async () => {
    const plain = JSON.stringify({ foo: 'bar' })
    const result = await decryptBackup(plain, 'qualsevol')
    expect(result).toBe(plain)
  })
})

describe('isEncryptedBackup', () => {
  it('retorna true per a un backup xifrat', async () => {
    const enc = await encryptBackup('data', 'pass')
    expect(isEncryptedBackup(enc)).toBe(true)
  })

  it('retorna false per a JSON pla sense encrypted=true', () => {
    expect(isEncryptedBackup(JSON.stringify({ foo: 'bar' }))).toBe(false)
  })

  it('retorna false per a text no-JSON', () => {
    expect(isEncryptedBackup('not json at all')).toBe(false)
  })
})

describe('generateAndStoreDeviceKeypair', () => {
  const deviceId = `test-device-${Date.now()}`

  beforeEach(async () => {
    localStorage.clear()
    await createLocalVault('test-passphrase')
  })

  afterEach(() => {
    lockLocalVault()
    localStorage.clear()
  })

  it('genera i emmagatzema un keypair correctament', async () => {
    await generateAndStoreDeviceKeypair(deviceId)
    const exists = await hasLocalDeviceKeypair(deviceId)
    expect(exists).toBe(true)
  })

  it('hasLocalDeviceKeypair retorna false per a un device desconegut', async () => {
    const exists = await hasLocalDeviceKeypair('non-existent-device-xyz')
    expect(exists).toBe(false)
  })

  it('llença KeypairDeviceIdExistsError si el device ja existeix', async () => {
    const id = `dup-device-${Date.now()}`
    await generateAndStoreDeviceKeypair(id)
    const { KeypairDeviceIdExistsError } = await import('./device-keys')
    await expect(generateAndStoreDeviceKeypair(id)).rejects.toBeInstanceOf(KeypairDeviceIdExistsError)
  })

  it('exportDeviceKeypair retorna JSON amb camps de keypair', async () => {
    const id = `export-device-${Date.now()}`
    await generateAndStoreDeviceKeypair(id)
    const exported = await exportDeviceKeypair(id)
    const parsed = JSON.parse(exported)
    expect(parsed.deviceId).toBe(id)
    expect(parsed.kemAlgorithm).toBe('ML-KEM-1024')
    expect(parsed.dsaAlgorithm).toBe('ML-DSA-87')
    expect(typeof parsed.kemPublicKey).toBe('string')
    expect(typeof parsed.kemSecretKey).toBe('string')
  })

  it('deleteDeviceKeypair elimina el keypair', async () => {
    const id = `del-device-${Date.now()}`
    await generateAndStoreDeviceKeypair(id)
    expect(await hasLocalDeviceKeypair(id)).toBe(true)
    await deleteDeviceKeypair(id)
    expect(await hasLocalDeviceKeypair(id)).toBe(false)
  })

  it('listDeviceKeypairs inclou el device generat', async () => {
    const id = `list-device-${Date.now()}`
    await generateAndStoreDeviceKeypair(id)
    const list = await listDeviceKeypairs()
    expect(list.some((d) => d.deviceId === id)).toBe(true)
  })
})
