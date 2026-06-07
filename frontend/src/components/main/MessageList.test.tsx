import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import '@testing-library/jest-dom'
import { MessageList } from './MessageList'

// jsdom no implementa scrollIntoView
Element.prototype.scrollIntoView = vi.fn()

// jsdom no implementa IntersectionObserver
const mockIntersectionObserver = vi.fn(() => ({
  observe: vi.fn(),
  unobserve: vi.fn(),
  disconnect: vi.fn(),
}))
vi.stubGlobal('IntersectionObserver', mockIntersectionObserver)

vi.mock('../../lib/api', () => ({
  messagesList: vi.fn(),
  attachmentGetDownload: vi.fn(),
  channelsMarkRead: vi.fn(),
}))

vi.mock('../../lib/channel-crypto', () => ({
  decryptMessagesForChannel: vi.fn(),
}))

vi.mock('../../lib/attachments', () => ({
  downloadAndDecryptAttachment: vi.fn(),
  decryptAttachmentToBlob: vi.fn(),
}))

vi.mock('../../lib/logger', () => ({
  logger: {
    debug: vi.fn(),
    error: vi.fn(),
  },
}))

vi.mock('../../contexts/AuthContext', () => ({
  useAuth: vi.fn(() => ({
    user: { userId: 'user-1', username: 'testuser', isAdmin: false, devices: [], quotas: {} },
    currentDeviceId: 'dev-1',
  })),
}))

import { messagesList, attachmentGetDownload } from '../../lib/api'
import { decryptMessagesForChannel } from '../../lib/channel-crypto'
import { decryptAttachmentToBlob } from '../../lib/attachments'

const mockMessagesList = vi.mocked(messagesList)
const mockDecryptMessagesForChannel = vi.mocked(decryptMessagesForChannel)
const mockAttachmentGetDownload = vi.mocked(attachmentGetDownload)
const mockDecryptAttachmentToBlob = vi.mocked(decryptAttachmentToBlob)

const BASE_MSG = {
  messageId: 'msg-1',
  channelId: 'ch-1',
  senderUserId: 'user-1',
  senderUsername: 'testuser',
  senderDeviceId: 'dev-1',
  encryptedPayload: 'ignored',
  iv: 'iv',
  keyVersion: 7,
  timestamp: '2026-01-01T00:00:00Z',
  expiresAt: '2026-01-02T00:00:00Z',
  editedAt: null,
  deletedAt: null,
}

const BASE_ATTACHMENT_RESPONSE = {
  attachmentId: 'att-1',
  fileName: 'photo.jpg',
  mimeType: 'image/jpeg',
  sizeBytes: 12345,
  createdAt: '2026-01-01T00:00:00Z',
  downloadUrl: '/api/channels/ch-1/attachments/att-1/data',
  crypto: {
    algorithm: 'aes-256-gcm',
    fileIv: 'aXY=',
    wrappedFileKey: 'a2V5',
    keyVersionId: 'kv-1',
    keyVersion: 1,
    chunkSizeBytes: 5242880,
    chunkCount: 1,
    ciphertextSha256: 'abc123',
  },
}

function setupBaseMessage(extra: object = {}) {
  mockMessagesList.mockResolvedValue({
    success: true,
    data: {
      data: [{ ...BASE_MSG, ...extra }],
      pagination: { has_more: false, next_cursor: null, prev_cursor: null },
    },
  } as any)
  mockDecryptMessagesForChannel.mockResolvedValue({ 'msg-1': 'Hola' })
}

// jsdom no implementa URL.createObjectURL
;(URL as any).createObjectURL = vi.fn(() => 'blob:test-url')
;(URL as any).revokeObjectURL = vi.fn()

