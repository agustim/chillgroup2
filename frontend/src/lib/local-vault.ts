const LOCAL_VAULT_META_KEY = 'chillgroup-local-vault-meta'
const LOCAL_VAULT_KDF_ITERATIONS = 600_000
const LOCAL_VAULT_VERIFIER_TEXT = 'chillgroup-local-vault-verifier-v1'

interface LocalVaultMeta {
  version: 1
  kdf: 'PBKDF2'
  kdfHash: 'SHA-256'
  kdfIterations: number
  salt: string
  verifierIv: string
  verifierCiphertext: string
  createdAt: number
}

interface EncryptedBytesBundle {
  version: 1
  algorithm: 'AES-GCM'
  iv: string
  ciphertext: string
}

let activeVaultKey: CryptoKey | null = null

function uint8ArrayToBase64(data: Uint8Array): string {
  let binary = ''
  const chunkSize = 0x8000
  for (let i = 0; i < data.length; i += chunkSize) {
    binary += String.fromCharCode(...data.subarray(i, i + chunkSize))
  }
  return btoa(binary)
}

function base64ToUint8Array(value: string): Uint8Array {
  const binary = atob(value)
  const output = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    output[i] = binary.charCodeAt(i)
  }
  return output
}

function toArrayBuffer(data: Uint8Array): ArrayBuffer {
  const output = new ArrayBuffer(data.byteLength)
  new Uint8Array(output).set(data)
  return output
}

function readLocalVaultMeta(): LocalVaultMeta | null {
  try {
    const raw = localStorage.getItem(LOCAL_VAULT_META_KEY)
    if (!raw) return null
    const parsed = JSON.parse(raw) as Partial<LocalVaultMeta>
    if (
      parsed.version !== 1 ||
      parsed.kdf !== 'PBKDF2' ||
      parsed.kdfHash !== 'SHA-256' ||
      typeof parsed.kdfIterations !== 'number' ||
      typeof parsed.salt !== 'string' ||
      typeof parsed.verifierIv !== 'string' ||
      typeof parsed.verifierCiphertext !== 'string'
    ) {
      return null
    }
    return parsed as LocalVaultMeta
  } catch {
    return null
  }
}

async function deriveVaultKey(passphrase: string, salt: Uint8Array): Promise<CryptoKey> {
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(passphrase),
    { name: 'PBKDF2' },
    false,
    ['deriveKey']
  )

  return crypto.subtle.deriveKey(
    {
      name: 'PBKDF2',
      salt: toArrayBuffer(salt),
      iterations: LOCAL_VAULT_KDF_ITERATIONS,
      hash: 'SHA-256',
    },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  )
}

async function encryptWithVaultKey(data: Uint8Array, key: CryptoKey): Promise<EncryptedBytesBundle> {
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const encrypted = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv: toArrayBuffer(iv) },
    key,
    toArrayBuffer(data)
  )

  return {
    version: 1,
    algorithm: 'AES-GCM',
    iv: uint8ArrayToBase64(iv),
    ciphertext: uint8ArrayToBase64(new Uint8Array(encrypted)),
  }
}

async function decryptWithVaultKey(bundle: EncryptedBytesBundle, key: CryptoKey): Promise<Uint8Array> {
  const iv = base64ToUint8Array(bundle.iv)
  const ciphertext = base64ToUint8Array(bundle.ciphertext)
  const plaintext = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: toArrayBuffer(iv) },
    key,
    toArrayBuffer(ciphertext)
  )
  return new Uint8Array(plaintext)
}

export function hasLocalVault(): boolean {
  return readLocalVaultMeta() !== null
}

export function isLocalVaultUnlocked(): boolean {
  return activeVaultKey !== null
}

export function lockLocalVault(): void {
  activeVaultKey = null
}

export async function createLocalVault(passphrase: string): Promise<void> {
  const trimmed = passphrase.trim()
  if (!trimmed) {
    throw new Error('Has d\'indicar la clau de desbloqueig local')
  }

  const salt = crypto.getRandomValues(new Uint8Array(16))
  const derived = await deriveVaultKey(trimmed, salt)
  const verifierBundle = await encryptWithVaultKey(new TextEncoder().encode(LOCAL_VAULT_VERIFIER_TEXT), derived)

  const meta: LocalVaultMeta = {
    version: 1,
    kdf: 'PBKDF2',
    kdfHash: 'SHA-256',
    kdfIterations: LOCAL_VAULT_KDF_ITERATIONS,
    salt: uint8ArrayToBase64(salt),
    verifierIv: verifierBundle.iv,
    verifierCiphertext: verifierBundle.ciphertext,
    createdAt: Date.now(),
  }

  localStorage.setItem(LOCAL_VAULT_META_KEY, JSON.stringify(meta))
  activeVaultKey = derived
}

export async function unlockLocalVault(passphrase: string): Promise<void> {
  const meta = readLocalVaultMeta()
  if (!meta) {
    throw new Error('No hi ha cap vault local configurat')
  }

  const trimmed = passphrase.trim()
  if (!trimmed) {
    throw new Error('Has d\'indicar la clau de desbloqueig local')
  }

  const derived = await deriveVaultKey(trimmed, base64ToUint8Array(meta.salt))
  try {
    const verifier = await decryptWithVaultKey(
      {
        version: 1,
        algorithm: 'AES-GCM',
        iv: meta.verifierIv,
        ciphertext: meta.verifierCiphertext,
      },
      derived
    )

    if (new TextDecoder().decode(verifier) !== LOCAL_VAULT_VERIFIER_TEXT) {
      throw new Error('Clau local incorrecta')
    }
  } catch {
    throw new Error('Clau local incorrecta')
  }

  activeVaultKey = derived
}

export async function rotateLocalVaultPassphrase(currentPassphrase: string, newPassphrase: string): Promise<void> {
  await unlockLocalVault(currentPassphrase)
  await createLocalVault(newPassphrase)
}

export async function encryptBytesForLocalVault(data: Uint8Array): Promise<string> {
  if (!activeVaultKey) {
    throw new Error('Vault local bloquejat')
  }
  const bundle = await encryptWithVaultKey(data, activeVaultKey)
  return JSON.stringify(bundle)
}

export async function decryptBytesFromLocalVault(bundleText: string): Promise<Uint8Array> {
  if (!activeVaultKey) {
    throw new Error('Vault local bloquejat')
  }

  let parsed: EncryptedBytesBundle
  try {
    parsed = JSON.parse(bundleText) as EncryptedBytesBundle
  } catch {
    throw new Error('Format de clau local xifrada invàlid')
  }

  if (parsed.version !== 1 || parsed.algorithm !== 'AES-GCM') {
    throw new Error('Format de clau local xifrada no compatible')
  }

  return decryptWithVaultKey(parsed, activeVaultKey)
}
