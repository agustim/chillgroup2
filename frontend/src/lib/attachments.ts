import {
  attachmentComplete,
  attachmentInit,
  attachmentSignPart,
  type AttachmentCompletePart,
} from './api'

const MULTIPART_CHUNK_SIZE = 5 * 1024 * 1024

interface UploadEncryptedAttachmentParams {
  channelId: string
  file: File
  keyVersionId: string
  keyVersion: number
  thumbnailAttachmentId?: string
  channelKeyBytes?: Uint8Array
}

interface UploadEncryptedAttachmentResult {
  attachmentId: string
  fileName: string
  sizeBytes: number
}

interface AttachmentDownloadPayload {
  fileName: string
  downloadUrl: string
  mimeType: string
  crypto: {
    wrappedFileKey: string
    fileIv: string
  }
  channelKeyBytes?: Uint8Array
}

function base64ToUint8Array(value: string): Uint8Array {
  const binary = atob(value)
  const output = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) {
    output[index] = binary.charCodeAt(index)
  }
  return output
}

function triggerFileDownload(blob: Blob, fileName: string): void {
  const objectUrl = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = objectUrl
  link.download = fileName
  document.body.appendChild(link)
  link.click()
  link.remove()
  URL.revokeObjectURL(objectUrl)
}

function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  const out = new ArrayBuffer(bytes.byteLength)
  new Uint8Array(out).set(bytes)
  return out
}

function bytesToBase64(bytes: Uint8Array): string {
  let binary = ''
  for (let i = 0; i < bytes.length; i += 1) {
    binary += String.fromCharCode(bytes[i])
  }
  return btoa(binary)
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('')
}

async function sha256Hex(input: Uint8Array): Promise<string> {
  const digestInput = new Uint8Array(input.byteLength)
  digestInput.set(input)
  const digest = await crypto.subtle.digest('SHA-256', digestInput.buffer)
  return bytesToHex(new Uint8Array(digest))
}

async function encryptFile(file: File) {
  const plainBuffer = await file.arrayBuffer()
  const plainBytes = new Uint8Array(plainBuffer)

  const fileKey = await crypto.subtle.generateKey(
    { name: 'AES-GCM', length: 256 },
    true,
    ['encrypt'],
  )

  const rawFileKey = await crypto.subtle.exportKey('raw', fileKey)
  const rawFileKeyBytes = new Uint8Array(rawFileKey)

  const ivBytes = crypto.getRandomValues(new Uint8Array(12))
  const encryptedBuffer = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv: ivBytes },
    fileKey,
    plainBytes,
  )

  const ciphertext = new Uint8Array(encryptedBuffer)
  const ciphertextSha256 = await sha256Hex(ciphertext)

  return {
    ciphertext,
    fileIvBase64: bytesToBase64(ivBytes),
    wrappedFileKeyBase64: bytesToBase64(rawFileKeyBytes),
    ciphertextSha256,
  }
}

function getEtagFromResponse(response: Response): string | null {
  return response.headers.get('etag') ?? response.headers.get('ETag')
}

function getAuthToken(): string | null {
  try {
    return localStorage.getItem('chillgroup-token') ?? sessionStorage.getItem('chillgroup-token')
  } catch {
    return null
  }
}

function isApiProxyUrl(url: string): boolean {
  return url.startsWith('/api/') || url.includes('/api/channels/')
}

async function resolveUploadEtag(response: Response): Promise<string | null> {
  const headerEtag = getEtagFromResponse(response)
  if (headerEtag) return headerEtag

  try {
    const body = await response.clone().json() as { etag?: string }
    if (typeof body?.etag === 'string' && body.etag.length > 0) {
      return body.etag
    }
  } catch {
    // Ignore non-JSON bodies.
  }

  return null
}

async function wrapFileKey(rawKeyBytes: Uint8Array, channelKeyBytes: Uint8Array): Promise<string> {
  const channelKey = await crypto.subtle.importKey(
    'raw', toArrayBuffer(channelKeyBytes), { name: 'AES-GCM' }, false, ['encrypt'],
  )
  const iv = crypto.getRandomValues(new Uint8Array(12))
  const wrapped = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv: toArrayBuffer(iv) }, channelKey, toArrayBuffer(rawKeyBytes),
  )
  const combined = new Uint8Array(12 + wrapped.byteLength)
  combined.set(iv)
  combined.set(new Uint8Array(wrapped), 12)
  return 'w1:' + bytesToBase64(combined)
}

async function unwrapFileKey(wrappedFileKey: string, channelKeyBytes?: Uint8Array): Promise<Uint8Array> {
  if (!wrappedFileKey.startsWith('w1:')) {
    return base64ToUint8Array(wrappedFileKey)
  }
  if (!channelKeyBytes) {
    throw new Error('Cal la clau de canal per desxifrar aquest adjunt')
  }
  const combined = base64ToUint8Array(wrappedFileKey.slice(3))
  const iv = combined.slice(0, 12)
  const ciphertext = combined.slice(12)
  const channelKey = await crypto.subtle.importKey(
    'raw', toArrayBuffer(channelKeyBytes), { name: 'AES-GCM' }, false, ['decrypt'],
  )
  const rawKey = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: toArrayBuffer(iv) }, channelKey, toArrayBuffer(ciphertext),
  )
  return new Uint8Array(rawKey)
}

