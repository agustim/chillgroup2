import React, { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  adminPlansCreate,
  adminPlansDelete,
  adminPlansList,
  adminPlansUpdate,
  adminServersCreate,
  adminServersDelete,
  adminServersList,
  adminServersUpdate,
  adminUserLimitsGet,
  adminUsersCreate,
  adminUsersDelete,
  adminUsersList,
  adminUsersUpdatePlan,
  adminUsersUpdateRole,
  invitationsCreate,
  invitationsList,
  adminPlanChangeRequestsList,
  adminPlanChangeRequestApprove,
  adminPlanChangeRequestReject,
  type AdminPlanInput,
  type AdminPlanItem,
  type AdminServerItem,
  type AdminUserLimitsInfo,
  type AdminUserItem,
  type AdminUserRole,
  type InvitationListItem,
  type PlanChangeRequest,
} from '../../lib/api'
import { getSocket } from '../../lib/socket'
import { Button } from '../shared/Button'

interface AdminServerOption {
  serverId: string
  name: string
}

interface AdminUsersPanelProps {
  isOpen: boolean
  onClose: () => void
  onFeedback: (message: string) => void
  selectedServerId?: string | null
  availableServers?: AdminServerOption[]
  onOpenServerConfig?: (serverId: string) => void
  onServerListRefresh?: () => Promise<void>
}

const DEFAULT_PLAN_ID = '550e8400-e29b-41d4-a716-446655441001'
const DEFAULT_PLAN_INPUT: AdminPlanInput = {
  name: '',
  displayName: '',
  description: '',
  maxServers: 1,
  maxChannelsTextPerServer: 3,
  maxChannelsVoicePerServer: 2,
  maxMembersPerServer: 20,
  apiCallsPerMinute: 60,
  messagesPerDay: 10000,
}

type ActiveTab = 'users' | 'invitations' | 'servers' | 'plans' | 'planRequests'

