import { describe, it, expect, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import '@testing-library/jest-dom'
import { ChannelList } from './ChannelList'

const baseProps = {
  channels: [],
  selectedChannel: null,
  voiceConnection: null,
  onSelectChannel: vi.fn(),
  username: 'testuser',
}

describe('ChannelList', () => {
  it('nomes mostra el bloc de missatges directes amb unread messages', () => {
    render(
      <ChannelList
        {...baseProps}
        channels={[
          {
            channelId: 'dm-1',
            name: 'amic',
            type: 'text',
            encryptionType: 'asymmetric',
            scope: 'dm',
            dmPeerUserId: 'friend-1',
            messageTTL: 86400,
            isPrivate: true,
            unreadCount: 0,
            createdAt: '2026-01-01T00:00:00Z',
          },
          {
            channelId: 'dm-2',
            name: 'nou-missatge',
            type: 'text',
            encryptionType: 'asymmetric',
            scope: 'dm',
            dmPeerUserId: 'friend-2',
            messageTTL: 86400,
            isPrivate: true,
            unreadCount: 3,
            createdAt: '2026-01-01T00:00:00Z',
          },
        ]}
      />
    )

    expect(screen.queryByText('💬 MISSATGES DIRECTES')).toBeTruthy()
    expect(screen.queryByText('amic')).toBeNull()
    expect(screen.getByText('nou-missatge')).toBeTruthy()
  })

  it('obre un DM en clicar una fila d amic o membre del servidor', () => {
    const onStartDirectMessage = vi.fn()

    render(
      <ChannelList
        {...baseProps}
        onStartDirectMessage={onStartDirectMessage}
        friends={[
          {
            userId: 'friend-1',
            username: 'amic',
            status: 'online',
            isOnline: true,
          },
        ]}
        serverMembers={[
          {
            userId: 'member-1',
            username: 'membre',
            role: 'member',
            joinedAt: '2026-01-01T00:00:00Z',
          },
        ]}
      />
    )

    fireEvent.click(screen.getByText('amic'))
    fireEvent.click(screen.getByText('membre'))

    expect(onStartDirectMessage).toHaveBeenCalledWith('friend-1', 'amic')
    expect(onStartDirectMessage).toHaveBeenCalledWith('member-1', 'membre')
  })
})
