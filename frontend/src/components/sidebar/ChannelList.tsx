import React, { useEffect, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Channel, FriendPresence, ServerMember, VoiceConnection, VoiceParticipant } from '../../types'
import { EncryptionIcon } from '../shared/EncryptionIcon'
import { LanguageSwitcher } from '../shared/LanguageSwitcher'

interface ChannelListProps {
  channels: Channel[]
  selectedChannel: Channel | null
  voiceConnection: VoiceConnection | null
  voicePresenceByChannel?: Record<string, VoiceParticipant[]>
  isMuted?: boolean
  isDeafened?: boolean
  isCameraOn?: boolean
  isScreenSharing?: boolean
  isMediaFileSharing?: boolean
  onToggleMute?: () => void
  onToggleDeafen?: () => void
  onToggleCamera?: () => void
  onToggleScreenShare?: () => void
  onToggleMediaFileShare?: () => void
  onSelectChannel: (channel: Channel) => void
  onStartDirectMessage?: (targetUserId: string, targetUsername: string) => void
  onConfigureChannel?: (channel: Channel) => void
  username: string
  onManageDevices?: () => void
  onManageChannelKeys?: () => void
  onManageFriends?: () => void
  onShowInvitations?: () => void
  pendingInvitationCount?: number
  onChangePassword?: () => void
  onManagePlan?: () => void
  onManagePermissions?: () => void
  onManageAdminUsers?: () => void
  onCollapseList?: () => void
  onLogout?: () => void
  onCreateTextChannel?: () => void
  onCreateVoiceChannel?: () => void
  canCreateTextChannel?: boolean
  canCreateVoiceChannel?: boolean
  canManageAdminUsers?: boolean
  friends?: FriendPresence[]
  serverMembers?: ServerMember[]
  serverMemberPresenceById?: Record<string, boolean>
  serverVersion?: string | null
}

