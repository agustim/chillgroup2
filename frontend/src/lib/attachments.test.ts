import { describe, it, expect, vi, beforeEach } from 'vitest'
import { generateThumbnail, uploadEncryptedAttachment } from './attachments'

// jsdom no implementa File.prototype.arrayBuffer
if (!(File.prototype as any).arrayBuffer) {
  ;(File.prototype as any).arrayBuffer = function (this: File): Promise<ArrayBuffer> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = () => resolve(reader.result as ArrayBuffer)
      reader.onerror = reject
      reader.readAsArrayBuffer(this)
    })
  }
}

vi.mock('./api', () => ({
  attachmentInit: vi.fn(),
  attachmentSignPart: vi.fn(),
  attachmentComplete: vi.fn(),
}))

import { attachmentInit, attachmentSignPart, attachmentComplete } from './api'

const mockAttachmentInit = vi.mocked(attachmentInit)
const mockAttachmentSignPart = vi.mocked(attachmentSignPart)
const mockAttachmentComplete = vi.mocked(attachmentComplete)

const mockFetch = vi.fn()
vi.stubGlobal('fetch', mockFetch)
vi.stubGlobal('sessionStorage', { getItem: vi.fn(() => null) })

// jsdom no té createImageBitmap ni OffscreenCanvas
const mockBitmapClose = vi.fn()
const mockBitmap = { width: 400, height: 300, close: mockBitmapClose }
const mockConvertToBlob = vi.fn()
const mockDrawImage = vi.fn()
const mockCanvasCtx = { drawImage: mockDrawImage }
const mockCanvas = {
  getContext: vi.fn(() => mockCanvasCtx),
  convertToBlob: mockConvertToBlob,
}
vi.stubGlobal('createImageBitmap', vi.fn())
vi.stubGlobal('OffscreenCanvas', vi.fn(() => mockCanvas))

function makeUploadMocks() {
  mockAttachmentInit.mockResolvedValue({
    success: true,
    data: {
      attachmentId: 'att-1',
      uploadId: 'upload-1',
      objectKey: 'key',
      chunkSizeBytes: 5242880,
      chunkCount: 1,
    },
  } as any)
  mockAttachmentSignPart.mockResolvedValue({
    success: true,
    data: { partNumber: 1, uploadUrl: '/api/channels/ch-1/upload/part' },
  } as any)
  mockFetch.mockResolvedValue({
    ok: true,
    headers: { get: (name: string) => (name === 'etag' ? '"etag-abc"' : null) },
    clone: () => ({ json: () => Promise.reject(new Error('not json')) }),
  })
  mockAttachmentComplete.mockResolvedValue({
    success: true,
    data: { attachmentId: 'att-1', status: 'complete' },
  } as any)
}

describe('generateThumbnail', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(globalThis.createImageBitmap).mockResolvedValue(mockBitmap as any)
    mockConvertToBlob.mockResolvedValue(new Blob(['jpeg'], { type: 'image/jpeg' }))
  })

  it('returns null for non-image files', async () => {
    const file = new File(['data'], 'doc.pdf', { type: 'application/pdf' })
    expect(await generateThumbnail(file)).toBeNull()
    expect(globalThis.createImageBitmap).not.toHaveBeenCalled()
  })

  it('returns JPEG Blob for image files', async () => {
    const file = new File(['data'], 'photo.jpg', { type: 'image/jpeg' })
    const result = await generateThumbnail(file)
    expect(result).toBeInstanceOf(Blob)
    expect(mockConvertToBlob).toHaveBeenCalledWith({ type: 'image/jpeg', quality: 0.7 })
  })

  it('scales down to fit maxPx keeping aspect ratio', async () => {
    const file = new File(['data'], 'big.png', { type: 'image/png' })
    // bitmap 400×300, maxPx=100 → scale=0.25 → 100×75
    await generateThumbnail(file, 100)
    expect(globalThis.OffscreenCanvas).toHaveBeenCalledWith(100, 75)
  })

  it('does not scale up images smaller than maxPx', async () => {
    vi.mocked(globalThis.createImageBitmap).mockResolvedValue({ width: 50, height: 40, close: mockBitmapClose } as any)
    const file = new File(['data'], 'small.png', { type: 'image/png' })
    await generateThumbnail(file, 200)
    // scale = min(1, 200/50) = 1 → 50×40
    expect(globalThis.OffscreenCanvas).toHaveBeenCalledWith(50, 40)
  })

  it('closes bitmap after use', async () => {
    const file = new File(['data'], 'photo.jpg', { type: 'image/jpeg' })
    await generateThumbnail(file)
    expect(mockBitmapClose).toHaveBeenCalled()
  })

  it('returns null if createImageBitmap throws', async () => {
    vi.mocked(globalThis.createImageBitmap).mockRejectedValue(new Error('not supported'))
    const file = new File(['data'], 'photo.jpg', { type: 'image/jpeg' })
    expect(await generateThumbnail(file)).toBeNull()
  })
})

describe('uploadEncryptedAttachment', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    makeUploadMocks()
  })

  it('passes thumbnail_attachment_id to complete request', async () => {
    const file = new File(['hello world'], 'photo.jpg', { type: 'image/jpeg' })
    await uploadEncryptedAttachment({
      channelId: 'ch-1',
      file,
      keyVersionId: 'kv-1',
      keyVersion: 1,
      thumbnailAttachmentId: 'thumb-att-1',
    })
    expect(mockAttachmentComplete).toHaveBeenCalledWith(
      'ch-1',
      'att-1',
      expect.objectContaining({ thumbnail_attachment_id: 'thumb-att-1' }),
    )
  })

  it('sends undefined thumbnail_attachment_id when not provided', async () => {
    const file = new File(['hello world'], 'doc.pdf', { type: 'application/pdf' })
    await uploadEncryptedAttachment({
      channelId: 'ch-1',
      file,
      keyVersionId: 'kv-1',
      keyVersion: 1,
    })
    expect(mockAttachmentComplete).toHaveBeenCalledWith(
      'ch-1',
      'att-1',
      expect.objectContaining({ thumbnail_attachment_id: undefined }),
    )
  })

  it('returns attachmentId, fileName i sizeBytes', async () => {
    const file = new File(['hello'], 'photo.jpg', { type: 'image/jpeg' })
    const result = await uploadEncryptedAttachment({
      channelId: 'ch-1',
      file,
      keyVersionId: 'kv-1',
      keyVersion: 1,
    })
    expect(result).toEqual({
      attachmentId: 'att-1',
      fileName: 'photo.jpg',
      sizeBytes: 5,
    })
  })

  it('envia el fitxer xifrat a la URL signada', async () => {
    const file = new File(['hello'], 'photo.jpg', { type: 'image/jpeg' })
    await uploadEncryptedAttachment({
      channelId: 'ch-1',
      file,
      keyVersionId: 'kv-1',
      keyVersion: 1,
    })
    expect(mockFetch).toHaveBeenCalledWith(
      '/api/channels/ch-1/upload/part',
      expect.objectContaining({ method: 'PUT' }),
    )
  })
})
