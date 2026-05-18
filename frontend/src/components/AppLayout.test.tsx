import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react'
import '@testing-library/jest-dom'
import { AppLayout } from './AppLayout'

vi.mock('../contexts/AuthContext', () => ({
  useAuth: () => ({
    user: {
      userId: 'user-1',
      username: 'testuser',
      isAdmin: false,
      devices: [],
      quotas: { maxServers: 10, maxChannelsPerServer: 50, maxMessagesPerMinute: 30 },
    },
  }),
}))

vi.mock('./sidebar/ServerBar', () => ({
  ServerBar: ({ servers, selectedServer, onSelectServer, onCreateServer }: any) => (
    <div data-testid="server-bar">
      {servers.length === 0 && <div data-testid="no-servers">No tens servidors</div>}
      {servers.map((s: any) => (
        <button
          key={s.serverId}
          onClick={() => onSelectServer(s.serverId)}
          data-server-id={s.serverId}
          className={selectedServer === s.serverId ? 'selected' : ''}
        >
          {s.name}
        </button>
      ))}
      <button data-testid="btn-create-server" onClick={onCreateServer}>+</button>
    </div>
  ),
}))

vi.mock('./sidebar/ChannelList', () => ({
  ChannelList: ({ channels, selectedChannel, onSelectChannel, onCreateChannel, canCreateChannel }: any) => (
    <div data-testid="channel-list">
      {channels.map((c: any) => (
        <button
          key={c.channelId}
          onClick={() => onSelectChannel(c)}
          data-channel-id={c.channelId}
          className={selectedChannel?.channelId === c.channelId ? 'selected' : ''}
        >
          {c.name}
        </button>
      ))}
      {canCreateChannel && (
        <button data-testid="btn-create-channel" onClick={onCreateChannel}>+ Canal</button>
      )}
    </div>
  ),
}))

vi.mock('./main/MainContent', () => ({
  MainContent: () => <div data-testid="main-content">Main Content</div>,
}))

vi.mock('./main/ChannelHeader', () => ({
  ChannelHeader: ({ onConfigureChannel, onInviteChannel, channel }: any) => (
    <div data-testid="channel-header">
      {channel && (
        <>
          <span>{channel.name}</span>
          <button data-testid="btn-configure-channel" onClick={onConfigureChannel}>Configurar canal</button>
          <button data-testid="btn-invite-channel" onClick={onInviteChannel}>Convidar al canal</button>
        </>
      )}
    </div>
  ),
}))

vi.mock('../lib/api', () => ({
  serversList: vi.fn(),
  serversCreate: vi.fn(),
  serversGet: vi.fn(),
  channelsList: vi.fn(),
  channelsCreate: vi.fn(),
  channelsUpdate: vi.fn(),
  serverInviteMember: vi.fn(),
  channelInvite: vi.fn(),
}))

import {
  serversList,
  serversCreate,
  serversGet,
  channelsList,
  channelsCreate,
  channelsUpdate,
  serverInviteMember,
  channelInvite,
} from '../lib/api'

const mockServersList = vi.mocked(serversList)
const mockServersCreate = vi.mocked(serversCreate)
const mockServersGet = vi.mocked(serversGet)
const mockChannelsList = vi.mocked(channelsList)
const mockChannelsCreate = vi.mocked(channelsCreate)
const mockChannelsUpdate = vi.mocked(channelsUpdate)
const mockServerInviteMember = vi.mocked(serverInviteMember)
const mockChannelInvite = vi.mocked(channelInvite)

const testServer: any = {
  serverId: 'srv-1',
  name: 'El meu servidor',
  iconUrl: null,
  ownerId: 'user-1',
  memberCount: 1,
  myRole: 'owner',
  createdAt: '2026-01-01T00:00:00Z',
}

const testChannel: any = {
  channelId: 'ch-1',
  name: 'general',
  type: 'text',
  encryptionType: 'none',
  isPrivate: false,
  messageTTL: null,
  createdAt: '2026-01-01T00:00:00Z',
}

function renderApp() {
  return render(<AppLayout username="testuser" />)
}