export function ChannelList({
  channels,
  selectedChannel,
  voiceConnection,
  voicePresenceByChannel = {},
  isMuted = true,
  isDeafened = false,
  isCameraOn = false,
  isScreenSharing = false,
  isMediaFileSharing = false,
  onToggleMute,
  onToggleDeafen,
  onToggleCamera,
  onToggleScreenShare,
  onToggleMediaFileShare,
  onSelectChannel,
  onStartDirectMessage,
  onConfigureChannel,
  username,
  onManageDevices,
  onManageChannelKeys,
  onManageFriends,
  onShowInvitations,
  pendingInvitationCount = 0,
  onChangePassword,
  onManagePlan,
  onManagePermissions,
  onManageAdminUsers,
  onCollapseList,
  onLogout,
  onCreateTextChannel,
  onCreateVoiceChannel,
  canCreateTextChannel = false,
  canCreateVoiceChannel = false,
  canManageAdminUsers = false,
  friends = [],
  serverMembers = [],
  serverMemberPresenceById = {},
  serverVersion,
}: ChannelListProps) {
  const { t } = useTranslation()
  const [isUserMenuOpen, setIsUserMenuOpen] = useState(false)
  const [collapsedSections, setCollapsedSections] = useState({
    dm: false,
    text: false,
    voice: false,
    friends: false,
    members: false,
  })
  const userActionsRef = useRef<HTMLDivElement | null>(null)
  const voiceControlsEnabled = !!voiceConnection
  const activeVoiceChannel = voiceConnection
    ? channels.find((c) => c.channelId === voiceConnection.channelId)
    : null
  const canSpeak = voiceControlsEnabled && (activeVoiceChannel?.permissionLevel ?? 2) >= 2
  const dmChannels = channels.filter((c) => c.scope === 'dm' && c.type === 'text')
  const unreadDmChannels = dmChannels.filter((channel) => (channel.unreadCount ?? 0) > 0)
  const textChannels = channels.filter((c) => c.type === 'text' && c.scope !== 'dm')
  const voiceChannels = channels.filter((c) => c.type === 'voice' && c.scope !== 'dm')
  const sortedFriends = [...friends].sort((a, b) => {
    if (a.isOnline === b.isOnline) {
      return a.username.localeCompare(b.username)
    }
    return a.isOnline ? -1 : 1
  })
  const sortedServerMembers = [...serverMembers].sort((a, b) => {
    const aIsOnline = !!serverMemberPresenceById[a.userId]
    const bIsOnline = !!serverMemberPresenceById[b.userId]

    if (aIsOnline === bIsOnline) {
      return a.username.localeCompare(b.username)
    }
    return aIsOnline ? -1 : 1
  })

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (userActionsRef.current && !userActionsRef.current.contains(event.target as Node)) {
        setIsUserMenuOpen(false)
      }
    }

    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  // Get participants for a voice channel (from the active connection or mock)
  const getParticipants = (channel: Channel): VoiceParticipant[] => {
    if (voiceConnection && voiceConnection.channelId === channel.channelId) {
      return voiceConnection.participants
    }
    return voicePresenceByChannel[channel.channelId] ?? []
  }

  const toggleSection = (section: 'dm' | 'text' | 'voice' | 'friends' | 'members') => {
    setCollapsedSections((current) => ({
      ...current,
      [section]: !current[section],
    }))
  }

  return (
    <div className="channel-list">
      {/* User Info */}
      <div className="channel-list-user">
        <div className="user-avatar">{username.charAt(0).toUpperCase()}</div>
        <span className="user-name">{username}</span>
        <div className="user-actions" ref={userActionsRef}>
          <button
            className="channel-list-toggle-btn"
            onClick={() => onCollapseList?.()}
            title={t('channels.collapsePanel')}
            aria-label={t('channels.collapsePanel')}
          >
            ◀
          </button>
          <button
            className={`user-actions-toggle ${isUserMenuOpen ? 'active' : ''}`}
            onClick={() => setIsUserMenuOpen((current) => !current)}
            title={t('channels.userMenu')}
          >
            ⚙️
          </button>
          {isUserMenuOpen && (
            <div className="user-actions-menu">
              <button onClick={() => { setIsUserMenuOpen(false); onManageDevices?.() }}>{t('channels.menuDevices')}</button>
              <button onClick={() => { setIsUserMenuOpen(false); onManageChannelKeys?.() }}>{t('channels.menuChannelKeys')}</button>
              <button onClick={() => { setIsUserMenuOpen(false); onManageFriends?.() }}>{t('channels.menuFriends')}</button>
              <button onClick={() => { setIsUserMenuOpen(false); onShowInvitations?.() }} style={{ position: 'relative' }}>
                {t('channels.menuInvitations')}
                {pendingInvitationCount > 0 && (
                  <span className="channel-unread-badge" style={{ marginLeft: 6 }}>{pendingInvitationCount}</span>
                )}
              </button>
              <button onClick={() => { setIsUserMenuOpen(false); onChangePassword?.() }}>{t('channels.menuChangePassword')}</button>
              <button onClick={() => { setIsUserMenuOpen(false); onManagePlan?.() }}>{t('channels.menuPlan')}</button>
              <button onClick={() => { setIsUserMenuOpen(false); onManagePermissions?.() }}>{t('channels.menuPermissions')}</button>
              {canManageAdminUsers && (
                <button onClick={() => { setIsUserMenuOpen(false); onManageAdminUsers?.() }}>{t('channels.menuAdminUsers')}</button>
              )}
              <button onClick={() => { setIsUserMenuOpen(false); onLogout?.() }}>{t('channels.menuLogout')}</button>
              <div className="user-actions-language" onClick={(e) => e.stopPropagation()}>
                <LanguageSwitcher />
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Text Channels */}
      {unreadDmChannels.length > 0 && (
        <div className="channel-category">
          <div className="category-header">
            <button
              className="category-toggle"
              onClick={() => toggleSection('dm')}
              aria-expanded={!collapsedSections.dm}
              title={collapsedSections.dm ? t('channels.expandSection') : t('channels.collapseSection')}
            >
              <span className="category-name">{t('channels.catDms')}</span>
              <span className="category-chevron">{collapsedSections.dm ? '🔻' : '🔺'}</span>
            </button>
          </div>
          {!collapsedSections.dm && unreadDmChannels.map((channel) => (
            <div
              key={channel.channelId}
              className={`channel-item ${selectedChannel?.channelId === channel.channelId ? 'active' : ''}`}
              onClick={() => onSelectChannel(channel)}
            >
              <span className="channel-voice-icon">💬</span>
              <span className="channel-name">{channel.name}</span>
              <span className="channel-unread-badge">{channel.unreadCount}</span>
              <EncryptionIcon type={channel.encryptionType} />
            </div>
          ))}
        </div>
      )}

      {/* Text Channels */}
      <div className="channel-category">
        <div className="category-header">
          <button
            className="category-toggle"
            onClick={() => toggleSection('text')}
            aria-expanded={!collapsedSections.text}
            title={collapsedSections.text ? t('channels.expandSection') : t('channels.collapseSection')}
          >
            <span className="category-name">{t('channels.catText')}</span>
            <span className="category-chevron">{collapsedSections.text ? '🔻' : '🔺'}</span>
          </button>
          {canCreateTextChannel && onCreateTextChannel && (
            <button
              className="create-channel-btn"
              onClick={onCreateTextChannel}
              title={t('channels.createTextChannel')}
            >
              +
            </button>
          )}
        </div>
        {!collapsedSections.text && textChannels.map((channel) => (
          <div
            key={channel.channelId}
            className={`channel-item ${selectedChannel?.channelId === channel.channelId ? 'active' : ''}`}
            onClick={() => onSelectChannel(channel)}
          >
            <span className="channel-hash">#</span>
            <span className="channel-name">{channel.name}</span>
            {(channel.unreadCount ?? 0) > 0 && (
              <span className="channel-unread-badge">{channel.unreadCount}</span>
            )}
            <EncryptionIcon type={channel.encryptionType} />
            {onConfigureChannel && (channel.permissionLevel ?? 0) >= 3 && (
              <button
                className="channel-item-settings-btn"
                onClick={(event) => {
                  event.stopPropagation()
                  onConfigureChannel(channel)
                }}
                title={t('channels.channelConfig')}
              >
                ⚙️
              </button>
            )}
          </div>
        ))}
      </div>

      {/* Voice Channels */}
      <div className="channel-category">
        <div className="category-header">
          <button
            className="category-toggle"
            onClick={() => toggleSection('voice')}
            aria-expanded={!collapsedSections.voice}
            title={collapsedSections.voice ? t('channels.expandSection') : t('channels.collapseSection')}
          >
            <span className="category-name">{t('channels.catVoice')}</span>
            <span className="category-chevron">{collapsedSections.voice ? '🔻' : '🔺'}</span>
          </button>
          {canCreateVoiceChannel && onCreateVoiceChannel && (
            <button
              className="create-channel-btn"
              onClick={onCreateVoiceChannel}
              title={t('channels.createVoiceChannel')}
            >
              +
            </button>
          )}
        </div>
        {!collapsedSections.voice && voiceChannels.map((channel) => {
          const participants = getParticipants(channel)

          return (
            <div key={channel.channelId} className="voice-channel-wrapper">
              <div
                className={`channel-item voice ${selectedChannel?.channelId === channel.channelId ? 'active' : ''}`}
                onClick={() => onSelectChannel(channel)}
              >
                <span className="channel-voice-icon">🔊</span>
                <span className="channel-name">{channel.name}</span>
                <EncryptionIcon type={channel.encryptionType} />
                {onConfigureChannel && (channel.permissionLevel ?? 0) >= 3 && (
                  <button
                    className="channel-item-settings-btn"
                    onClick={(event) => {
                      event.stopPropagation()
                      onConfigureChannel(channel)
                    }}
                    title={t('channels.channelConfig')}
                  >
                    ⚙️
                  </button>
                )}
              </div>
              
              {/* Show connected users indented below the channel */}
              {participants.length > 0 && (
                <div className="voice-channel-participants">
                  {participants.map((p) => (
                    <div key={p.userId} className="voice-participant-indicator">
                      <span className={`participant-avatar-small ${p.isSpeaking ? 'speaking' : ''}`}>
                        {p.username.charAt(0).toUpperCase()}
                      </span>
                      <span className="participant-name-small">{p.username}</span>
                      {p.isSuppressed && <span className="deafened-dot" title={t('channels.micOff')}>🔕</span>}
                      {p.isDeafened && <span className="deafened-dot" title={t('channels.speakerOff')}>🔇</span>}
                    </div>
                  ))}
                </div>
              )}
            </div>
          )
        })}
      </div>

      {friends.length > 0 && (
        <div className="channel-category friends-category">
          <div className="category-header">
            <button
              className="category-toggle"
              onClick={() => toggleSection('friends')}
              aria-expanded={!collapsedSections.friends}
              title={collapsedSections.friends ? t('channels.expandSection') : t('channels.collapseSection')}
            >
              <span className="category-name">{t('channels.catFriends')}</span>
              <span className="category-chevron">{collapsedSections.friends ? '🔻' : '🔺'}</span>
            </button>
          </div>
          {!collapsedSections.friends && (
            <div className="friends-list">
              {sortedFriends.map((friend) => (
                <div
                  key={friend.userId}
                  className="friend-item friend-item-clickable"
                  onClick={() => onStartDirectMessage?.(friend.userId, friend.username)}
                  role={onStartDirectMessage ? 'button' : undefined}
                  tabIndex={onStartDirectMessage ? 0 : undefined}
                  onKeyDown={(event) => {
                    if (!onStartDirectMessage) return
                    if (event.key === 'Enter' || event.key === ' ') {
                      event.preventDefault()
                      onStartDirectMessage(friend.userId, friend.username)
                    }
                  }}
                >
                  <div
                    className={`friend-avatar ${friend.isOnline ? 'online' : 'offline'}`}
                    title={friend.isOnline ? t('channels.online') : t('channels.offline')}
                  >
                    {friend.username.charAt(0).toUpperCase()}
                  </div>
                  <div className="friend-meta">
                    <span className="friend-name">{friend.username}</span>
                  </div>
                  {onStartDirectMessage && (
                    <button
                      className="channel-item-settings-btn"
                      onClick={(event) => {
                        event.stopPropagation()
                        onStartDirectMessage(friend.userId, friend.username)
                      }}
                      title={t('channels.openDm')}
                    >
                      💬
                    </button>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      <div className="channel-category server-members-category friends-category">
        <div className="category-header">
          <button
            className="category-toggle"
            onClick={() => toggleSection('members')}
            aria-expanded={!collapsedSections.members}
            title={collapsedSections.members ? t('channels.expandSection') : t('channels.collapseSection')}
          >
            <span className="category-name">{t('channels.catMembers')}</span>
            <span className="category-chevron">{collapsedSections.members ? '🔻' : '🔺'}</span>
          </button>
        </div>
        {!collapsedSections.members && (serverMembers.length > 0 ? (
          <div className="friends-list">
            {sortedServerMembers.map((member) => {
              const isActive = !!serverMemberPresenceById[member.userId]
              return (
                <div
                  key={member.userId}
                  className="friend-item friend-item-clickable"
                  onClick={() => onStartDirectMessage?.(member.userId, member.username)}
                  role={onStartDirectMessage ? 'button' : undefined}
                  tabIndex={onStartDirectMessage ? 0 : undefined}
                  onKeyDown={(event) => {
                    if (!onStartDirectMessage) return
                    if (event.key === 'Enter' || event.key === ' ') {
                      event.preventDefault()
                      onStartDirectMessage(member.userId, member.username)
                    }
                  }}
                >
                  <div
                    className={`friend-avatar ${isActive ? 'online' : 'offline'}`}
                    title={isActive ? t('channels.online') : t('channels.offline')}
                  >
                    {member.username.charAt(0).toUpperCase()}
                  </div>
                  <div className="friend-meta">
                    <span className="friend-name">{member.username}</span>
                  </div>
                  {onStartDirectMessage && (
                    <button
                      className="channel-item-settings-btn"
                      onClick={(event) => {
                        event.stopPropagation()
                        onStartDirectMessage(member.userId, member.username)
                      }}
                      title={t('channels.openDm')}
                    >
                      💬
                    </button>
                  )}
                </div>
              )
            })}
          </div>
        ) : (
          <p className="friends-empty-state">{t('channels.noMembers')}</p>
        ))}
      </div>

      <div className="channel-list-footer">
        {voiceControlsEnabled && !canSpeak && (
          <div className="voice-listen-only-notice">
            {t('channels.listenOnly')}
          </div>
        )}
        {serverVersion && (
          <div className="channel-list-version">
            v{serverVersion}
          </div>
        )}
        <div className="channel-list-bottom-controls">
        <button
          className={`voice-user-btn ${isMuted ? 'active-off' : 'active-on'}`}
          onClick={onToggleMute}
          title={isMuted ? t('channels.micActivate') : t('channels.micMute')}
          disabled={!canSpeak}
        >
          🎤
        </button>
        <button
          className={`voice-user-btn ${isDeafened ? 'active-off' : 'active-on'}`}
          onClick={onToggleDeafen}
          title={isDeafened ? t('channels.soundActivate') : t('channels.soundMute')}
          disabled={!voiceControlsEnabled}
        >
          🔊
        </button>
        <button
          className={`voice-user-btn ${isCameraOn ? 'active-on' : 'active-off'}`}
          onClick={onToggleCamera}
          title={isCameraOn ? t('channels.cameraOff') : t('channels.cameraOn')}
          disabled={!canSpeak}
        >
          🎥
        </button>
        <button
          className={`voice-user-btn ${isScreenSharing ? 'active-on' : 'active-off'}`}
          onClick={onToggleScreenShare}
          title={isScreenSharing ? t('channels.screenStop') : t('channels.screenShare')}
          disabled={!canSpeak}
        >
          🖥️
        </button>
        <button
          className={`voice-user-btn ${isMediaFileSharing ? 'active-on' : 'active-off'}`}
          onClick={onToggleMediaFileShare}
          title={isMediaFileSharing ? t('channels.mediaStop') : t('channels.mediaShare')}
          disabled={!canSpeak}
        >
          🎬
        </button>
        </div>
      </div>
    </div>
  )
}
