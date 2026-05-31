import { beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen, waitFor } from '@testing-library/react'
import '@testing-library/jest-dom'

import { ChannelKeysPanel } from './ChannelKeysModal'
import { channelsList, serversList } from '../../lib/api'

vi.mock('../../lib/device-keys', () => ({
  deleteSymmetricChannelKey: vi.fn(async () => undefined),
  exportAsymmetricChannelKeys: vi.fn(async () => ''),
  exportSymmetricChannelKeys: vi.fn(async () => ''),
  importAsymmetricChannelKeys: vi.fn(async () => 0),
  importSymmetricChannelKeys: vi.fn(async () => 0),
  listSymmetricChannelKeys: vi.fn(async () => [
    {
      channelId: 'ch-1',
      keyVersion: 3,
      acquiredAt: 1710000000000,
      preview: 'k1',
    },
  ]),
  listChannelKeys: vi.fn(async () => [
    {
      channelId: 'ch-2',
      keyVersion: 7,
      keyVersionId: 'kv-2',
      type: 'asymmetric',
      acquiredAt: 1710000000001,
      expiresAt: null,
    },
  ]),
}))

vi.mock('../../lib/api', () => ({
  serversList: vi.fn(),
  channelsList: vi.fn(),
}))

describe('ChannelKeysPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('mostra nom de servidor i canal, i deixa channelId només al title', async () => {
    vi.mocked(serversList).mockResolvedValue({
      success: true,
      data: [
        {
          serverId: 'srv-1',
          name: 'Workspace',
          iconUrl: null,
          ownerId: 'u-1',
          memberCount: 2,
          myRole: 'owner',
          createdAt: '2026-01-01T00:00:00Z',
        },
        {
          serverId: 'srv-2',
          name: 'Altre',
          iconUrl: null,
          ownerId: 'u-1',
          memberCount: 1,
          myRole: 'owner',
          createdAt: '2026-01-01T00:00:00Z',
        },
      ],
    })

    vi.mocked(channelsList).mockImplementation(async (serverId: string) => {
      if (serverId === 'srv-1') {
        return {
          success: true,
          data: [
            {
              channelId: 'ch-1',
              name: 'general',
              type: 'text',
              encryptionType: 'symmetric',
              messageTTL: null,
              isPrivate: false,
              createdAt: '2026-01-01T00:00:00Z',
            },
            {
              channelId: 'ch-2',
              name: 'secrets',
              type: 'text',
              encryptionType: 'asymmetric',
              messageTTL: null,
              isPrivate: true,
              createdAt: '2026-01-01T00:00:00Z',
            },
          ],
        }
      }

      return { success: true, data: [] }
    })

    render(<ChannelKeysPanel channels={[]} />)

    await waitFor(() => {
      expect(screen.getByText('Canal Workspace · #general · v3')).toBeInTheDocument()
      expect(screen.getByText('Canal Workspace · #secrets · v7')).toBeInTheDocument()
    })

    expect(screen.queryByText('Canal desconegut')).not.toBeInTheDocument()
    expect(screen.queryByText('ch-1')).not.toBeInTheDocument()
    expect(screen.queryByText('ch-2')).not.toBeInTheDocument()

    expect(screen.getByText('Canal Workspace · #general · v3')).toHaveAttribute('title', 'ch-1')
    expect(screen.getByText('Canal Workspace · #secrets · v7')).toHaveAttribute('title', 'ch-2')

    expect(channelsList).toHaveBeenCalledTimes(1)
    expect(channelsList).toHaveBeenCalledWith('srv-1')
  })
})
