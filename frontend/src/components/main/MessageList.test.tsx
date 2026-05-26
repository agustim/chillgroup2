import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import '@testing-library/jest-dom'
import { MessageList } from './MessageList'

vi.mock('../../lib/api', () => ({
  messagesList: vi.fn(),
}))

vi.mock('../../lib/channel-crypto', () => ({
  decryptMessagesForChannel: vi.fn(),
}))

vi.mock('../../lib/logger', () => ({
  logger: {
    debug: vi.fn(),
  },
}))

import { messagesList } from '../../lib/api'
import { decryptMessagesForChannel } from '../../lib/channel-crypto'

const mockMessagesList = vi.mocked(messagesList)
const mockDecryptMessagesForChannel = vi.mocked(decryptMessagesForChannel)

describe('MessageList', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockMessagesList.mockResolvedValue({
      success: true,
      data: {
        data: [
          {
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
          },
        ],
        pagination: {
          has_more: false,
          next_cursor: null,
          prev_cursor: null,
        },
      },
    })
    mockDecryptMessagesForChannel.mockResolvedValue({
      'msg-1': 'Hello **bold** [site](https://example.com)\n\n- one\n- two',
    })
  })

  it('renderitza markdown segur dins dels missatges', async () => {
    render(
      <MessageList
        channelId="ch-1"
        scope="server"
        encryptionType="asymmetric"
      />
    )

    await waitFor(() => {
      expect(screen.getByText('bold')).toBeTruthy()
    })

    expect(screen.getByRole('link', { name: 'site' })).toHaveAttribute('href', 'https://example.com')
    expect(screen.getByText('one')).toBeTruthy()
    expect(screen.getByText('two')).toBeTruthy()
    expect(screen.getByText('🔐v7')).toBeTruthy()
  })
})