export function AdminUsersPanel({
  isOpen,
  onClose,
  onFeedback,
  selectedServerId,
  availableServers = [],
  onOpenServerConfig,
  onServerListRefresh,
}: AdminUsersPanelProps) {
  const { t } = useTranslation()
  const [activeTab, setActiveTab] = useState<ActiveTab>('users')
  const [error, setError] = useState('')

  // Users state
  const [users, setUsers] = useState<AdminUserItem[]>([])
  const [loadingUsers, setLoadingUsers] = useState(false)
  const [newUsername, setNewUsername] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [newRole, setNewRole] = useState<AdminUserRole>('user')
  const [newPlanId, setNewPlanId] = useState(DEFAULT_PLAN_ID)
  const [isCreatingUser, setIsCreatingUser] = useState(false)
  const [expandedTierUserId, setExpandedTierUserId] = useState<string | null>(null)
  const [tierByUserId, setTierByUserId] = useState<Record<string, AdminUserLimitsInfo>>({})
  const [loadingTierUserId, setLoadingTierUserId] = useState<string | null>(null)

  // Plans state
  const [plans, setPlans] = useState<AdminPlanItem[]>([])
  const [loadingPlans, setLoadingPlans] = useState(false)
  const [creatingPlan, setCreatingPlan] = useState(false)
  const [newPlan, setNewPlan] = useState<AdminPlanInput>(DEFAULT_PLAN_INPUT)
  const [editingPlanId, setEditingPlanId] = useState<string | null>(null)
  const [editingPlan, setEditingPlan] = useState<AdminPlanInput>(DEFAULT_PLAN_INPUT)
  const [savingPlanId, setSavingPlanId] = useState<string | null>(null)
  const [pendingDeletePlanId, setPendingDeletePlanId] = useState<string | null>(null)
  const [deletingPlanId, setDeletingPlanId] = useState<string | null>(null)

  // Invitations state
  const [invitations, setInvitations] = useState<InvitationListItem[]>([])
  const [loadingInvitations, setLoadingInvitations] = useState(false)
  const [inviteMaxUses, setInviteMaxUses] = useState(1)
  const [inviteServerId, setInviteServerId] = useState<string>('')
  const [lastCreatedCode, setLastCreatedCode] = useState<string | null>(null)
  const [isCreatingInvite, setIsCreatingInvite] = useState(false)

  // Servers state
  const [adminServers, setAdminServers] = useState<AdminServerItem[]>([])
  const [loadingServers, setLoadingServers] = useState(false)
  const [newServerName, setNewServerName] = useState('')
  const [newServerIconUrl, setNewServerIconUrl] = useState('')
  const [creatingServer, setCreatingServer] = useState(false)
  const [editingServerId, setEditingServerId] = useState<string | null>(null)
  const [editingServerName, setEditingServerName] = useState('')
  const [editingServerIconUrl, setEditingServerIconUrl] = useState('')
  const [editingServerUsesDefaultLiveKit, setEditingServerUsesDefaultLiveKit] = useState(true)
  const [editingServerLiveKitHost, setEditingServerLiveKitHost] = useState('')
  const [editingServerLiveKitApiKey, setEditingServerLiveKitApiKey] = useState('')
  const [editingServerLiveKitApiSecret, setEditingServerLiveKitApiSecret] = useState('')
  const [savingServerId, setSavingServerId] = useState<string | null>(null)
  const [pendingDeleteServerId, setPendingDeleteServerId] = useState<string | null>(null)
  const [deletingServerId, setDeletingServerId] = useState<string | null>(null)

  // Plan change requests state
  const [planRequests, setPlanRequests] = useState<PlanChangeRequest[]>([])
  const [loadingPlanRequests, setLoadingPlanRequests] = useState(false)
  const [planRequestsBadge, setPlanRequestsBadge] = useState(0)
  const [resolvingRequestId, setResolvingRequestId] = useState<string | null>(null)
  const [requestAdminNote, setRequestAdminNote] = useState<Record<string, string>>({})

  const loadPlanRequests = async () => {
    setLoadingPlanRequests(true)
    const result = await adminPlanChangeRequestsList()
    setLoadingPlanRequests(false)
    if (!result.success) { setError(result.error.message); return }
    setPlanRequests(result.data)
    setPlanRequestsBadge(result.data.filter((r) => r.status === 'pending').length)
  }

  const loadUsers = async () => {
    setLoadingUsers(true)
    const result = await adminUsersList()
    setLoadingUsers(false)
    if (!result.success) { setError(result.error.message); return }
    setUsers(result.data)
  }

  const loadPlans = async () => {
    setLoadingPlans(true)
    const result = await adminPlansList()
    setLoadingPlans(false)
    if (!result.success) {
      setError(result.error.message)
      return
    }
    setPlans(result.data)

    const nextPlanId = result.data.some((plan) => plan.id === newPlanId)
      ? newPlanId
      : (result.data.find((plan) => plan.name === 'free')?.id ?? result.data[0]?.id ?? DEFAULT_PLAN_ID)
    setNewPlanId(nextPlanId)
  }

  const loadInvitations = async () => {
    setLoadingInvitations(true)
    const result = await invitationsList()
    setLoadingInvitations(false)
    if (!result.success) { setError(result.error.message); return }
    setInvitations(result.data)
  }

  const loadAdminServers = async () => {
    setLoadingServers(true)
    const result = await adminServersList()
    setLoadingServers(false)
    if (!result.success) {
      setError(result.error.message)
      return
    }
    setAdminServers(result.data)
  }

  useEffect(() => {
    if (!isOpen) return
    setError('')
    setNewUsername('')
    setNewPassword('')
    setNewRole('user')
    setNewPlanId(DEFAULT_PLAN_ID)
    setInviteMaxUses(1)
    setInviteServerId(selectedServerId ?? '')
    setLastCreatedCode(null)
    setNewServerName('')
    setNewServerIconUrl('')
    setEditingServerId(null)
    setEditingServerName('')
    setEditingServerIconUrl('')
    setEditingServerUsesDefaultLiveKit(true)
    setEditingServerLiveKitHost('')
    setEditingServerLiveKitApiKey('')
    setEditingServerLiveKitApiSecret('')
    setPendingDeleteServerId(null)
    setCreatingPlan(false)
    setNewPlan(DEFAULT_PLAN_INPUT)
    setEditingPlanId(null)
    setEditingPlan(DEFAULT_PLAN_INPUT)
    setPendingDeletePlanId(null)
    void loadUsers()
    void loadPlans()
    void loadInvitations()
    void loadAdminServers()
    void loadPlanRequests()
  }, [isOpen, selectedServerId])

  useEffect(() => {
    const socket = getSocket()
    const handlePlanChangeRequest = () => {
      setPlanRequestsBadge((n) => n + 1)
      void loadPlanRequests()
    }
    socket.on('plan_change_request', handlePlanChangeRequest)
    return () => { socket.off('plan_change_request', handlePlanChangeRequest) }
  }, [])

  const handleCreateUser = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const username = newUsername.trim()
    if (!username || !newPassword.trim()) { setError(t('admin.errUserPassRequired')); return }
    setIsCreatingUser(true)
    setError('')
    const result = await adminUsersCreate(username, newPassword, newRole, newPlanId)
    setIsCreatingUser(false)
    if (!result.success) { setError(result.error.message); return }
    onFeedback(t('admin.userCreated', { username: result.data.username }))
    setNewUsername('')
    setNewPassword('')
    setNewRole('user')
    setNewPlanId(plans.find((plan) => plan.name === 'free')?.id ?? plans[0]?.id ?? DEFAULT_PLAN_ID)
    await loadUsers()
  }

  const handlePlanFieldChange = (field: keyof AdminPlanInput, value: string | number) => {
    setNewPlan((current) => ({ ...current, [field]: value }))
  }

  const handleEditPlanFieldChange = (field: keyof AdminPlanInput, value: string | number) => {
    setEditingPlan((current) => ({ ...current, [field]: value }))
  }

  const handleCreatePlan = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const payload: AdminPlanInput = {
      ...newPlan,
      name: newPlan.name.trim(),
      displayName: newPlan.displayName.trim(),
      description: newPlan.description?.trim() ? newPlan.description.trim() : null,
      maxServers: Number(newPlan.maxServers),
      maxChannelsTextPerServer: Number(newPlan.maxChannelsTextPerServer),
      maxChannelsVoicePerServer: Number(newPlan.maxChannelsVoicePerServer),
      maxMembersPerServer: Number(newPlan.maxMembersPerServer),
      apiCallsPerMinute: Number(newPlan.apiCallsPerMinute),
      messagesPerDay: Number(newPlan.messagesPerDay),
    }

    if (!payload.name || !payload.displayName) {
      setError(t('admin.errPlanNamesRequired'))
      return
    }

    setCreatingPlan(true)
    setError('')
    const result = await adminPlansCreate(payload)
    setCreatingPlan(false)
    if (!result.success) {
      setError(result.error.message)
      return
    }

    onFeedback(t('admin.planCreated', { name: result.data.displayName }))
    setNewPlan(DEFAULT_PLAN_INPUT)
    await loadPlans()
  }

  const startEditPlan = (plan: AdminPlanItem) => {
    setEditingPlanId(plan.id)
    setEditingPlan({
      name: plan.name,
      displayName: plan.displayName,
      description: plan.description ?? '',
      maxServers: plan.maxServers,
      maxChannelsTextPerServer: plan.maxChannelsTextPerServer,
      maxChannelsVoicePerServer: plan.maxChannelsVoicePerServer,
      maxMembersPerServer: plan.maxMembersPerServer,
      apiCallsPerMinute: plan.apiCallsPerMinute,
      messagesPerDay: plan.messagesPerDay,
    })
    setPendingDeletePlanId(null)
  }

  const handleUpdatePlan = async (planId: string) => {
    const payload: AdminPlanInput = {
      ...editingPlan,
      name: editingPlan.name.trim(),
      displayName: editingPlan.displayName.trim(),
      description: editingPlan.description?.trim() ? editingPlan.description.trim() : null,
      maxServers: Number(editingPlan.maxServers),
      maxChannelsTextPerServer: Number(editingPlan.maxChannelsTextPerServer),
      maxChannelsVoicePerServer: Number(editingPlan.maxChannelsVoicePerServer),
      maxMembersPerServer: Number(editingPlan.maxMembersPerServer),
      apiCallsPerMinute: Number(editingPlan.apiCallsPerMinute),
      messagesPerDay: Number(editingPlan.messagesPerDay),
    }

    if (!payload.name || !payload.displayName) {
      setError(t('admin.errPlanNamesRequired'))
      return
    }

    setSavingPlanId(planId)
    setError('')
    const result = await adminPlansUpdate(planId, payload)
    setSavingPlanId(null)
    if (!result.success) {
      setError(result.error.message)
      return
    }

    onFeedback(t('admin.planUpdated'))
    setEditingPlanId(null)
    setEditingPlan(DEFAULT_PLAN_INPUT)
    await loadPlans()
  }

  const handleDeletePlan = async (planId: string) => {
    setDeletingPlanId(planId)
    setError('')
    const result = await adminPlansDelete(planId)
    setDeletingPlanId(null)
    if (!result.success) {
      setError(result.error.message)
      return
    }

    onFeedback(t('admin.planDeleted'))
    setPendingDeletePlanId(null)
    if (editingPlanId === planId) {
      setEditingPlanId(null)
      setEditingPlan(DEFAULT_PLAN_INPUT)
    }
    await loadPlans()
  }

  const handleRoleChange = async (userId: string, role: AdminUserRole) => {
    const result = await adminUsersUpdateRole(userId, role)
    if (!result.success) { setError(result.error.message); return }
    setUsers((c) => c.map((u) => (u.userId === userId ? { ...u, role } : u)))
    onFeedback(t('admin.roleUpdated'))
  }

  const handlePlanChange = async (userId: string, planId: string) => {
    const result = await adminUsersUpdatePlan(userId, planId)
    if (!result.success) { setError(result.error.message); return }
    setUsers((c) => c.map((u) => (u.userId === userId ? { ...u, planId } : u)))
    onFeedback(t('admin.planUpdated'))
  }

  const handleDeleteUser = async (userId: string, username: string) => {
    const result = await adminUsersDelete(userId)
    if (!result.success) { setError(result.error.message); return }
    setUsers((c) => c.filter((u) => u.userId !== userId))
    onFeedback(t('admin.userDeleted', { username }))
  }

  const handleToggleTiers = async (userId: string) => {
    if (expandedTierUserId === userId) {
      setExpandedTierUserId(null)
      return
    }

    setExpandedTierUserId(userId)
    if (tierByUserId[userId]) {
      return
    }

    setLoadingTierUserId(userId)
    const result = await adminUserLimitsGet(userId)
    setLoadingTierUserId(null)

    if (!result.success) {
      setError(result.error.message)
      return
    }

    setTierByUserId((current) => ({ ...current, [userId]: result.data }))
  }

  const handleCreateInvitation = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const maxUses = Number.isFinite(inviteMaxUses) ? Math.max(1, Math.floor(inviteMaxUses)) : 1
    setIsCreatingInvite(true)
    setError('')
    const result = await invitationsCreate(maxUses, inviteServerId || null)
    setIsCreatingInvite(false)
    if (!result.success) { setError(result.error.message); return }
    setLastCreatedCode(result.data.code)
    onFeedback(t('admin.inviteCreated'))
    await loadInvitations()
  }

  const handleCopyCode = async (code: string) => {
    try {
      await navigator.clipboard.writeText(code)
      onFeedback(t('admin.codeCopied'))
    } catch {
      onFeedback(t('admin.codeCopyFail'))
    }
  }

  const handleCreateServer = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const name = newServerName.trim()
    if (!name) {
      setError(t('createServer.errNameRequired'))
      return
    }

    setCreatingServer(true)
    setError('')
    const iconUrl = newServerIconUrl.trim() || null
    const result = await adminServersCreate(name, iconUrl)
    setCreatingServer(false)

    if (!result.success) {
      setError(result.error.message)
      return
    }

    onFeedback(t('admin.serverCreated', { name: result.data.name }))
    setNewServerName('')
    setNewServerIconUrl('')
    await loadAdminServers()
    if (onServerListRefresh) {
      await onServerListRefresh()
    }
  }

  const handleStartEditServer = (server: AdminServerItem) => {
    setEditingServerId(server.serverId)
    setEditingServerName(server.name)
    setEditingServerIconUrl(server.iconUrl ?? '')
    setEditingServerUsesDefaultLiveKit(!server.livekitConfig?.isOverride)
    setEditingServerLiveKitHost(server.livekitConfig?.host ?? '')
    setEditingServerLiveKitApiKey(server.livekitConfig?.apiKey ?? '')
    setEditingServerLiveKitApiSecret('')
    setPendingDeleteServerId(null)
  }

  const handleSaveServer = async (serverId: string) => {
    const name = editingServerName.trim()
    if (!name) {
      setError(t('createServer.errNameRequired'))
      return
    }

    setSavingServerId(serverId)
    setError('')
    const iconUrl = editingServerIconUrl.trim() || null
    let livekitHost: string | null | undefined
    let livekitApiKey: string | null | undefined
    let livekitApiSecret: string | null | undefined

    if (editingServerUsesDefaultLiveKit) {
      livekitHost = null
      livekitApiKey = null
      livekitApiSecret = null
    } else {
      livekitHost = editingServerLiveKitHost.trim()
      livekitApiKey = editingServerLiveKitApiKey.trim()
      livekitApiSecret = editingServerLiveKitApiSecret.trim() || undefined

      if (!livekitHost || !livekitApiKey) {
        setSavingServerId(null)
        setError(t('admin.errLiveKit'))
        return
      }
    }

    const result = await adminServersUpdate(serverId, name, iconUrl, livekitHost, livekitApiKey, livekitApiSecret)
    setSavingServerId(null)

    if (!result.success) {
      setError(result.error.message)
      return
    }

    onFeedback(t('admin.serverUpdated'))
    setEditingServerId(null)
    setEditingServerName('')
    setEditingServerIconUrl('')
    setEditingServerUsesDefaultLiveKit(true)
    setEditingServerLiveKitHost('')
    setEditingServerLiveKitApiKey('')
    setEditingServerLiveKitApiSecret('')
    await loadAdminServers()
    if (onServerListRefresh) {
      await onServerListRefresh()
    }
  }

  const handleDeleteServer = async (serverId: string) => {
    setDeletingServerId(serverId)
    setError('')
    const result = await adminServersDelete(serverId)
    setDeletingServerId(null)

    if (!result.success) {
      setError(result.error.message)
      return
    }

    onFeedback(t('admin.serverDeleted'))
    setPendingDeleteServerId(null)
    await loadAdminServers()
    if (onServerListRefresh) {
      await onServerListRefresh()
    }
  }

  const handleOpenConfig = (serverId: string) => {
    onOpenServerConfig?.(serverId)
    onClose()
  }

  if (!isOpen) return null

  return (
    <div className="panel admin-users-panel">
      <div className="admin-users-panel-header">
        <h3>{t('admin.title')}</h3>
        <div className="admin-panel-tabs">
          <button
            type="button"
            className={`admin-panel-tab${activeTab === 'users' ? ' active' : ''}`}
            onClick={() => setActiveTab('users')}
          >
            {t('appLayout.tabUsers')}
          </button>
          <button
            type="button"
            className={`admin-panel-tab${activeTab === 'invitations' ? ' active' : ''}`}
            onClick={() => setActiveTab('invitations')}
          >
            {t('admin.tabInvitations')}
          </button>
          <button
            type="button"
            className={`admin-panel-tab${activeTab === 'servers' ? ' active' : ''}`}
            onClick={() => setActiveTab('servers')}
          >
            {t('admin.tabServers')}
          </button>
          <button
            type="button"
            className={`admin-panel-tab${activeTab === 'plans' ? ' active' : ''}`}
            onClick={() => setActiveTab('plans')}
          >
            {t('admin.tabPlans')}
          </button>
          <button
            type="button"
            className={`admin-panel-tab${activeTab === 'planRequests' ? ' active' : ''}`}
            onClick={() => { setActiveTab('planRequests'); setPlanRequestsBadge(0) }}
            style={{ position: 'relative' }}
          >
            {t('admin.tabRequests')}
            {planRequestsBadge > 0 && (
              <span style={{
                position: 'absolute', top: -4, right: -4,
                background: '#ef4444', color: '#fff',
                borderRadius: '50%', width: 16, height: 16,
                fontSize: 10, display: 'flex', alignItems: 'center', justifyContent: 'center',
              }}>
                {planRequestsBadge}
              </span>
            )}
          </button>
        </div>
        <Button type="button" variant="ghost" size="sm" onClick={onClose}>✕</Button>
      </div>

      {error && <div className="modal-error" style={{ marginBottom: '8px' }}>{error}</div>}

      {activeTab === 'users' && (
        <div className="admin-users-grid">
          <section className="device-keys-section">
            <h4>{t('admin.createUser')}</h4>
            <form className="modal-inline-stack" onSubmit={handleCreateUser}>
              <div className="form-group">
                <label htmlFor="admin-create-username">{t('login.username')}</label>
                <input
                  id="admin-create-username"
                  type="text"
                  value={newUsername}
                  onChange={(e) => setNewUsername(e.target.value)}
                  placeholder={t('admin.newUserPlaceholder')}
                />
              </div>
              <div className="form-group">
                <label htmlFor="admin-create-password">{t('login.password')}</label>
                <input
                  id="admin-create-password"
                  type="password"
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  placeholder="********"
                />
              </div>
              <div className="form-group">
                <label htmlFor="admin-create-role">{t('admin.role')}</label>
                <select
                  id="admin-create-role"
                  value={newRole}
                  onChange={(e) => setNewRole(e.target.value === 'admin' ? 'admin' : 'user')}
                >
                  <option value="user">User</option>
                  <option value="admin">Admin</option>
                </select>
              </div>
              <div className="form-group">
                <label htmlFor="admin-create-plan">{t('admin.plan')}</label>
                <select
                  id="admin-create-plan"
                  value={newPlanId}
                  disabled={plans.length === 0}
                  onChange={(e) => setNewPlanId(e.target.value)}
                >
                  {plans.map((plan) => (
                    <option key={plan.id} value={plan.id}>{plan.displayName}</option>
                  ))}
                </select>
              </div>
              {plans.length === 0 && (
                <p className="admin-server-row-meta">{t('admin.noPlansHint')}</p>
              )}
              <div className="modal-actions-row">
                <Button type="submit" disabled={isCreatingUser || plans.length === 0}>
                  {isCreatingUser ? t('common.creating') : t('admin.createUser')}
                </Button>
              </div>
            </form>
          </section>

          <section className="device-keys-section">
            <h4>{t('appLayout.tabUsers')} {loadingUsers ? '...' : `(${users.length})`}</h4>
            {!loadingUsers && users.length === 0 && <p>{t('admin.noUsers')}</p>}
            {users.length > 0 && (
              <ul className="admin-compact-list">
                {users.map((user) => {
                  const tierInfo = tierByUserId[user.userId]
                  const isExpanded = expandedTierUserId === user.userId
                  const isLoadingTier = loadingTierUserId === user.userId

                  return (
                    <li key={user.userId} className="admin-compact-list-item admin-compact-list-item--col">
                      <div className="admin-user-row-main">
                        <span className="admin-compact-name">{user.username}</span>
                        <select
                          aria-label={`rol-${user.username}`}
                          value={user.role}
                          onChange={(e) => { void handleRoleChange(user.userId, e.target.value === 'admin' ? 'admin' : 'user') }}
                        >
                          <option value="user">User</option>
                          <option value="admin">Admin</option>
                        </select>
                        <select
                          aria-label={`pla-${user.username}`}
                          value={user.planId ?? newPlanId}
                          onChange={(e) => { void handlePlanChange(user.userId, e.target.value) }}
                        >
                          {plans.map((plan) => (
                            <option key={plan.id} value={plan.id}>{plan.displayName}</option>
                          ))}
                        </select>
                        <Button type="button" variant="danger" size="sm" onClick={() => { void handleDeleteUser(user.userId, user.username) }}>
                          {t('common.remove')}
                        </Button>
                        <Button type="button" variant="secondary" size="sm" onClick={() => { void handleToggleTiers(user.userId) }}>
                          {t('admin.tiers')}
                        </Button>
                      </div>

                      {isExpanded && (
                        <div className="admin-tier-panel">
                          {isLoadingTier && <span>{t('admin.loadingTiers')}</span>}
                          {!isLoadingTier && tierInfo && (
                            <>
                              <div className="admin-tier-grid">
                                <span>{t('admin.tierPlan')} <strong>{tierInfo.plan.displayName}</strong></span>
                                <span>{t('planSettings.servers')} <strong>{tierInfo.usage.totalServers}</strong> / {tierInfo.plan.limits.maxServers === -1 ? '∞' : tierInfo.plan.limits.maxServers}</span>
                                <span>{t('admin.tierText')} <strong>{tierInfo.usage.totalTextChannels}</strong> / {tierInfo.plan.limits.maxChannelsTextPerServer === -1 ? '∞' : tierInfo.plan.limits.maxChannelsTextPerServer}</span>
                                <span>{t('admin.tierVoice')} <strong>{tierInfo.usage.totalVoiceChannels}</strong> / {tierInfo.plan.limits.maxChannelsVoicePerServer === -1 ? '∞' : tierInfo.plan.limits.maxChannelsVoicePerServer}</span>
                                <span>{t('planSettings.members')} <strong>{tierInfo.usage.totalMembersAcrossServers}</strong> / {tierInfo.plan.limits.maxMembersPerServer === -1 ? '∞' : tierInfo.plan.limits.maxMembersPerServer}</span>
                                <span>{t('admin.tierMsgsToday')} <strong>{tierInfo.usage.messagesToday}</strong> / {tierInfo.plan.limits.messagesPerDay === -1 ? '∞' : tierInfo.plan.limits.messagesPerDay}</span>
                              </div>
                              <div className="admin-tier-grid admin-tier-grid--remaining">
                                <span>{t('admin.remainingServers')} <strong>{tierInfo.remaining.servers === -1 ? '∞' : tierInfo.remaining.servers}</strong></span>
                                <span>{t('admin.remainingText')} <strong>{tierInfo.remaining.textChannels === -1 ? '∞' : tierInfo.remaining.textChannels}</strong></span>
                                <span>{t('admin.remainingVoice')} <strong>{tierInfo.remaining.voiceChannels === -1 ? '∞' : tierInfo.remaining.voiceChannels}</strong></span>
                                <span>{t('admin.remainingMembers')} <strong>{tierInfo.remaining.members === -1 ? '∞' : tierInfo.remaining.members}</strong></span>
                                <span>{t('admin.remainingMsgsToday')} <strong>{tierInfo.remaining.messagesToday === -1 ? '∞' : tierInfo.remaining.messagesToday}</strong></span>
                              </div>
                            </>
                          )}
                        </div>
                      )}
                    </li>
                  )
                })}
              </ul>
            )}
          </section>
        </div>
      )}

      {activeTab === 'invitations' && (
        <div className="admin-users-grid">
          <section className="device-keys-section">
            <h4>{t('admin.createInvite')}</h4>
            <form className="modal-inline-stack" onSubmit={handleCreateInvitation}>
              <div className="form-group">
                <label htmlFor="admin-invite-max-uses">{t('admin.maxUses')}</label>
                <input
                  id="admin-invite-max-uses"
                  type="number"
                  min={1}
                  step={1}
                  value={inviteMaxUses}
                  onChange={(e) => setInviteMaxUses(Number(e.target.value || '1'))}
                />
              </div>
              <div className="form-group">
                <label htmlFor="admin-invite-server-target">{t('admin.targetServer')}</label>
                <select
                  id="admin-invite-server-target"
                  value={inviteServerId}
                  onChange={(e) => setInviteServerId(e.target.value)}
                >
                  <option value="">{t('admin.noneRegisterOnly')}</option>
                  {adminServers.map((server) => (
                    <option key={server.serverId} value={server.serverId}>{server.name}</option>
                  ))}
                </select>
              </div>
              <div className="modal-actions-row">
                <Button type="submit" disabled={isCreatingInvite}>
                  {isCreatingInvite ? t('common.creating') : t('admin.createInvite')}
                </Button>
              </div>
            </form>

            {lastCreatedCode && (
              <div className="admin-invite-code-box">
                <span className="admin-invite-code">{lastCreatedCode}</span>
                <Button type="button" variant="secondary" size="sm" onClick={() => { void handleCopyCode(lastCreatedCode) }}>
                  {t('admin.copy')}
                </Button>
              </div>
            )}
          </section>

          <section className="device-keys-section">
            <h4>{t('admin.tabInvitations')} {loadingInvitations ? '...' : `(${invitations.length})`}</h4>
            {!loadingInvitations && invitations.length === 0 && <p>{t('admin.noInvites')}</p>}
            {invitations.length > 0 && (
              <ul className="admin-compact-list">
                {invitations.map((inv) => (
                  <li key={inv.invitationId} className="admin-compact-list-item admin-compact-list-item--col">
                    <div className="admin-compact-invite-row">
                      <span className="admin-invite-code-inline">{inv.code}</span>
                      <Button type="button" variant="ghost" size="sm" onClick={() => { void handleCopyCode(inv.code) }}>
                        {t('admin.copy')}
                      </Button>
                    </div>
                    <div className="admin-compact-invite-meta">
                      <span>{t('admin.serverIdLabel')} <strong>{inv.serverId ?? t('admin.none')}</strong></span>
                      {inv.serverId && (
                        <span>
                          {t('admin.serverLabel')} <strong>{adminServers.find((server) => server.serverId === inv.serverId)?.name ?? t('admin.unknown')}</strong>
                        </span>
                      )}
                      <span>{t('admin.uses')} <strong>{inv.usesCount}</strong></span>
                      <span>{t('admin.remaining')} <strong>{inv.remainingUses === null ? '∞' : inv.remainingUses}</strong></span>
                      <span>{t('admin.max')} <strong>{inv.maxUses < 0 ? '∞' : inv.maxUses}</strong></span>
                      <span className={inv.isActive ? 'admin-badge-active' : 'admin-badge-inactive'}>
                        {inv.isActive ? t('admin.active') : t('admin.inactive')}
                      </span>
                    </div>
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      )}

      {activeTab === 'servers' && (
        <div className="admin-users-grid">
          <section className="device-keys-section">
            <h4>{t('admin.createServerHeading')}</h4>
            <form className="modal-inline-stack" onSubmit={handleCreateServer}>
              <div className="form-group">
                <label htmlFor="admin-server-create-name">{t('admin.name')}</label>
                <input
                  id="admin-server-create-name"
                  type="text"
                  value={newServerName}
                  onChange={(e) => setNewServerName(e.target.value)}
                  placeholder={t('appLayout.tabNewServer')}
                />
              </div>
              <div className="form-group">
                <label htmlFor="admin-server-create-icon">{t('admin.iconUrlOptional')}</label>
                <input
                  id="admin-server-create-icon"
                  type="text"
                  value={newServerIconUrl}
                  onChange={(e) => setNewServerIconUrl(e.target.value)}
                  placeholder="https://..."
                />
              </div>
              <div className="modal-actions-row">
                <Button type="submit" disabled={creatingServer}>
                  {creatingServer ? t('common.creating') : t('appLayout.panelCreateServer')}
                </Button>
              </div>
            </form>
          </section>

          <section className="device-keys-section">
            <h4>{t('admin.tabServers')} {loadingServers ? '...' : `(${adminServers.length})`}</h4>
            {!loadingServers && adminServers.length === 0 && <p>{t('admin.noServers')}</p>}
            {adminServers.length > 0 && (
              <ul className="admin-compact-list">
                {adminServers.map((server) => {
                  const isEditing = editingServerId === server.serverId
                  const isSaving = savingServerId === server.serverId
                  const isDeleting = deletingServerId === server.serverId

                  return (
                    <li key={server.serverId} className="admin-compact-list-item admin-compact-list-item--col">
                      <div className="admin-server-row-main">
                        <div className="admin-server-row-title">
                          <span className="admin-compact-name">{server.name}</span>
                          <span className="admin-server-row-meta">{server.serverId}</span>
                          <span className="admin-server-row-meta">
                            {t('admin.livekitLabel')} {server.livekitConfig?.isOverride ? server.livekitConfig.host : t('admin.livekitDefault')}
                          </span>
                        </div>

                        <div className="admin-server-row-actions">
                          <Button type="button" variant="secondary" size="sm" onClick={() => handleOpenConfig(server.serverId)}>
                            {t('admin.configuration')}
                          </Button>
                          {isEditing ? (
                            <>
                              <Button type="button" variant="primary" size="sm" disabled={isSaving} onClick={() => { void handleSaveServer(server.serverId) }}>
                                {isSaving ? t('common.saving') : t('common.saveAction')}
                              </Button>
                              <Button type="button" variant="ghost" size="sm" onClick={() => setEditingServerId(null)}>
                                {t('common.cancel')}
                              </Button>
                            </>
                          ) : (
                            <Button type="button" variant="ghost" size="sm" onClick={() => handleStartEditServer(server)}>
                              {t('common.modify')}
                            </Button>
                          )}
                          <Button
                            type="button"
                            variant="danger"
                            size="sm"
                            disabled={isDeleting}
                            onClick={() => setPendingDeleteServerId(server.serverId)}
                          >
                            {isDeleting ? t('common.erasing') : t('common.erase')}
                          </Button>
                        </div>
                      </div>

                      {isEditing && (
                        <div className="admin-server-edit-grid">
                          <div className="form-group">
                            <label htmlFor={`admin-server-name-${server.serverId}`}>{t('createServer.nameLabel')}</label>
                            <input
                              id={`admin-server-name-${server.serverId}`}
                              type="text"
                              value={editingServerName}
                              onChange={(e) => setEditingServerName(e.target.value)}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-server-icon-${server.serverId}`}>{t('admin.iconUrlOptional')}</label>
                            <input
                              id={`admin-server-icon-${server.serverId}`}
                              type="text"
                              value={editingServerIconUrl}
                              onChange={(e) => setEditingServerIconUrl(e.target.value)}
                              placeholder={t('admin.leaveEmptyToRemove')}
                            />
                          </div>
                          <div className="form-group" style={{ gridColumn: '1 / -1' }}>
                            <label style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
                              <input
                                type="checkbox"
                                checked={editingServerUsesDefaultLiveKit}
                                onChange={(e) => setEditingServerUsesDefaultLiveKit(e.target.checked)}
                              />
                              {t('admin.useDefaultLiveKit')}
                            </label>
                          </div>
                          {!editingServerUsesDefaultLiveKit && (
                            <>
                              <div className="form-group">
                                <label htmlFor={`admin-server-livekit-host-${server.serverId}`}>{t('admin.livekitHost')}</label>
                                <input
                                  id={`admin-server-livekit-host-${server.serverId}`}
                                  type="text"
                                  value={editingServerLiveKitHost}
                                  onChange={(e) => setEditingServerLiveKitHost(e.target.value)}
                                  placeholder={t('admin.livekitHostPlaceholder')}
                                />
                              </div>
                              <div className="form-group">
                                <label htmlFor={`admin-server-livekit-key-${server.serverId}`}>{t('admin.livekitApiKey')}</label>
                                <input
                                  id={`admin-server-livekit-key-${server.serverId}`}
                                  type="text"
                                  value={editingServerLiveKitApiKey}
                                  onChange={(e) => setEditingServerLiveKitApiKey(e.target.value)}
                                  placeholder={t('admin.apiKeyPlaceholder')}
                                />
                              </div>
                              <div className="form-group" style={{ gridColumn: '1 / -1' }}>
                                <label htmlFor={`admin-server-livekit-secret-${server.serverId}`}>{t('admin.livekitApiSecret')}</label>
                                <input
                                  id={`admin-server-livekit-secret-${server.serverId}`}
                                  type="password"
                                  value={editingServerLiveKitApiSecret}
                                  onChange={(e) => setEditingServerLiveKitApiSecret(e.target.value)}
                                  placeholder={server.livekitConfig?.isOverride ? t('admin.secretKeepPlaceholder') : t('admin.apiSecretPlaceholder')}
                                />
                              </div>
                            </>
                          )}
                        </div>
                      )}

                      {pendingDeleteServerId === server.serverId && (
                        <div className="admin-server-delete-confirm">
                          <span>{t('admin.confirmDeleteServer')}</span>
                          <div className="admin-server-delete-actions">
                            <Button
                              type="button"
                              variant="danger"
                              size="sm"
                              disabled={isDeleting}
                              onClick={() => { void handleDeleteServer(server.serverId) }}
                            >
                              {t('common.confirmAction')}
                            </Button>
                            <Button type="button" variant="ghost" size="sm" onClick={() => setPendingDeleteServerId(null)}>
                              {t('common.cancel')}
                            </Button>
                          </div>
                        </div>
                      )}
                    </li>
                  )
                })}
              </ul>
            )}
          </section>
        </div>
      )}

      {activeTab === 'plans' && (
        <div className="admin-users-grid">
          <section className="device-keys-section">
            <h4>{t('admin.createPlan')}</h4>
            <form className="modal-inline-stack" onSubmit={handleCreatePlan}>
              <div className="form-group">
                <label htmlFor="admin-plan-create-name">{t('admin.internalName')}</label>
                <input
                  id="admin-plan-create-name"
                  type="text"
                  value={newPlan.name}
                  onChange={(e) => handlePlanFieldChange('name', e.target.value)}
                  placeholder="team_plus"
                />
              </div>
              <div className="form-group">
                <label htmlFor="admin-plan-create-display-name">{t('admin.displayName')}</label>
                <input
                  id="admin-plan-create-display-name"
                  type="text"
                  value={newPlan.displayName}
                  onChange={(e) => handlePlanFieldChange('displayName', e.target.value)}
                  placeholder="Team Plus"
                />
              </div>
              <div className="form-group">
                <label htmlFor="admin-plan-create-description">{t('admin.descriptionOptional')}</label>
                <input
                  id="admin-plan-create-description"
                  type="text"
                  value={newPlan.description ?? ''}
                  onChange={(e) => handlePlanFieldChange('description', e.target.value)}
                  placeholder={t('admin.planExampleDesc')}
                />
              </div>
              <div className="admin-plan-limits-grid">
                <div className="form-group">
                  <label htmlFor="admin-plan-create-max-servers">{t('admin.maxServers')}</label>
                  <input
                    id="admin-plan-create-max-servers"
                    type="number"
                    min={-1}
                    value={newPlan.maxServers}
                    onChange={(e) => handlePlanFieldChange('maxServers', Number(e.target.value || '0'))}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="admin-plan-create-max-text">{t('admin.maxTextChannels')}</label>
                  <input
                    id="admin-plan-create-max-text"
                    type="number"
                    min={-1}
                    value={newPlan.maxChannelsTextPerServer}
                    onChange={(e) => handlePlanFieldChange('maxChannelsTextPerServer', Number(e.target.value || '0'))}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="admin-plan-create-max-voice">{t('admin.maxVoiceChannels')}</label>
                  <input
                    id="admin-plan-create-max-voice"
                    type="number"
                    min={-1}
                    value={newPlan.maxChannelsVoicePerServer}
                    onChange={(e) => handlePlanFieldChange('maxChannelsVoicePerServer', Number(e.target.value || '0'))}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="admin-plan-create-max-members">{t('admin.maxMembers')}</label>
                  <input
                    id="admin-plan-create-max-members"
                    type="number"
                    min={-1}
                    value={newPlan.maxMembersPerServer}
                    onChange={(e) => handlePlanFieldChange('maxMembersPerServer', Number(e.target.value || '0'))}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="admin-plan-create-max-api">{t('admin.apiPerMin')}</label>
                  <input
                    id="admin-plan-create-max-api"
                    type="number"
                    min={-1}
                    value={newPlan.apiCallsPerMinute}
                    onChange={(e) => handlePlanFieldChange('apiCallsPerMinute', Number(e.target.value || '0'))}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="admin-plan-create-max-messages">{t('admin.msgsPerDay')}</label>
                  <input
                    id="admin-plan-create-max-messages"
                    type="number"
                    min={-1}
                    value={newPlan.messagesPerDay}
                    onChange={(e) => handlePlanFieldChange('messagesPerDay', Number(e.target.value || '0'))}
                  />
                </div>
              </div>
              <div className="modal-actions-row">
                <Button type="submit" disabled={creatingPlan}>
                  {creatingPlan ? t('common.creating') : t('admin.createPlan')}
                </Button>
              </div>
            </form>
          </section>

          <section className="device-keys-section">
            <h4>{t('admin.tabPlans')} {loadingPlans ? '...' : `(${plans.length})`}</h4>
            {!loadingPlans && plans.length === 0 && <p>{t('admin.noPlans')}</p>}
            {plans.length > 0 && (
              <ul className="admin-compact-list">
                {plans.map((plan) => {
                  const isEditing = editingPlanId === plan.id
                  const isSaving = savingPlanId === plan.id
                  const isDeleting = deletingPlanId === plan.id
                  const isSystem = plan.isSystem

                  return (
                    <li key={plan.id} className="admin-compact-list-item admin-compact-list-item--col">
                      <div className="admin-server-row-main">
                        <div className="admin-server-row-title">
                          <span className="admin-compact-name">{plan.displayName} ({plan.name})</span>
                          <span className="admin-server-row-meta">{plan.id}</span>
                        </div>

                        <div className="admin-server-row-actions">
                          {isSystem && <span className="admin-badge-active">{t('admin.system')}</span>}
                          {isEditing ? (
                            <>
                              <Button type="button" variant="primary" size="sm" disabled={isSaving} onClick={() => { void handleUpdatePlan(plan.id) }}>
                                {isSaving ? t('common.saving') : t('common.saveAction')}
                              </Button>
                              <Button type="button" variant="ghost" size="sm" onClick={() => setEditingPlanId(null)}>
                                {t('common.cancel')}
                              </Button>
                            </>
                          ) : (
                            <Button type="button" variant="ghost" size="sm" disabled={isSystem} onClick={() => startEditPlan(plan)}>
                              {t('common.modify')}
                            </Button>
                          )}
                          <Button
                            type="button"
                            variant="danger"
                            size="sm"
                            disabled={isDeleting || isSystem}
                            onClick={() => setPendingDeletePlanId(plan.id)}
                          >
                            {isDeleting ? t('common.erasing') : t('common.erase')}
                          </Button>
                        </div>
                      </div>

                      {isEditing ? (
                        <div className="admin-plan-limits-grid">
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-name-${plan.id}`}>{t('admin.internalName')}</label>
                            <input
                              id={`admin-plan-edit-name-${plan.id}`}
                              type="text"
                              value={editingPlan.name}
                              onChange={(e) => handleEditPlanFieldChange('name', e.target.value)}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-display-${plan.id}`}>{t('admin.displayName')}</label>
                            <input
                              id={`admin-plan-edit-display-${plan.id}`}
                              type="text"
                              value={editingPlan.displayName}
                              onChange={(e) => handleEditPlanFieldChange('displayName', e.target.value)}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-description-${plan.id}`}>{t('admin.description')}</label>
                            <input
                              id={`admin-plan-edit-description-${plan.id}`}
                              type="text"
                              value={editingPlan.description ?? ''}
                              onChange={(e) => handleEditPlanFieldChange('description', e.target.value)}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-max-servers-${plan.id}`}>{t('admin.maxServers')}</label>
                            <input
                              id={`admin-plan-edit-max-servers-${plan.id}`}
                              type="number"
                              min={-1}
                              value={editingPlan.maxServers}
                              onChange={(e) => handleEditPlanFieldChange('maxServers', Number(e.target.value || '0'))}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-max-text-${plan.id}`}>{t('admin.maxTextChannels')}</label>
                            <input
                              id={`admin-plan-edit-max-text-${plan.id}`}
                              type="number"
                              min={-1}
                              value={editingPlan.maxChannelsTextPerServer}
                              onChange={(e) => handleEditPlanFieldChange('maxChannelsTextPerServer', Number(e.target.value || '0'))}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-max-voice-${plan.id}`}>{t('admin.maxVoiceChannels')}</label>
                            <input
                              id={`admin-plan-edit-max-voice-${plan.id}`}
                              type="number"
                              min={-1}
                              value={editingPlan.maxChannelsVoicePerServer}
                              onChange={(e) => handleEditPlanFieldChange('maxChannelsVoicePerServer', Number(e.target.value || '0'))}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-max-members-${plan.id}`}>{t('admin.maxMembers')}</label>
                            <input
                              id={`admin-plan-edit-max-members-${plan.id}`}
                              type="number"
                              min={-1}
                              value={editingPlan.maxMembersPerServer}
                              onChange={(e) => handleEditPlanFieldChange('maxMembersPerServer', Number(e.target.value || '0'))}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-max-api-${plan.id}`}>{t('admin.apiPerMin')}</label>
                            <input
                              id={`admin-plan-edit-max-api-${plan.id}`}
                              type="number"
                              min={-1}
                              value={editingPlan.apiCallsPerMinute}
                              onChange={(e) => handleEditPlanFieldChange('apiCallsPerMinute', Number(e.target.value || '0'))}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-max-messages-${plan.id}`}>{t('admin.msgsPerDay')}</label>
                            <input
                              id={`admin-plan-edit-max-messages-${plan.id}`}
                              type="number"
                              min={-1}
                              value={editingPlan.messagesPerDay}
                              onChange={(e) => handleEditPlanFieldChange('messagesPerDay', Number(e.target.value || '0'))}
                            />
                          </div>
                        </div>
                      ) : (
                        <div className="admin-tier-grid">
                          <span>{t('planSettings.servers')} <strong>{plan.maxServers === -1 ? '∞' : plan.maxServers}</strong></span>
                          <span>{t('admin.tierText')} <strong>{plan.maxChannelsTextPerServer === -1 ? '∞' : plan.maxChannelsTextPerServer}</strong></span>
                          <span>{t('admin.tierVoice')} <strong>{plan.maxChannelsVoicePerServer === -1 ? '∞' : plan.maxChannelsVoicePerServer}</strong></span>
                          <span>{t('planSettings.members')} <strong>{plan.maxMembersPerServer === -1 ? '∞' : plan.maxMembersPerServer}</strong></span>
                          <span>{t('admin.apiMin')} <strong>{plan.apiCallsPerMinute === -1 ? '∞' : plan.apiCallsPerMinute}</strong></span>
                          <span>{t('admin.msgsDay')} <strong>{plan.messagesPerDay === -1 ? '∞' : plan.messagesPerDay}</strong></span>
                        </div>
                      )}

                      {pendingDeletePlanId === plan.id && (
                        <div className="admin-server-delete-confirm">
                          <span>{t('admin.confirmDeletePlan')}</span>
                          <div className="admin-server-delete-actions">
                            <Button
                              type="button"
                              variant="danger"
                              size="sm"
                              disabled={isDeleting}
                              onClick={() => { void handleDeletePlan(plan.id) }}
                            >
                              {t('common.confirmAction')}
                            </Button>
                            <Button type="button" variant="ghost" size="sm" onClick={() => setPendingDeletePlanId(null)}>
                              {t('common.cancel')}
                            </Button>
                          </div>
                        </div>
                      )}
                    </li>
                  )
                })}
              </ul>
            )}
          </section>
        </div>
      )}

      {activeTab === 'planRequests' && (
        <div className="admin-users-grid">
          <section className="device-keys-section">
            <h4>{t('admin.requestsHeading')}</h4>
            {loadingPlanRequests ? (
              <p style={{ color: 'var(--text-secondary)', fontSize: 13 }}>{t('common.loadingShort')}</p>
            ) : planRequests.length === 0 ? (
              <p style={{ color: 'var(--text-secondary)', fontSize: 13 }}>{t('admin.noRequests')}</p>
            ) : (
              <ul style={{ listStyle: 'none', padding: 0, margin: 0, display: 'flex', flexDirection: 'column', gap: 12 }}>
                {planRequests.map((req) => (
                  <li
                    key={req.id}
                    style={{
                      border: '1px solid var(--bg-active)',
                      borderRadius: 6,
                      padding: '12px 14px',
                      background: req.status === 'pending' ? 'var(--bg-secondary)' : 'transparent',
                      opacity: req.status !== 'pending' ? 0.6 : 1,
                    }}
                  >
                    <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: 8 }}>
                      <div>
                        <span style={{ fontWeight: 500 }}>{req.username}</span>
                        <span style={{ color: 'var(--text-secondary)', fontSize: 12, marginLeft: 8 }}>→ {req.requestedPlanName}</span>
                        <span style={{
                          marginLeft: 8, fontSize: 11, padding: '1px 6px', borderRadius: 3,
                          background: req.status === 'pending' ? '#f59e0b33' : req.status === 'approved' ? '#22c55e33' : '#ef444433',
                          color: req.status === 'pending' ? '#f59e0b' : req.status === 'approved' ? '#22c55e' : '#ef4444',
                        }}>
                          {req.status === 'pending' ? t('admin.statusPending') : req.status === 'approved' ? t('admin.statusApproved') : t('admin.statusRejected')}
                        </span>
                      </div>
                      <span style={{ fontSize: 11, color: 'var(--text-secondary)', whiteSpace: 'nowrap' }}>
                        {new Date(req.createdAt).toLocaleDateString('ca')}
                      </span>
                    </div>
                    {req.message && (
                      <p style={{ margin: '6px 0 0', fontSize: 13, color: 'var(--text-secondary)' }}>{req.message}</p>
                    )}
                    {req.adminNote && (
                      <p style={{ margin: '4px 0 0', fontSize: 12, color: 'var(--text-secondary)', fontStyle: 'italic' }}>{t('admin.noteLabel')} {req.adminNote}</p>
                    )}
                    {req.status === 'pending' && (
                      <div style={{ marginTop: 10, display: 'flex', flexDirection: 'column', gap: 6 }}>
                        <input
                          type="text"
                          placeholder={t('admin.notePlaceholder')}
                          value={requestAdminNote[req.id] ?? ''}
                          onChange={(e) => setRequestAdminNote((prev) => ({ ...prev, [req.id]: e.target.value }))}
                          style={{ fontSize: 12, padding: '4px 8px' }}
                          disabled={resolvingRequestId === req.id}
                        />
                        <div style={{ display: 'flex', gap: 8 }}>
                          <Button
                            type="button"
                            variant="primary"
                            size="sm"
                            disabled={resolvingRequestId === req.id}
                            onClick={async () => {
                              setResolvingRequestId(req.id)
                              const result = await adminPlanChangeRequestApprove(req.id, requestAdminNote[req.id] || undefined)
                              setResolvingRequestId(null)
                              if (!result.success) { setError(result.error.message); return }
                              onFeedback(t('admin.planChangedTo', { username: req.username, plan: req.requestedPlanName }))
                              void loadPlanRequests()
                            }}
                          >
                            {t('admin.approve')}
                          </Button>
                          <Button
                            type="button"
                            variant="ghost"
                            size="sm"
                            disabled={resolvingRequestId === req.id}
                            onClick={async () => {
                              setResolvingRequestId(req.id)
                              const result = await adminPlanChangeRequestReject(req.id, requestAdminNote[req.id] || undefined)
                              setResolvingRequestId(null)
                              if (!result.success) { setError(result.error.message); return }
                              onFeedback(t('admin.requestRejected', { username: req.username }))
                              void loadPlanRequests()
                            }}
                          >
                            {t('admin.reject')}
                          </Button>
                        </div>
                      </div>
                    )}
                  </li>
                ))}
              </ul>
            )}
          </section>
        </div>
      )}
    </div>
  )
}