describe('MessageList', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    ;(URL as any).createObjectURL = vi.fn(() => 'blob:test-url')
    ;(URL as any).revokeObjectURL = vi.fn()
  })

  it('renderitza markdown segur dins dels missatges', async () => {
    mockMessagesList.mockResolvedValue({
      success: true,
      data: {
        data: [{ ...BASE_MSG }],
        pagination: { has_more: false, next_cursor: null, prev_cursor: null },
      },
    } as any)
    mockDecryptMessagesForChannel.mockResolvedValue({
      'msg-1': 'Hello **bold** [site](https://example.com)\n\n- one\n- two',
    })

    render(<MessageList channelId="ch-1" scope="server" encryptionType="asymmetric" />)

    await waitFor(() => {
      expect(screen.getByText('bold')).toBeTruthy()
    })

    expect(screen.getByRole('link', { name: 'site' })).toHaveAttribute('href', 'https://example.com')
    expect(screen.getByText('one')).toBeTruthy()
    expect(screen.getByText('two')).toBeTruthy()
    expect(screen.getByText('🔐v7')).toBeTruthy()
  })

  describe('adjunts sense thumbnail', () => {
    it('mostra chip de descàrrega per fitxers normals', async () => {
      setupBaseMessage({ attachmentIds: ['att-1'] })
      mockAttachmentGetDownload.mockResolvedValue({
        success: true,
        data: { ...BASE_ATTACHMENT_RESPONSE, mimeType: 'application/pdf', fileName: 'doc.pdf' },
      } as any)

      render(<MessageList channelId="ch-1" scope="server" encryptionType="asymmetric" />)

      await waitFor(() => {
        expect(screen.getByText(/doc\.pdf/)).toBeTruthy()
      })
      expect(screen.queryByRole('img')).toBeNull()
    })

    it('mostra chip de descàrrega per imatge sense thumbnail_attachment_id', async () => {
      setupBaseMessage({ attachmentIds: ['att-1'] })
      mockAttachmentGetDownload.mockResolvedValue({
        success: true,
        data: { ...BASE_ATTACHMENT_RESPONSE },
      } as any)

      render(<MessageList channelId="ch-1" scope="server" encryptionType="asymmetric" />)

      await waitFor(() => {
        expect(screen.getByText(/photo\.jpg/)).toBeTruthy()
      })
      expect(screen.queryByRole('img')).toBeNull()
    })
  })

  describe('adjunts amb thumbnail', () => {
    const THUMB_RESPONSE = {
      ...BASE_ATTACHMENT_RESPONSE,
      attachmentId: 'thumb-1',
      fileName: 'thumb_photo.jpg',
    }

    beforeEach(() => {
      setupBaseMessage({ attachmentIds: ['att-1'] })

      mockAttachmentGetDownload.mockImplementation((_channelId, attachmentId) => {
        if (attachmentId === 'att-1') {
          return Promise.resolve({
            success: true,
            data: { ...BASE_ATTACHMENT_RESPONSE, thumbnail_attachment_id: 'thumb-1' },
          } as any)
        }
        if (attachmentId === 'thumb-1') {
          return Promise.resolve({ success: true, data: THUMB_RESPONSE } as any)
        }
        return Promise.resolve({ success: false, error: { message: 'not found' } } as any)
      })

      mockDecryptAttachmentToBlob.mockResolvedValue(new Blob(['jpeg'], { type: 'image/jpeg' }))
    })

    it('mostra <img> amb blob URL del thumbnail', async () => {
      render(<MessageList channelId="ch-1" scope="server" encryptionType="asymmetric" />)

      await waitFor(() => {
        const img = screen.getByRole('img')
        expect(img).toHaveAttribute('src', 'blob:test-url')
      })
    })

    it('mostra link de descàrrega juntament amb el thumbnail', async () => {
      render(<MessageList channelId="ch-1" scope="server" encryptionType="asymmetric" />)

      await waitFor(() => {
        expect(screen.getByRole('img')).toBeTruthy()
        expect(screen.getByText(/photo\.jpg/)).toBeTruthy()
      })
    })

    it('desencripta el thumbnail amb les claus correctes', async () => {
      render(<MessageList channelId="ch-1" scope="server" encryptionType="asymmetric" />)

      await waitFor(() => {
        expect(mockDecryptAttachmentToBlob).toHaveBeenCalledWith(
          expect.objectContaining({
            downloadUrl: THUMB_RESPONSE.downloadUrl,
            crypto: expect.objectContaining({
              wrappedFileKey: THUMB_RESPONSE.crypto.wrappedFileKey,
              fileIv: THUMB_RESPONSE.crypto.fileIv,
            }),
          }),
        )
      })
    })

    it('no mostra img si el fetch del thumbnail falla', async () => {
      mockAttachmentGetDownload.mockImplementation((_channelId, attachmentId) => {
        if (attachmentId === 'att-1') {
          return Promise.resolve({
            success: true,
            data: { ...BASE_ATTACHMENT_RESPONSE, thumbnail_attachment_id: 'thumb-1' },
          } as any)
        }
        return Promise.resolve({ success: false, error: { message: 'error' } } as any)
      })

      render(<MessageList channelId="ch-1" scope="server" encryptionType="asymmetric" />)

      await waitFor(() => {
        expect(screen.getByText(/photo\.jpg/)).toBeTruthy()
      })
      expect(screen.queryByRole('img')).toBeNull()
    })
  })
})
