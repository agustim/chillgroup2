import React, { useState, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import i18n from '../i18n'
import { ServerBar } from './sidebar/ServerBar'
import { ChannelList } from './sidebar/ChannelList'
import { MainContent } from './main/MainContent'
import { CreateServerPanel } from './modals/CreateServerModal'
import { CreateTextChannelPanel } from './modals/CreateTextChannelModal'
import { CreateVoiceChannelPanel } from './modals/CreateVoiceChannelModal'
import { InviteMemberModal } from './modals/InviteMemberModal'
import { DeviceKeysPanel } from './modals/DeviceKeysModal'
import { ChannelKeysPanel } from './modals/ChannelKeysModal'
import { PermissionsPanel } from './modals/PermissionsModal'
import { ChangePasswordPanel } from './modals/ChangePasswordModal'
import { FriendsPanel } from './modals/FriendsModal'
import { AdminUsersPanel } from './main/AdminUsersPanel'
import { LogoutBackupModal } from './modals/LogoutBackupModal'
import { ServerInvitationsModal } from './modals/ServerInvitationsModal'
import { ServerConfigPanel } from './main/ServerConfigPanel'
import { ChannelConfigPanel } from './main/ChannelConfigPanel'
import { LeaveServerModal } from './modals/LeaveServerModal'
import { useAppState } from '../hooks/useAppState'
import { serversList } from '../lib/api'

interface MobileLayoutProps {
  username: string
  onLogout?: () => void
}

export function MobileLayout({ username }: MobileLayoutProps) {
  const { t } = useTranslation()
  const [drawerOpen, setDrawerOpen] = useState(false)
  const mediaFileInputRef = useRef<HTMLInputElement>(null)

  const {
    user,
    currentDeviceId,
    serverVersion,
    servers,
    selectedServer,
    serverDetails,
    selectedServerInfo,
    canManageServer,
    canCreateServer,
    canCreateTextChannel,
    canCreateVoiceChannel,
    channels,
    selectedChannel,
    resolvedSelectedChannel,
    voiceConnection,
    activeVoiceChannel,
    voiceAsTextMode,
    liveKitMuted,
    liveKitDeafened,
    liveKitCameraOn,
    liveKitScreenSharing,
    liveKitMediaFileSharing,
    localVideoTrack,
    localScreenTrack,
    localMediaFileTrack,
    mediaFileName,
    mediaFileElementRef,
    remoteVideoTracks,
    liveKitError,
    voicePresenceByChannel,
    serverMemberPresenceById,
    friends,
    panel,
    setPanel,
    feedback,
    setFeedback,
    quotaWarning,
    setQuotaWarning,
    dmKeyActionBusy,
    showInviteServer,
    setShowInviteServer,
    leaveServerConfirm,
    setLeaveServerConfirm,
    leaveServerBusy,
    showServerInvitations,
    setShowServerInvitations,
    pendingInvitationCount,
    setPendingInvitationCount,
    showLogoutModal,
    setShowLogoutModal,
    pendingMemberRemovalId,
    setPendingMemberRemovalId,
    channelConfigName,
    setChannelConfigName,
    channelConfigMessageTTL,
    setChannelConfigMessageTTL,
    channelConfigIsPrivate,
    setChannelConfigIsPrivate,
    channelConfigPosition,
    setChannelConfigPosition,
    channelExplicitPermissionsLoading,
    canViewChannelExplicitPermissions,
    channelPermissionRows,
    updatingChannelPermissionUserId,
    handleUpdateChannelExplicitPermission,
    handleChannelConfigSave,
    handleSelectServer,
    handleOpenTextChannel,
    handleStartDirectMessage,
    handleCloseTextTab,
    handleRepairKey,
    handleRotateKey,
    handleUpdateDmTTL,
    handleVoiceChannelClick,
    handleLeaveVoiceChannel,
    handleToggleMute,
    handleToggleDeafen,
    handleToggleCamera,
    handleToggleScreenShare,
    handleStartMediaFileShare,
    handleStopMediaFileShare,
    handleSetParticipantLocalMuted,
    handleCreateServer,
    handleCreateServerSubmit,
    handleCreateTextChannel,
    handleCreateVoiceChannel,
    handleInviteServerSubmit,
    handleUpdateServerMemberRole,
    handleRemoveServerMember,
    handleInviteChannelSubmit,
    handleManageDevices,
    handleManageChannelKeys,
    handleManageFriends,
    handleChangePassword,
    handleManagePermissions,
    handleManageAdminUsers,
    handleOpenAdminServerConfig,
    handleAddFriend,
    handleRemoveFriend,
    handleSearchUsers,
    handleLogout,
    handleLogoutConfirm,
    handleConfigureChannel,
    handleDeleteChannel,
    handleServerMenuAction,
    handleLeaveServerConfirm,
    handleUnreadUpdated,
    toggleVoiceAsTextMode,
    refreshServers,
  } = useAppState()

  const closeDrawer = () => setDrawerOpen(false)

  const handleChannelSelect = (channel: typeof channels[0]) => {
    if (channel.type === 'voice') {
      handleVoiceChannelClick(channel)
    } else {
      handleOpenTextChannel(channel)
    }
    setPanel('none')
    closeDrawer()
  }

  const handleMediaFileShareToggle = () => {
    if (liveKitMediaFileSharing) {
      handleStopMediaFileShare()
    } else {
      mediaFileInputRef.current?.click()
    }
  }

  const isPanelOpen = panel !== 'none'
  const activeChannel = resolvedSelectedChannel ?? activeVoiceChannel ?? null
  const channelName = activeChannel?.name ?? null
  const encryptionLabel = activeChannel
    ? { none: '', symmetric: ' 🔑', asymmetric: ' 🔒' }[activeChannel.encryptionType] ?? ''
    : ''
  const channelPrefix = activeChannel?.type === 'voice' ? '🔊' : activeChannel?.scope === 'dm' ? '💬' : '#'

  return (
    <div className="mobile-layout">
      <input
        ref={mediaFileInputRef}
        type="file"
        accept="audio/*,video/*"
        style={{ display: 'none' }}
        onChange={(e) => {
          const file = e.target.files?.[0]
          if (file) void handleStartMediaFileShare(file)
          e.target.value = ''
        }}
      />
      {/* Drawer overlay */}
      {drawerOpen && (
        <div className="mobile-drawer-overlay" onClick={closeDrawer} />
      )}

      {/* Drawer */}
      <div className={`mobile-drawer ${drawerOpen ? 'open' : ''}`}>
        <ServerBar
          servers={servers}
          selectedServer={selectedServer}
          onSelectServer={(id) => { handleSelectServer(id); closeDrawer() }}
          onCreateServer={handleCreateServer}
          canCreateServer={canCreateServer}
          isChannelListCollapsed={false}
          onShowChannelList={() => {}}
          onServerAction={handleServerMenuAction}
        />
        {selectedServer && (
          <ChannelList
            channels={channels}
            selectedChannel={selectedChannel}
            voiceConnection={voiceConnection}
            voicePresenceByChannel={voicePresenceByChannel}
            isMuted={liveKitMuted}
            isDeafened={liveKitDeafened}
            isCameraOn={liveKitCameraOn}
            isScreenSharing={liveKitScreenSharing}
            isMediaFileSharing={liveKitMediaFileSharing}
            onToggleMute={handleToggleMute}
            onToggleDeafen={handleToggleDeafen}
            onToggleCamera={() => { void handleToggleCamera() }}
            onToggleScreenShare={() => { void handleToggleScreenShare() }}
            onToggleMediaFileShare={handleMediaFileShareToggle}
            onSelectChannel={handleChannelSelect}
            onStartDirectMessage={(uid, uname) => { handleStartDirectMessage(uid, uname); closeDrawer() }}
            onConfigureChannel={handleConfigureChannel}
            username={username}
            onLogout={handleLogout}
            onManageDevices={handleManageDevices}
            onManageChannelKeys={handleManageChannelKeys}
            onManageFriends={handleManageFriends}
            onShowInvitations={() => { setShowServerInvitations(true); closeDrawer() }}
            pendingInvitationCount={pendingInvitationCount}
            onChangePassword={handleChangePassword}
            onManagePlan={() => setPanel('planSettings')}
            onManagePermissions={handleManagePermissions}
            onManageAdminUsers={handleManageAdminUsers}
            onCollapseList={closeDrawer}
            onCreateTextChannel={canManageServer && canCreateTextChannel ? () => { setPanel('createTextChannel'); closeDrawer() } : undefined}
            onCreateVoiceChannel={canManageServer && canCreateVoiceChannel ? () => { setPanel('createVoiceChannel'); closeDrawer() } : undefined}
            canCreateTextChannel={canManageServer && canCreateTextChannel}
            canCreateVoiceChannel={canManageServer && canCreateVoiceChannel}
            canManageAdminUsers={user?.isAdmin ?? false}
            friends={friends}
            serverMembers={serverDetails?.members ?? []}
            serverMemberPresenceById={serverMemberPresenceById}
            serverVersion={serverVersion}
          />
        )}
      </div>

      {/* Main area */}
      <div className="mobile-main">
        {/* Mobile header */}
        <div className="mobile-header">
          <button
            className="mobile-hamburger"
            onClick={() => setDrawerOpen(true)}
            aria-label={t('appLayout.openMenu')}
          >
            ☰
          </button>
          <span className="mobile-header-title">
            {isPanelOpen
              ? panelTitle(panel)
              : channelName
                ? `${channelPrefix} ${channelName}${encryptionLabel}`
                : 'ChillGroup'}
          </span>
          {!isPanelOpen && activeChannel && canManageServer && (
            <button
              className="mobile-header-back"
              onClick={() => setPanel('channelConfig')}
              title={t('appLayout.configChannel')}
            >
              ⚙️
            </button>
          )}
          {isPanelOpen && (
            <button className="mobile-header-back" onClick={() => setPanel('none')}>
              ✕
            </button>
          )}
        </div>

        {/* Feedback banners */}
        {feedback && <div className="feedback-banner">{feedback}</div>}
        {liveKitError && <div className="feedback-banner" style={{ backgroundColor: '#ff4444' }}>{liveKitError}</div>}
        {quotaWarning && (
          <div className="feedback-banner feedback-banner--warning" style={{ backgroundColor: '#f59e0b', color: '#1f2937' }}>
            {quotaWarning}
            <button onClick={() => setQuotaWarning(null)} style={{ marginLeft: 8, background: 'none', border: 'none', cursor: 'pointer', fontWeight: 'bold' }}>✕</button>
          </div>
        )}

        {/* Panel content (full-screen on mobile) */}
        {panel === 'createServer' ? (
          <div className="mobile-panel-content">
            <CreateServerPanel onClose={() => setPanel('none')} onCreate={handleCreateServerSubmit} />
          </div>
        ) : panel === 'friends' ? (
          <div className="mobile-panel-content">
            <FriendsPanel
              friends={friends}
              onAddFriend={handleAddFriend}
              onRemoveFriend={handleRemoveFriend}
              onSearchUsers={handleSearchUsers}
            />
          </div>
        ) : panel === 'devices' ? (
          <div className="mobile-panel-content">
            <DeviceKeysPanel currentDeviceId={currentDeviceId} channels={channels} devices={user?.devices ?? []} />
          </div>
        ) : panel === 'changePassword' ? (
          <div className="mobile-panel-content">
            <ChangePasswordPanel onClose={() => setPanel('none')} />
          </div>
        ) : panel === 'channelKeys' ? (
          <div className="mobile-panel-content">
            <ChannelKeysPanel channels={channels} serverName={serverDetails?.name} />
          </div>
        ) : panel === 'createTextChannel' ? (
          <div className="mobile-panel-content">
            <CreateTextChannelPanel onClose={() => setPanel('none')} onCreate={handleCreateTextChannel} />
          </div>
        ) : panel === 'createVoiceChannel' ? (
          <div className="mobile-panel-content">
            <CreateVoiceChannelPanel onClose={() => setPanel('none')} onCreate={handleCreateVoiceChannel} />
          </div>
        ) : panel === 'adminUsers' ? (
          <div className="mobile-panel-content">
            <AdminUsersPanel
              isOpen={true}
              onClose={() => setPanel('none')}
              onFeedback={setFeedback}
              selectedServerId={selectedServer}
              availableServers={servers.map((server) => ({
                serverId: server.serverId,
                name: server.name,
                ownerId: server.ownerId,
                myRole: server.myRole,
                memberCount: server.memberCount,
              }))}
              onOpenServerConfig={handleOpenAdminServerConfig}
              onServerListRefresh={refreshServers}
            />
          </div>
        ) : panel === 'permissions' ? (
          <div className="mobile-panel-content">
            <button className="mobile-back-btn" onClick={() => setPanel('serverConfig')}>← {t('appLayout.tabServer')}</button>
            <PermissionsPanel server={serverDetails} channels={channels} currentDeviceId={currentDeviceId} />
          </div>
        ) : panel === 'serverConfig' && serverDetails ? (
          <div className="mobile-panel-content">
            <ServerConfigPanel
              serverDetails={serverDetails}
              channels={channels}
              canManageServer={!!canManageServer}
              currentUserId={user?.userId}
              pendingMemberRemovalId={pendingMemberRemovalId}
              onSetPendingMemberRemovalId={setPendingMemberRemovalId}
              onSearchUsers={handleSearchUsers}
              onInviteServerSubmit={handleInviteServerSubmit}
              onConfigureChannel={handleConfigureChannel}
              onUpdateServerMemberRole={handleUpdateServerMemberRole}
              onRemoveServerMember={handleRemoveServerMember}
              onOpenPermissions={() => setPanel('permissions')}
            />
          </div>
        ) : panel === 'channelConfig' && resolvedSelectedChannel ? (
          <div className="mobile-panel-content">
            <ChannelConfigPanel
              channel={resolvedSelectedChannel}
              channelConfigName={channelConfigName}
              setChannelConfigName={setChannelConfigName}
              channelConfigMessageTTL={channelConfigMessageTTL}
              setChannelConfigMessageTTL={setChannelConfigMessageTTL}
              channelConfigIsPrivate={channelConfigIsPrivate}
              setChannelConfigIsPrivate={setChannelConfigIsPrivate}
              channelConfigPosition={channelConfigPosition}
              setChannelConfigPosition={setChannelConfigPosition}
              onSave={handleChannelConfigSave}
              onSearchUsers={handleSearchUsers}
              onInviteChannelSubmit={handleInviteChannelSubmit}
              onDeleteChannel={handleDeleteChannel}
              onBackToServer={() => setPanel('serverConfig')}
              canViewChannelExplicitPermissions={canViewChannelExplicitPermissions}
              channelExplicitPermissionsLoading={channelExplicitPermissionsLoading}
              channelPermissionRows={channelPermissionRows}
              updatingChannelPermissionUserId={updatingChannelPermissionUserId}
              onUpdateChannelExplicitPermission={handleUpdateChannelExplicitPermission}
            />
          </div>
        ) : resolvedSelectedChannel ? (
          <MainContent
            channel={resolvedSelectedChannel}
            voiceConnection={voiceConnection}
            currentDeviceId={currentDeviceId}
            onLeaveVoice={handleLeaveVoiceChannel}
            onUnreadUpdated={handleUnreadUpdated}
            localVideoTrack={localVideoTrack}
            localScreenTrack={localScreenTrack}
            localMediaFileTrack={localMediaFileTrack}
            mediaFileName={mediaFileName}
            mediaFileElementRef={mediaFileElementRef}
            onStopMediaFileShare={handleStopMediaFileShare}
            onSetParticipantLocalMuted={handleSetParticipantLocalMuted}
            isMediaFileSharing={liveKitMediaFileSharing}
            remoteVideoTracks={remoteVideoTracks}
            voiceAsTextMode={voiceAsTextMode}
            onToggleVoiceAsTextMode={toggleVoiceAsTextMode}
            onRepairKey={handleRepairKey}
            onRotateKey={handleRotateKey}
            onUpdateDmTTL={handleUpdateDmTTL}
            keyActionBusy={dmKeyActionBusy}
            isChannelAdmin={canManageServer}
          />
        ) : voiceConnection ? (
          <MainContent
            channel={null}
            voiceConnection={voiceConnection}
            currentDeviceId={currentDeviceId}
            onLeaveVoice={handleLeaveVoiceChannel}
            localVideoTrack={localVideoTrack}
            localScreenTrack={localScreenTrack}
            localMediaFileTrack={localMediaFileTrack}
            mediaFileName={mediaFileName}
            mediaFileElementRef={mediaFileElementRef}
            onStopMediaFileShare={handleStopMediaFileShare}
            onSetParticipantLocalMuted={handleSetParticipantLocalMuted}
            isMediaFileSharing={liveKitMediaFileSharing}
            remoteVideoTracks={remoteVideoTracks}
            voiceAsTextMode={voiceAsTextMode}
            onToggleVoiceAsTextMode={toggleVoiceAsTextMode}
          />
        ) : (
          <div className="welcome-screen">
            <p>{t('appLayout.welcomeHintShort')}</p>
            <button className="mobile-open-drawer-btn" onClick={() => setDrawerOpen(true)}>
              {t('appLayout.openMenu')} ☰
            </button>
          </div>
        )}
      </div>

      {/* Modals */}
      {selectedServer && (
        <InviteMemberModal
          isOpen={showInviteServer}
          onClose={() => setShowInviteServer(false)}
          onInvite={handleInviteServerSubmit}
          onSearchUsers={handleSearchUsers}
          inviteType="server"
          targetName={selectedServerInfo?.name ?? selectedServer}
        />
      )}

      {showServerInvitations && (
        <ServerInvitationsModal
          onClose={() => { setShowServerInvitations(false); setPendingInvitationCount(0) }}
          onAccepted={() => {
            setShowServerInvitations(false)
            setPendingInvitationCount(0)
            void refreshServers()
          }}
        />
      )}

      {showLogoutModal && (
        <LogoutBackupModal
          username={username}
          onConfirm={handleLogoutConfirm}
          onCancel={() => setShowLogoutModal(false)}
        />
      )}

      <LeaveServerModal
        confirm={leaveServerConfirm}
        busy={leaveServerBusy}
        onConfirm={handleLeaveServerConfirm}
        onCancel={() => setLeaveServerConfirm(null)}
      />
    </div>
  )
}

function panelTitle(panel: string): string {
  const titles: Record<string, string> = {
    serverConfig: i18n.t('appLayout.tabServer'),
    channelConfig: i18n.t('appLayout.mobileTitleChannel'),
    devices: i18n.t('appLayout.tabDevices'),
    adminUsers: i18n.t('appLayout.tabUsers'),
    permissions: i18n.t('appLayout.tabPermissions'),
    friends: i18n.t('appLayout.tabFriends'),
    createServer: i18n.t('appLayout.tabNewServer'),
    changePassword: i18n.t('appLayout.mobileTitlePassword'),
    channelKeys: i18n.t('appLayout.tabKeys'),
    createTextChannel: i18n.t('appLayout.mobileTitleNewText'),
    createVoiceChannel: i18n.t('appLayout.mobileTitleNewVoice'),
  }
  return titles[panel] ?? ''
}
