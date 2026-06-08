import React, { useRef } from 'react'
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
import { PanelTab } from './shared/PanelTab'
import { ServerConfigPanel } from './main/ServerConfigPanel'
import { ChannelConfigPanel } from './main/ChannelConfigPanel'
import { LeaveServerModal } from './modals/LeaveServerModal'
import { useAppState } from '../hooks/useAppState'
export type { PanelType } from '../types'

interface AppLayoutProps {
  username: string
  onLogout?: () => void
}

export function AppLayout({ username }: AppLayoutProps) {
  const {
    user,
    currentDeviceId,
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
    isChannelListCollapsed,
    openTextTabs,
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
    showTabBar,
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
    setIsChannelListCollapsed,
    refreshServers,
  } = useAppState()

  const mediaFileInputRef = useRef<HTMLInputElement>(null)

  const handleMediaFileShareToggle = () => {
    if (liveKitMediaFileSharing) {
      handleStopMediaFileShare()
    } else {
      mediaFileInputRef.current?.click()
    }
  }

  return (
    <div className="app-layout">
      <ServerBar
        servers={servers}
        selectedServer={selectedServer}
        onSelectServer={handleSelectServer}
        onCreateServer={handleCreateServer}
        canCreateServer={canCreateServer}
        isChannelListCollapsed={isChannelListCollapsed}
        onShowChannelList={() => setIsChannelListCollapsed(false)}
        onServerAction={handleServerMenuAction}
      />

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

      {selectedServer && !isChannelListCollapsed && (
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
          onSelectChannel={(channel) => {
            if (channel.type === 'voice') {
              handleVoiceChannelClick(channel)
            } else {
              handleOpenTextChannel(channel)
            }
          }}
          onStartDirectMessage={handleStartDirectMessage}
          onConfigureChannel={handleConfigureChannel}
          username={username}
          onLogout={handleLogout}
          onManageDevices={handleManageDevices}
          onManageChannelKeys={handleManageChannelKeys}
          onManageFriends={handleManageFriends}
          onShowInvitations={() => setShowServerInvitations(true)}
          pendingInvitationCount={pendingInvitationCount}
          onChangePassword={handleChangePassword}
          onManagePermissions={handleManagePermissions}
          onManageAdminUsers={handleManageAdminUsers}
          onCollapseList={() => setIsChannelListCollapsed(true)}
          onCreateTextChannel={canManageServer && canCreateTextChannel ? () => setPanel('createTextChannel') : undefined}
          onCreateVoiceChannel={canManageServer && canCreateVoiceChannel ? () => setPanel('createVoiceChannel') : undefined}
          canCreateTextChannel={canManageServer && canCreateTextChannel}
          canCreateVoiceChannel={canManageServer && canCreateVoiceChannel}
          canManageAdminUsers={user?.isAdmin ?? false}
          friends={friends}
          serverMembers={serverDetails?.members ?? []}
          serverMemberPresenceById={serverMemberPresenceById}
        />
      )}

      <div className="main-content-area">
        {showTabBar && (
          <div className="main-content-tabs">
            {openTextTabs.map((channel) => (
              <div
                key={channel.channelId}
                className={`main-content-tab ${resolvedSelectedChannel?.channelId === channel.channelId ? 'active' : ''}`}
                onClick={() => { setPanel('none'); handleOpenTextChannel(channel) }}
              >
                <span>#</span>
                <span>{channel.name}</span>
                {(channel.unreadCount ?? 0) > 0 && (
                  <span className="channel-unread-badge">{channel.unreadCount}</span>
                )}
                <button
                  type="button"
                  className="main-content-tab-close"
                  onClick={(event) => { event.stopPropagation(); handleCloseTextTab(channel.channelId) }}
                  title="Tancar pestanya"
                >
                  ✕
                </button>
              </div>
            ))}

            {activeVoiceChannel && (
              <div
                className={`main-content-tab ${resolvedSelectedChannel?.channelId === activeVoiceChannel.channelId ? 'active' : ''}`}
                onClick={() => { setPanel('none'); handleOpenTextChannel(activeVoiceChannel) }}
              >
                <span>🔊</span>
                <span>{activeVoiceChannel.name}</span>
                <button
                  type="button"
                  className="main-content-tab-close"
                  onClick={(event) => { event.stopPropagation(); handleLeaveVoiceChannel() }}
                  title="Surt del canal de veu"
                >
                  ✕
                </button>
              </div>
            )}

            <PanelTab icon="⚙️" label="Servidor" isActive={panel === 'serverConfig'} onClick={() => setPanel('serverConfig')} onClose={() => setPanel('none')} />
            <PanelTab icon="#" label={resolvedSelectedChannel?.name ?? ''} isActive={panel === 'channelConfig' && !!resolvedSelectedChannel} onClick={() => setPanel('channelConfig')} onClose={() => setPanel('none')} />
            <PanelTab icon="🛡️" label="Permisos" isActive={panel === 'permissions'} onClick={() => setPanel('permissions')} onClose={() => setPanel('none')} />
            <PanelTab icon="🛠️" label="Usuaris" isActive={panel === 'adminUsers'} onClick={() => setPanel('adminUsers')} onClose={() => setPanel('none')} />
            <PanelTab icon="➕" label="Nou servidor" isActive={panel === 'createServer'} onClick={() => setPanel('createServer')} onClose={() => setPanel('none')} />
            <PanelTab icon="👥" label="Amics" isActive={panel === 'friends'} onClick={() => setPanel('friends')} onClose={() => setPanel('none')} />
            <PanelTab icon="📱" label="Dispositius" isActive={panel === 'devices'} onClick={() => setPanel('devices')} onClose={() => setPanel('none')} />
            <PanelTab icon="🔒" label="Password" isActive={panel === 'changePassword'} onClick={() => setPanel('changePassword')} onClose={() => setPanel('none')} />
            <PanelTab icon="🔑" label="Claus" isActive={panel === 'channelKeys'} onClick={() => setPanel('channelKeys')} onClose={() => setPanel('none')} />
            <PanelTab icon="#" label="Nou text" isActive={panel === 'createTextChannel'} onClick={() => setPanel('createTextChannel')} onClose={() => setPanel('none')} />
            <PanelTab icon="🔊" label="Nou veu" isActive={panel === 'createVoiceChannel'} onClick={() => setPanel('createVoiceChannel')} onClose={() => setPanel('none')} />
          </div>
        )}

        {feedback && <div className="feedback-banner">{feedback}</div>}
        {liveKitError && <div className="feedback-banner" style={{ backgroundColor: '#ff4444' }}>{liveKitError}</div>}
        {quotaWarning && (
          <div className="feedback-banner feedback-banner--warning" style={{ backgroundColor: '#f59e0b', color: '#1f2937' }}>
            {quotaWarning}
            <button onClick={() => setQuotaWarning(null)} style={{ marginLeft: 8, background: 'none', border: 'none', cursor: 'pointer', fontWeight: 'bold' }}>✕</button>
          </div>
        )}

        {panel === 'createServer' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Crear servidor</h3>
            </div>
            <CreateServerPanel onClose={() => setPanel('none')} onCreate={handleCreateServerSubmit} />
          </div>
        ) : panel === 'friends' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Gestio d'amics</h3>
            </div>
            <FriendsPanel
              friends={friends}
              onAddFriend={handleAddFriend}
              onRemoveFriend={handleRemoveFriend}
              onSearchUsers={handleSearchUsers}
            />
          </div>
        ) : panel === 'devices' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Gestio de dispositius</h3>
            </div>
            <DeviceKeysPanel currentDeviceId={currentDeviceId} channels={channels} devices={user?.devices ?? []} />
          </div>
        ) : panel === 'changePassword' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Canviar password</h3>
            </div>
            <ChangePasswordPanel onClose={() => setPanel('none')} />
          </div>
        ) : panel === 'channelKeys' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Gestió de claus de canals</h3>
            </div>
            <ChannelKeysPanel channels={channels} serverName={serverDetails?.name} />
          </div>
        ) : panel === 'createTextChannel' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Crear canal de text</h3>
            </div>
            <CreateTextChannelPanel onClose={() => setPanel('none')} onCreate={handleCreateTextChannel} />
          </div>
        ) : panel === 'createVoiceChannel' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Crear canal de veu</h3>
            </div>
            <CreateVoiceChannelPanel onClose={() => setPanel('none')} onCreate={handleCreateVoiceChannel} />
          </div>
        ) : panel === 'adminUsers' ? (
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
        ) : panel === 'permissions' ? (
          <div className="panel admin-users-panel">
            <div className="admin-users-panel-header">
              <h3>Permisos i accessos</h3>
              <button className="admin-panel-tab" onClick={() => setPanel('serverConfig')}>
                Tornar a servidor
              </button>
            </div>
            <PermissionsPanel server={serverDetails} channels={channels} currentDeviceId={currentDeviceId} />
          </div>
        ) : panel === 'serverConfig' && serverDetails ? (
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
        ) : panel === 'channelConfig' && resolvedSelectedChannel ? (
          <ChannelConfigPanel
            channel={resolvedSelectedChannel}
            channelConfigName={channelConfigName}
            setChannelConfigName={setChannelConfigName}
            channelConfigMessageTTL={channelConfigMessageTTL}
            setChannelConfigMessageTTL={setChannelConfigMessageTTL}
            channelConfigIsPrivate={channelConfigIsPrivate}
            setChannelConfigIsPrivate={setChannelConfigIsPrivate}
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
            <h1>Benvingut/da, {username}!</h1>
            <p>Selecciona un servidor i un canal per començar.</p>
          </div>
        )}

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
    </div>
  )
}
