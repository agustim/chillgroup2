import React, { useState } from 'react'
import { ServerBar } from './sidebar/ServerBar'
import { ChannelList } from './sidebar/ChannelList'
import { MainContent } from './main/MainContent'
import { ChannelHeader } from './main/ChannelHeader'
import { Channel } from '../types'

interface AppLayoutProps {
  username: string
  onLogout?: () => void
}

export function AppLayout({ username, onLogout }: AppLayoutProps) {
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null)
  const [selectedServer, setSelectedServer] = useState<string | null>(null)
  const [voiceJoined, setVoiceJoined] = useState(false)

  const channels: Channel[] = [
    {
      channelId: 'ch-1',
      name: 'general',
      type: 'text',
      encryptionType: 'none',
      messageTTL: null,
      isPrivate: false,
      createdAt: '2026-01-01T00:00:00Z',
    },
    {
      channelId: 'ch-2',
      name: 'tecnologia',
      type: 'text',
      encryptionType: 'none',
      messageTTL: null,
      isPrivate: false,
      createdAt: '2026-01-01T00:00:00Z',
    },
    {
      channelId: 'ch-3',
      name: 'general-veus',
      type: 'voice',
      encryptionType: 'none',
      messageTTL: null,
      isPrivate: false,
      createdAt: '2026-01-01T00:00:00Z',
    },
  ]

  const handleChannelSelect = (channel: Channel) => {
    if (channel.type === 'voice') {
      setVoiceJoined(!voiceJoined)
    }
    setSelectedChannel(channel)
  }

  return (
    <div className="app-layout">
      {/* Server Bar */}
      <ServerBar
        servers={[
          { serverId: 'srv-1', name: 'ChillGroup', iconUrl: null, ownerId: 'user-1', memberCount: 5, myRole: 'owner', createdAt: '2026-01-01T00:00:00Z' },
        ]}
        selectedServer={selectedServer}
        onSelectServer={setSelectedServer}
      />

      {/* Channel List */}
      <ChannelList
        channels={channels}
        selectedChannel={selectedChannel}
        onSelectChannel={handleChannelSelect}
        username={username}
        onLogout={onLogout}
      />

      {/* Main Content Area */}
      <div className="main-content-area">
        {selectedChannel ? (
          <>
            <ChannelHeader channel={selectedChannel} />
            <MainContent
              channel={selectedChannel}
              voiceJoined={voiceJoined}
              onToggleVoice={() => setVoiceJoined(!voiceJoined)}
            />
          </>
        ) : (
          <div className="welcome-screen">
            <h1>Benvingut/da, {username}!</h1>
            <p>Selecciona un canal per començar a xerrar</p>
          </div>
        )}
      </div>
    </div>
  )
}