describe('AppLayout', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mockServersList.mockResolvedValue({ success: true, data: [testServer] })
    mockServersGet.mockResolvedValue({ success: true, data: { ...testServer, members: [] } })
    mockChannelsList.mockResolvedValue({ success: true, data: [testChannel] })
    mockChannelsCreate.mockResolvedValue({ success: true, data: testChannel })
    mockChannelsUpdate.mockResolvedValue({ success: true, data: { ...testChannel, name: 'general-modificat' } })
    mockServerInviteMember.mockResolvedValue({ success: true, data: { invitedUser: 'x' } })
    mockChannelInvite.mockResolvedValue({ success: true, data: { invitedUser: 'x' } })
  })

  afterEach(() => {
    cleanup()
    document.body.innerHTML = ''
    document.body.style.overflow = ''
  })

  describe('renderitzat inicial', () => {
    it('mostra el nom d usuari', async () => {
      renderApp()
      await waitFor(() => {
        expect(screen.getByText(/testuser/)).toBeTruthy()
      })
    })

    it('mostra el servidor seleccionat', async () => {
      renderApp()
      await waitFor(() => {
        expect(screen.getByText('El meu servidor')).toBeTruthy()
      })
    })

    it('no renderitza cap modal inicialment', () => {
      renderApp()
      expect(screen.queryByRole('dialog')).toBeNull()
    })

    it('mostra botó de crear servidor', async () => {
      renderApp()
      await waitFor(() => {
        expect(screen.getByTestId('btn-create-server')).toBeTruthy()
      })
    })

    it('mostra canals del servidor', async () => {
      renderApp()
      await waitFor(() => {
        expect(screen.getByText('general')).toBeTruthy()
      })
    })
  })

  describe('Open modals', () => {
    it('CreateServerModal es obre amb el botó +', async () => {
      renderApp()
      await waitFor(() => {
        fireEvent.click(screen.getByTestId('btn-create-server'))
        expect(screen.getByRole('dialog')).toBeTruthy()
        expect(screen.getByText('Crear servidor')).toBeTruthy()
      })
    })

    it('CreateChannelModal es obre amb el botó + Canal', async () => {
      renderApp()
      await waitFor(() => {
        fireEvent.click(screen.getByTestId('btn-create-channel'))
        expect(screen.getByRole('dialog')).toBeTruthy()
        expect(screen.getByText('Crear canal')).toBeTruthy()
      })
    })

    it('InviteMemberModal servidor es obre amb el botó Convidar', async () => {
      renderApp()
      await waitFor(() => {
        fireEvent.click(screen.getByText(/Convidar al servidor/))
        expect(screen.getByRole('dialog')).toBeTruthy()
      })
    })

    it('InviteMemberModal canal es obre amb el botó del header', async () => {
      renderApp()
      await waitFor(() => {
        fireEvent.click(screen.getByTestId('btn-invite-channel'))
        expect(screen.getByRole('dialog')).toBeTruthy()
      })
    })

    it('ConfigureChannelModal es obre amb el botó Configurar canal', async () => {
      renderApp()
      await waitFor(() => {
        fireEvent.click(screen.getByTestId('btn-configure-channel'))
        expect(screen.getByRole('dialog')).toBeTruthy()
        expect(screen.getByText((c) => c.includes('Configuració del canal'))).toBeTruthy()
      })
    })
  })

  describe('Form inputs', () => {
    it('CreateServerModal té el camp de nom', async () => {
      renderApp()
      await waitFor(() => {
        fireEvent.click(screen.getByTestId('btn-create-server'))
        expect(screen.getByLabelText('Nom del servidor')).toBeTruthy()
      })
    })

    it('CreateChannelModal té el camp de nom i selector de tipus', async () => {
      renderApp()
      await waitFor(() => {
        fireEvent.click(screen.getByTestId('btn-create-channel'))
        expect(screen.getByLabelText('Nom del canal')).toBeTruthy()
        expect(screen.getByText('# Text')).toBeTruthy()
        expect(screen.getByText('🔊 Veu')).toBeTruthy()
      })
    })

    it('InviteMemberModal té el camp usuari', async () => {
      renderApp()
      await waitFor(() => {
        fireEvent.click(screen.getByText(/Convidar al servidor/))
        expect(screen.getByRole('textbox')).toBeTruthy()
      })
    })
  })
})
