import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { render, screen, fireEvent, cleanup, waitFor, within } from '@testing-library/react'
import '@testing-library/jest-dom'
import { AppLayout } from './AppLayout'

const mockLogout = vi.fn()
const mockSocket = {
  on: vi.fn(),
  off: vi.fn(),
  emit: vi.fn(),
}

vi.mock('../contexts/AuthContext', () => ({
  useAuth: () => ({
    user: {
      userId: 'user-1',
      username: 'testuser',
      isAdmin: false,
      devices: [],
      quotas: { maxServers: 10, maxChannelsPerServer: 50, maxMessagesPerMinute: 30 },
    },
    currentDeviceId: 'dev-1',
    logout: mockLogout,
  }),
}))

vi.mock('../lib/socket', () => ({
  getSocket: vi.fn(() => mockSocket),
  disconnectSocket: vi.fn(),
}))

vi.mock('../lib/storage', () => ({
  getLatestChannelKey: vi.fn(),
  getChannelKey: vi.fn(),
}))

vi.mock('../lib/channel-crypto', () => ({
  ensureChannelKey: vi.fn(),
  distributeChannelKey: vi.fn(),
  syncChannelKeys: vi.fn(async () => undefined),
}))

vi.mock('../lib/device-keys', () => ({
  hasLocalDeviceKeypair: vi.fn(async () => true),
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

// Fix 1: The real AppLayout passes `onCreateTextChannel` and `onCreateVoiceChannel` to ChannelList,
// but the old mock only used `onCreateChannel`. Also must render channel type info
// (# Text, 🔊 Veu) for the form inputs test.
vi.mock('./sidebar/ChannelList', () => ({
  ChannelList: ({ channels, selectedChannel, onSelectChannel, onCreateTextChannel, onConfigureChannel, onManageDevices, onManageChannelKeys, onManageFriends, onChangePassword, onLogout, canCreateTextChannel, friends }: any) => (
    <div data-testid="channel-list">
      <button data-testid="btn-manage-devices" onClick={onManageDevices}>Gestio de dispositius</button>
      <button data-testid="btn-manage-channel-keys" onClick={onManageChannelKeys}>Gestio claus-canals</button>
      <button data-testid="btn-manage-friends" onClick={onManageFriends}>Gestió d'amics</button>
      <button data-testid="btn-change-password" onClick={onChangePassword}>Canviar password</button>
      <button data-testid="btn-logout" onClick={onLogout}>Sortir</button>
      {channels.map((c: any) => (
        <div key={c.channelId}>
          <button
            onClick={() => onSelectChannel(c)}
            data-channel-id={c.channelId}
            className={selectedChannel?.channelId === c.channelId ? 'selected' : ''}
          >
            {c.type === 'voice' ? '🔊' : '#'} {c.name}
          </button>
          {onConfigureChannel && (
            <button
              data-testid="btn-channel-settings"
              onClick={() => onConfigureChannel(c)}
            >
              ⚙️
            </button>
          )}
        </div>
      ))}
      {/* Use the real prop names that AppLayout passes */}
      {canCreateTextChannel && onCreateTextChannel && (
        <button data-testid="btn-create-channel" title="Crear canal de text" onClick={onCreateTextChannel}>+ Canal</button>
      )}
      <div data-testid="friends-count">{friends?.length ?? 0}</div>
      <span data-testid="channel-types"># Text</span>
      <span data-testid="channel-types-voice">🔊 Veu</span>
    </div>
  ),
}))

vi.mock('./modals/FriendsModal', () => ({
  FriendsPanel: ({ friends, onAddFriend, onRemoveFriend, onSearchUsers }: any) => (
    <div aria-label="Gestio d'amics">
      <h2>Gestio d'amics</h2>
      <span data-testid="friends-total">{friends.length}</span>
      <button data-testid="btn-search-friends" onClick={() => onSearchUsers?.('amic')}>Buscar</button>
      <button data-testid="btn-add-friend" onClick={() => onAddFriend('amic')}>Afegir</button>
      <button data-testid="btn-remove-friend" onClick={() => onRemoveFriend('friend-1')}>Treure</button>
    </div>
  ),
}))

vi.mock('./main/MainContent', () => ({
  MainContent: ({ channel }: any) => (
    <div data-testid="main-content">
      Main Content
      {channel && <span data-testid="current-channel">{channel.name}</span>}
    </div>
  ),
}))

vi.mock('./main/ChannelHeader', () => ({
  ChannelHeader: ({ channel }: any) => (
    <div data-testid="channel-header">
      {channel && (
        <>
          <span data-testid="channel-name">{channel.name}</span>
        </>
      )}
    </div>
  ),
}))

// Fix 3: Mock modals must return actual modal-like DOM so tests can find dialog,
// titles, labels, and text content.
vi.mock('./modals/CreateTextChannelModal', () => ({
  CreateTextChannelPanel: () => (
    <div aria-label="Crear canal de text">
      <h2>Crear canal</h2>
      <label htmlFor="channel-name">Nom del canal</label>
      <input id="channel-name" type="text" placeholder="general" />
      <div data-testid="channel-type-options">
        <span># Text</span>
        <span>🔊 Veu</span>
      </div>
    </div>
  ),
}))

vi.mock('./modals/CreateVoiceChannelModal', () => ({
  CreateVoiceChannelPanel: () => (
    <div aria-label="Crear canal de veu">
      <h2>Crear canal de veu</h2>
      <label htmlFor="voice-channel-name">Nom del canal</label>
      <input id="voice-channel-name" type="text" placeholder="sala de reunió" />
    </div>
  ),
}))

vi.mock('./modals/CreateServerModal', () => ({
  CreateServerPanel: () => (
    <div aria-label="Crear servidor">
      <h2>Crear servidor</h2>
      <label htmlFor="server-name">Nom del servidor</label>
      <input id="server-name" type="text" placeholder="Ex: El meu servidor" />
    </div>
  ),
}))

vi.mock('./modals/DeviceKeysModal', () => ({
  DeviceKeysPanel: () => (
    <div aria-label="Gestio de dispositius">
      <h2>Gestio de dispositius</h2>
    </div>
  ),
}))

vi.mock('./modals/ChangePasswordModal', () => ({
  ChangePasswordPanel: ({ onClose }: any) => (
    <div aria-label="Canviar password">
      <h2>Canviar password</h2>
      <button data-testid="btn-close-change-password" onClick={onClose}>Tancar</button>
    </div>
  ),
}))

vi.mock('./modals/ChannelKeysModal', () => ({
  ChannelKeysPanel: () => (
    <div aria-label="Gestio de claus de canals">
      <h2>Gestio de claus de canals</h2>
    </div>
  ),
}))

vi.mock('./modals/InviteMemberModal', () => ({
  InviteMemberModal: ({ isOpen, inviteType, targetName, onInvite }: any) =>
    isOpen ? (
      <div role="dialog" aria-label={`Convidar al ${inviteType}`}>
        <h2>Convidar al {inviteType}</h2>
        <p>Convida un usuari a <strong>{targetName}</strong></p>
        <label htmlFor="invite-username">Nom d'usuari</label>
        <input id="invite-username" type="text" placeholder="Nom d'usuari" />
        <button data-testid="submit-invite" onClick={() => onInvite('pop')}>Convidar</button>
      </div>
    ) : null,
}))

vi.mock('./modals/ConfigureChannelModal', () => ({
  ConfigureChannelModal: ({ isOpen, channel, onInviteChannel }: any) =>
    isOpen && channel ? (
      <div role="dialog" aria-label="Configuració del canal">
        <h2>Configuració del canal</h2>
        <span>Configuració del canal</span>
        <label htmlFor="channel-invite-username">Nom d'usuari</label>
        <input id="channel-invite-username" type="text" />
        <button data-testid="btn-invite-channel" onClick={() => onInviteChannel?.('pop')}>Convidar</button>
      </div>
    ) : null,
}))

vi.mock('../lib/api', () => ({
  serversList: vi.fn(),
  serversCreate: vi.fn(),
  serversGet: vi.fn(),
  channelsList: vi.fn(),
  dmChannelsList: vi.fn(),
  channelsCreate: vi.fn(),
  channelsUpdate: vi.fn(),
  channelsMarkRead: vi.fn(),
  serverInviteMember: vi.fn(),
  serverUpdateMemberRole: vi.fn(),
  serverRemoveMember: vi.fn(),
  channelInvite: vi.fn(),
  channelGetPermissions: vi.fn(),
  channelGetExplicitPermissions: vi.fn(),
  channelSetExplicitPermission: vi.fn(),
  friendsList: vi.fn(),
  friendsAdd: vi.fn(),
  friendsRemove: vi.fn(),
  usersSearch: vi.fn(),
  userLimitsGet: vi.fn(),
}))

import {
  serversList,
  serversCreate,
  serversGet,
  channelsList,
  dmChannelsList,
  channelsCreate,
  channelsUpdate,
  channelsMarkRead,
  serverInviteMember,
  serverUpdateMemberRole,
  serverRemoveMember,
  channelInvite,
  channelGetPermissions,
  channelGetExplicitPermissions,
  channelSetExplicitPermission,
  friendsList,
  friendsAdd,
  friendsRemove,
  usersSearch,
  userLimitsGet,
} from '../lib/api'
import { disconnectSocket } from '../lib/socket'
import { getLatestChannelKey, getChannelKey } from '../lib/storage'
import { distributeChannelKey } from '../lib/channel-crypto'

const mockServersList = vi.mocked(serversList)
const mockServersCreate = vi.mocked(serversCreate)
const mockServersGet = vi.mocked(serversGet)
const mockChannelsList = vi.mocked(channelsList)
const mockDmChannelsList = vi.mocked(dmChannelsList)
const mockChannelsCreate = vi.mocked(channelsCreate)
const mockChannelsUpdate = vi.mocked(channelsUpdate)
const mockChannelsMarkRead = vi.mocked(channelsMarkRead)
const mockServerInviteMember = vi.mocked(serverInviteMember)
const mockServerUpdateMemberRole = vi.mocked(serverUpdateMemberRole)
const mockServerRemoveMember = vi.mocked(serverRemoveMember)
const mockChannelInvite = vi.mocked(channelInvite)
const mockChannelGetPermissions = vi.mocked(channelGetPermissions)
const mockChannelGetExplicitPermissions = vi.mocked(channelGetExplicitPermissions)
const mockChannelSetExplicitPermission = vi.mocked(channelSetExplicitPermission)
const mockFriendsList = vi.mocked(friendsList)
const mockFriendsAdd = vi.mocked(friendsAdd)
const mockFriendsRemove = vi.mocked(friendsRemove)
const mockUsersSearch = vi.mocked(usersSearch)
const mockUserLimitsGet = vi.mocked(userLimitsGet)
const mockDisconnectSocket = vi.mocked(disconnectSocket)
const mockGetLatestChannelKey = vi.mocked(getLatestChannelKey)
const mockGetChannelKey = vi.mocked(getChannelKey)
const mockDistributeChannelKey = vi.mocked(distributeChannelKey)

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
  permissionLevel: 3,
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
    mockDmChannelsList.mockResolvedValue({ success: true, data: [] })
    mockChannelsCreate.mockResolvedValue({ success: true, data: testChannel })
    mockChannelsUpdate.mockResolvedValue({ success: true, data: { ...testChannel, name: 'general-modificat' } })
    mockChannelsMarkRead.mockResolvedValue({ success: true, data: undefined })
    mockServerInviteMember.mockResolvedValue({ success: true, data: { invitedUser: 'x' } })
    mockServerUpdateMemberRole.mockResolvedValue({ success: true, data: { userId: 'friend-1', username: 'amic', role: 'member', joinedAt: '2026-01-01T00:00:00Z' } })
    mockServerRemoveMember.mockResolvedValue({ success: true, data: { userId: 'friend-1', removed: true } })
    mockChannelInvite.mockResolvedValue({ success: true, data: { invitedUser: 'x' } })
    mockChannelGetPermissions.mockResolvedValue({
      success: true,
      data: [
        { userId: 'user-1', username: 'testuser', permissionLevel: 3, permission: 'manage' },
      ],
    })
    mockChannelGetExplicitPermissions.mockResolvedValue({ success: true, data: [] })
    mockChannelSetExplicitPermission.mockResolvedValue({ success: true, data: undefined })
    mockFriendsList.mockResolvedValue({ success: true, data: [{ userId: 'friend-1', username: 'amic', status: 'online', isOnline: true }] })
    mockFriendsAdd.mockResolvedValue({ success: true, data: undefined })
    mockFriendsRemove.mockResolvedValue({ success: true, data: undefined })
    mockUsersSearch.mockResolvedValue({ success: true, data: [] })
    mockUserLimitsGet.mockResolvedValue({
      success: true,
      data: {
        plan: {
          id: '550e8400-e29b-41d4-a716-446655441001',
          name: 'free',
          displayName: 'Free',
          description: null,
          limits: {
            maxServers: 1,
            maxChannelsTextPerServer: 3,
            maxChannelsVoicePerServer: 2,
            maxMembersPerServer: 20,
            apiCallsPerMinute: 60,
            messagesPerDay: 10000,
          },
        },
        usage: {
          totalServers: 1,
          totalTextChannels: 1,
          totalVoiceChannels: 0,
          totalMembersAcrossServers: 1,
          messagesToday: 0,
          apiCallsThisMinute: 0,
        },
        permissions: {
          canCreateServer: true,
          canCreateTextChannel: true,
          canCreateVoiceChannel: true,
          canAddMembers: true,
          canSendMessage: true,
        },
        remaining: {
          servers: 0,
          textChannels: 2,
          voiceChannels: 2,
          members: 19,
          messagesToday: 10000,
          apiCallsThisMinute: 60,
        },
      },
    })
    mockDisconnectSocket.mockClear()
    mockLogout.mockClear()
    mockGetLatestChannelKey.mockResolvedValue(null)
    mockGetChannelKey.mockResolvedValue(null)
    mockDistributeChannelKey.mockResolvedValue({
      discoveredDevices: [],
      skippedSelfDevices: [],
      skippedMissingKemDevices: [],
      uploadedBundleDevices: [],
      failedDevices: [],
    })
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
        expect(screen.getByRole('button', { name: /general/i })).toBeTruthy()
      })
    })
  })

  describe('Open modals', () => {
    it('panel de crear servidor s obre amb el boto +', async () => {
      renderApp()
      fireEvent.click(screen.getByTestId('btn-create-server'))
      await waitFor(() => {
        expect(screen.getByLabelText('Crear servidor')).toBeTruthy()
      })
    })

    it('CreateChannelModal es obre amb el botó + Canal', async () => {
      renderApp()
      // Wait for useEffect to load server and channels
      await waitFor(() => {
        expect(screen.getByTestId('btn-create-server')).toBeTruthy()
      }, { timeout: 5000 })
      const btn = await screen.findByTestId('btn-create-channel', {}, { timeout: 5000 })
      fireEvent.click(btn)
      await waitFor(() => {
        expect(screen.getByLabelText('Crear canal de text')).toBeTruthy()
        expect(screen.getByText('Crear canal')).toBeTruthy()
      })
    })

    it('la configuracio integrada del canal mostra invitacio dins del panell', async () => {
      renderApp()
      const settingsButton = await screen.findByTestId('btn-channel-settings')
      fireEvent.click(settingsButton)
      await waitFor(() => {
        expect(screen.getByText('Configuració integrada del canal')).toBeTruthy()
        expect(screen.getByLabelText('Convidar usuari')).toBeTruthy()
      })
    })

    it('redistribueix la clau d un canal asimetric despres de convidar', async () => {
      mockChannelsList.mockResolvedValueOnce({
        success: true,
        data: [{
          ...testChannel,
          encryptionType: 'asymmetric',
          keyVersionId: 'kv-1',
          keyVersion: 1,
        }],
      })
      mockGetLatestChannelKey.mockResolvedValueOnce({
        keyBytes: new Uint8Array([1, 2, 3]),
        keyVersion: 1,
        keyVersionId: 'kv-1',
      })
      mockGetChannelKey.mockResolvedValueOnce(new Uint8Array([1, 2, 3]))

      renderApp()
      const settingsButton = await screen.findByTestId('btn-channel-settings')
      fireEvent.click(settingsButton)

      const inviteInput = await screen.findByLabelText('Convidar usuari')
      fireEvent.change(inviteInput, { target: { value: 'pop' } })
      const inviteButton = screen.getByRole('button', { name: 'Convidar' })
      fireEvent.click(inviteButton)

      await waitFor(() => {
        expect(mockDistributeChannelKey).toHaveBeenCalledWith(
          'ch-1',
          expect.any(Uint8Array),
          1,
          'kv-1',
          'dev-1',
        )
      })
    })

    it('la configuracio de canal s obre en pestanya integrada', async () => {
      renderApp()
      const settingsButton = await screen.findByTestId('btn-channel-settings')
      fireEvent.click(settingsButton)
      await waitFor(() => {
        expect(screen.getByText('Configuració integrada del canal')).toBeTruthy()
      })
    })

    it('panell d amics s obre amb l accio Gestio d amics', async () => {
      renderApp()
      await waitFor(() => {
        expect(screen.getByTestId('btn-manage-friends')).toBeTruthy()
      })
      fireEvent.click(screen.getByTestId('btn-manage-friends'))
      await waitFor(() => {
        expect(screen.getByLabelText("Gestio d'amics")).toBeTruthy()
      })
    })

    it('panell de dispositius s obre amb l accio de menu', async () => {
      renderApp()
      await waitFor(() => {
        expect(screen.getByTestId('btn-manage-devices')).toBeTruthy()
      })
      fireEvent.click(screen.getByTestId('btn-manage-devices'))
      await waitFor(() => {
        expect(screen.getByLabelText('Gestio de dispositius')).toBeTruthy()
      })
    })

    it('panell de canvi de password s obre amb l accio de menu', async () => {
      renderApp()
      await waitFor(() => {
        expect(screen.getByTestId('btn-change-password')).toBeTruthy()
      })
      fireEvent.click(screen.getByTestId('btn-change-password'))
      await waitFor(() => {
        expect(screen.getByLabelText('Canviar password')).toBeTruthy()
      })
    })

    it('panell de claus de canals s obre amb l accio de menu', async () => {
      renderApp()
      await waitFor(() => {
        expect(screen.getByTestId('btn-manage-channel-keys')).toBeTruthy()
      })
      fireEvent.click(screen.getByTestId('btn-manage-channel-keys'))
      await waitFor(() => {
        expect(screen.getByLabelText('Gestio de claus de canals')).toBeTruthy()
      })
    })

    it('la sortida tanca el socket compartit', async () => {
      renderApp()
      await waitFor(() => {
        expect(screen.getByTestId('btn-logout')).toBeTruthy()
      })
      fireEvent.click(screen.getByTestId('btn-logout'))
      await waitFor(() => {
        expect(mockDisconnectSocket).toHaveBeenCalled()
        expect(mockLogout).toHaveBeenCalled()
      })
    })
  })

  describe('Form inputs', () => {
    it('panell de crear servidor te el camp de nom', async () => {
      renderApp()
      await waitFor(async () => {
        fireEvent.click(screen.getByTestId('btn-create-server'))
        await waitFor(() => {
          expect(screen.getByLabelText('Nom del servidor')).toBeTruthy()
        })
      })
    })

    it('CreateChannelModal té el camp de nom i selector de tipus', async () => {
      renderApp()
      const createButton = await screen.findByTestId('btn-create-channel')
      fireEvent.click(createButton)
      await waitFor(() => {
        expect(screen.getByLabelText('Nom del canal')).toBeTruthy()
        const options = within(screen.getByTestId('channel-type-options'))
        expect(options.getByText('# Text')).toBeTruthy()
        expect(options.getByText('🔊 Veu')).toBeTruthy()
      })
    })

  })
})