export async function generateThumbnail(file: File, maxPx = 200): Promise<Blob | null> {
  if (!file.type.startsWith('image/')) return null
  try {
    const bitmap = await createImageBitmap(file)
    const scale = Math.min(1, maxPx / Math.max(bitmap.width, bitmap.height))
    const w = Math.round(bitmap.width * scale)
    const h = Math.round(bitmap.height * scale)
    const canvas = new OffscreenCanvas(w, h)
    const ctx = canvas.getContext('2d')!
    ctx.drawImage(bitmap, 0, 0, w, h)
    bitmap.close()
    return await canvas.convertToBlob({ type: 'image/jpeg', quality: 0.7 })
  } catch {
    return null
  }
}

export async function decryptAttachmentToBlob(attachment: AttachmentDownloadPayload): Promise<Blob> {
  const headers: Record<string, string> = {}
  if (isApiProxyUrl(attachment.downloadUrl)) {
    const token = getAuthToken()
    if (token) headers.Authorization = `Bearer ${token}`
  }

  const response = await fetch(attachment.downloadUrl, { headers })
  if (!response.ok) throw new Error(`Download de l'adjunt fallit (${response.status})`)

  const ciphertext = new Uint8Array(await response.arrayBuffer())
  const fileKeyBytes = await unwrapFileKey(attachment.crypto.wrappedFileKey, attachment.channelKeyBytes)
  const ivBytes = base64ToUint8Array(attachment.crypto.fileIv)

  const fileKey = await crypto.subtle.importKey(
    'raw',
    toArrayBuffer(fileKeyBytes),
    { name: 'AES-GCM' },
    false,
    ['decrypt'],
  )

  const plaintextBuffer = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: toArrayBuffer(ivBytes) },
    fileKey,
    toArrayBuffer(ciphertext),
  )

  return new Blob([plaintextBuffer], { type: attachment.mimeType || 'application/octet-stream' })
}

export async function uploadEncryptedAttachment(
  params: UploadEncryptedAttachmentParams,
): Promise<UploadEncryptedAttachmentResult> {
  const { channelId, file, keyVersionId, keyVersion, thumbnailAttachmentId, channelKeyBytes } = params

  const encrypted = await encryptFile(file)

  const wrappedFileKeyBase64 = channelKeyBytes
    ? await wrapFileKey(base64ToUint8Array(encrypted.wrappedFileKeyBase64), channelKeyBytes)
    : encrypted.wrappedFileKeyBase64
  const chunkCount = Math.max(1, Math.ceil(encrypted.ciphertext.byteLength / MULTIPART_CHUNK_SIZE))

  const initResult = await attachmentInit(channelId, {
    fileName: file.name,
    mimeType: file.type || 'application/octet-stream',
    sizeBytes: encrypted.ciphertext.byteLength,
    chunkSizeBytes: MULTIPART_CHUNK_SIZE,
    chunkCount,
  })

  if (!initResult.success) {
    throw new Error(initResult.error.message || "No s'ha pogut inicialitzar l'adjunt")
  }

  const { attachmentId, uploadId } = initResult.data
  const parts: AttachmentCompletePart[] = []

  for (let index = 0; index < chunkCount; index += 1) {
    const partNumber = index + 1
    const signResult = await attachmentSignPart(channelId, attachmentId, uploadId, partNumber)

    if (!signResult.success) {
      throw new Error(signResult.error.message || `No s'ha pogut signar la part ${partNumber}`)
    }

    const start = index * MULTIPART_CHUNK_SIZE
    const end = Math.min((index + 1) * MULTIPART_CHUNK_SIZE, encrypted.ciphertext.byteLength)
    const chunk = encrypted.ciphertext.slice(start, end)

    const headers: Record<string, string> = {}
    if (isApiProxyUrl(signResult.data.uploadUrl)) {
      const token = getAuthToken()
      if (token) {
        headers.Authorization = `Bearer ${token}`
      }
    }

    const uploadResponse = await fetch(signResult.data.uploadUrl, {
      method: 'PUT',
      body: chunk,
      headers,
    })

    if (!uploadResponse.ok) {
      throw new Error(`Upload de la part ${partNumber} fallit (${uploadResponse.status})`)
    }

    const etag = await resolveUploadEtag(uploadResponse)
    if (!etag) {
      throw new Error(`No s'ha rebut ETag per la part ${partNumber}`)
    }

    parts.push({ partNumber, etag })
  }

  const completeResult = await attachmentComplete(channelId, attachmentId, {
    uploadId,
    parts,
    crypto: {
      algorithm: 'aes-256-gcm',
      fileIv: encrypted.fileIvBase64,
      wrappedFileKey: wrappedFileKeyBase64,
      keyVersionId,
      keyVersion,
      ciphertextSha256: encrypted.ciphertextSha256,
    },
    thumbnail_attachment_id: thumbnailAttachmentId,
  })

  if (!completeResult.success) {
    throw new Error(completeResult.error.message || "No s'ha pogut completar l'adjunt")
  }

  return {
    attachmentId,
    fileName: file.name,
    sizeBytes: file.size,
  }
}

export async function downloadAndDecryptAttachment(attachment: AttachmentDownloadPayload): Promise<void> {
  const blob = await decryptAttachmentToBlob(attachment)
  triggerFileDownload(blob, attachment.fileName)
}
