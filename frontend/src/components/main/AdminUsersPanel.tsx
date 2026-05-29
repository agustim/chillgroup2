import React, { useEffect, useState } from 'react'

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
  type AdminPlanInput,
  type AdminPlanItem,
  type AdminServerItem,
  type AdminUserLimitsInfo,
  type AdminUserItem,
  type AdminUserRole,
  type InvitationListItem,
} from '../../lib/api'
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

type ActiveTab = 'users' | 'invitations' | 'servers' | 'plans'

export function AdminUsersPanel({
  isOpen,
  onClose,
  onFeedback,
  selectedServerId,
  availableServers = [],
  onOpenServerConfig,
  onServerListRefresh,
}: AdminUsersPanelProps) {
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
  const [savingServerId, setSavingServerId] = useState<string | null>(null)
  const [pendingDeleteServerId, setPendingDeleteServerId] = useState<string | null>(null)
  const [deletingServerId, setDeletingServerId] = useState<string | null>(null)

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
  }, [isOpen, selectedServerId])

  const handleCreateUser = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const username = newUsername.trim()
    if (!username || !newPassword.trim()) { setError('Usuari i contrasenya son obligatoris'); return }
    setIsCreatingUser(true)
    setError('')
    const result = await adminUsersCreate(username, newPassword, newRole, newPlanId)
    setIsCreatingUser(false)
    if (!result.success) { setError(result.error.message); return }
    onFeedback(`Usuari ${result.data.username} creat`)
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
      setError('Nom intern i display name del pla són obligatoris')
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

    onFeedback(`Pla ${result.data.displayName} creat`)
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
      setError('Nom intern i display name del pla són obligatoris')
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

    onFeedback('Pla actualitzat')
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

    onFeedback('Pla eliminat')
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
    onFeedback('Rol actualitzat')
  }

  const handlePlanChange = async (userId: string, planId: string) => {
    const result = await adminUsersUpdatePlan(userId, planId)
    if (!result.success) { setError(result.error.message); return }
    setUsers((c) => c.map((u) => (u.userId === userId ? { ...u, planId } : u)))
    onFeedback('Pla actualitzat')
  }

  const handleDeleteUser = async (userId: string, username: string) => {
    const result = await adminUsersDelete(userId)
    if (!result.success) { setError(result.error.message); return }
    setUsers((c) => c.filter((u) => u.userId !== userId))
    onFeedback(`Usuari ${username} eliminat`)
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
    onFeedback('Invitacio creada')
    await loadInvitations()
  }

  const handleCopyCode = async (code: string) => {
    try {
      await navigator.clipboard.writeText(code)
      onFeedback('Codi copiat al porta-retalls')
    } catch {
      onFeedback('No s\'ha pogut copiar el codi')
    }
  }

  const handleCreateServer = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const name = newServerName.trim()
    if (!name) {
      setError('El nom del servidor és obligatori')
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

    onFeedback(`Servidor ${result.data.name} creat`)
    setNewServerName('')
    setNewServerIconUrl('')
    await loadAdminServers()
    if (onServerListRefresh) {
      await onServerListRefresh()
    }
  }

  const handleStartEditServer = (server: AdminServerOption) => {
    setEditingServerId(server.serverId)
    setEditingServerName(server.name)
    setEditingServerIconUrl('')
    setPendingDeleteServerId(null)
  }

  const handleSaveServer = async (serverId: string) => {
    const name = editingServerName.trim()
    if (!name) {
      setError('El nom del servidor és obligatori')
      return
    }

    setSavingServerId(serverId)
    setError('')
    const iconUrl = editingServerIconUrl.trim() || null
    const result = await adminServersUpdate(serverId, name, iconUrl)
    setSavingServerId(null)

    if (!result.success) {
      setError(result.error.message)
      return
    }

    onFeedback('Servidor actualitzat')
    setEditingServerId(null)
    setEditingServerName('')
    setEditingServerIconUrl('')
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

    onFeedback('Servidor eliminat')
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
        <h3>Gestio (admin)</h3>
        <div className="admin-panel-tabs">
          <button
            type="button"
            className={`admin-panel-tab${activeTab === 'users' ? ' active' : ''}`}
            onClick={() => setActiveTab('users')}
          >
            Usuaris
          </button>
          <button
            type="button"
            className={`admin-panel-tab${activeTab === 'invitations' ? ' active' : ''}`}
            onClick={() => setActiveTab('invitations')}
          >
            Invitacions
          </button>
          <button
            type="button"
            className={`admin-panel-tab${activeTab === 'servers' ? ' active' : ''}`}
            onClick={() => setActiveTab('servers')}
          >
            Servidors
          </button>
          <button
            type="button"
            className={`admin-panel-tab${activeTab === 'plans' ? ' active' : ''}`}
            onClick={() => setActiveTab('plans')}
          >
            Plans
          </button>
        </div>
        <Button type="button" variant="ghost" size="sm" onClick={onClose}>✕</Button>
      </div>

      {error && <div className="modal-error" style={{ marginBottom: '8px' }}>{error}</div>}

      {activeTab === 'users' && (
        <div className="admin-users-grid">
          <section className="device-keys-section">
            <h4>Crear usuari</h4>
            <form className="modal-inline-stack" onSubmit={handleCreateUser}>
              <div className="form-group">
                <label htmlFor="admin-create-username">Nom d'usuari</label>
                <input
                  id="admin-create-username"
                  type="text"
                  value={newUsername}
                  onChange={(e) => setNewUsername(e.target.value)}
                  placeholder="nou-usuari"
                />
              </div>
              <div className="form-group">
                <label htmlFor="admin-create-password">Contrasenya</label>
                <input
                  id="admin-create-password"
                  type="password"
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  placeholder="********"
                />
              </div>
              <div className="form-group">
                <label htmlFor="admin-create-role">Rol</label>
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
                <label htmlFor="admin-create-plan">Pla</label>
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
                <p className="admin-server-row-meta">No hi ha plans disponibles. Crea'n un a la pestanya Plans.</p>
              )}
              <div className="modal-actions-row">
                <Button type="submit" disabled={isCreatingUser || plans.length === 0}>
                  {isCreatingUser ? 'Creant...' : 'Crear usuari'}
                </Button>
              </div>
            </form>
          </section>

          <section className="device-keys-section">
            <h4>Usuaris {loadingUsers ? '...' : `(${users.length})`}</h4>
            {!loadingUsers && users.length === 0 && <p>No hi ha usuaris.</p>}
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
                          Eliminar
                        </Button>
                        <Button type="button" variant="secondary" size="sm" onClick={() => { void handleToggleTiers(user.userId) }}>
                          Tiers
                        </Button>
                      </div>

                      {isExpanded && (
                        <div className="admin-tier-panel">
                          {isLoadingTier && <span>Carregant tiers...</span>}
                          {!isLoadingTier && tierInfo && (
                            <>
                              <div className="admin-tier-grid">
                                <span>Pla: <strong>{tierInfo.plan.displayName}</strong></span>
                                <span>Servidors: <strong>{tierInfo.usage.totalServers}</strong> / {tierInfo.plan.limits.maxServers === -1 ? '∞' : tierInfo.plan.limits.maxServers}</span>
                                <span>Text: <strong>{tierInfo.usage.totalTextChannels}</strong> / {tierInfo.plan.limits.maxChannelsTextPerServer === -1 ? '∞' : tierInfo.plan.limits.maxChannelsTextPerServer}</span>
                                <span>Veu: <strong>{tierInfo.usage.totalVoiceChannels}</strong> / {tierInfo.plan.limits.maxChannelsVoicePerServer === -1 ? '∞' : tierInfo.plan.limits.maxChannelsVoicePerServer}</span>
                                <span>Membres: <strong>{tierInfo.usage.totalMembersAcrossServers}</strong> / {tierInfo.plan.limits.maxMembersPerServer === -1 ? '∞' : tierInfo.plan.limits.maxMembersPerServer}</span>
                                <span>Msgs avui: <strong>{tierInfo.usage.messagesToday}</strong> / {tierInfo.plan.limits.messagesPerDay === -1 ? '∞' : tierInfo.plan.limits.messagesPerDay}</span>
                              </div>
                              <div className="admin-tier-grid admin-tier-grid--remaining">
                                <span>Restant servidors: <strong>{tierInfo.remaining.servers === -1 ? '∞' : tierInfo.remaining.servers}</strong></span>
                                <span>Restant text: <strong>{tierInfo.remaining.textChannels === -1 ? '∞' : tierInfo.remaining.textChannels}</strong></span>
                                <span>Restant veu: <strong>{tierInfo.remaining.voiceChannels === -1 ? '∞' : tierInfo.remaining.voiceChannels}</strong></span>
                                <span>Restant membres: <strong>{tierInfo.remaining.members === -1 ? '∞' : tierInfo.remaining.members}</strong></span>
                                <span>Restant msgs avui: <strong>{tierInfo.remaining.messagesToday === -1 ? '∞' : tierInfo.remaining.messagesToday}</strong></span>
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
            <h4>Crear invitacio</h4>
            <form className="modal-inline-stack" onSubmit={handleCreateInvitation}>
              <div className="form-group">
                <label htmlFor="admin-invite-max-uses">Usos maxims</label>
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
                <label htmlFor="admin-invite-server-target">Servidor objectiu</label>
                <select
                  id="admin-invite-server-target"
                  value={inviteServerId}
                  onChange={(e) => setInviteServerId(e.target.value)}
                >
                  <option value="">Cap (nomes registre)</option>
                  {adminServers.map((server) => (
                    <option key={server.serverId} value={server.serverId}>{server.name}</option>
                  ))}
                </select>
              </div>
              <div className="modal-actions-row">
                <Button type="submit" disabled={isCreatingInvite}>
                  {isCreatingInvite ? 'Creant...' : 'Crear invitacio'}
                </Button>
              </div>
            </form>

            {lastCreatedCode && (
              <div className="admin-invite-code-box">
                <span className="admin-invite-code">{lastCreatedCode}</span>
                <Button type="button" variant="secondary" size="sm" onClick={() => { void handleCopyCode(lastCreatedCode) }}>
                  Copiar
                </Button>
              </div>
            )}
          </section>

          <section className="device-keys-section">
            <h4>Invitacions {loadingInvitations ? '...' : `(${invitations.length})`}</h4>
            {!loadingInvitations && invitations.length === 0 && <p>No hi ha invitacions creades.</p>}
            {invitations.length > 0 && (
              <ul className="admin-compact-list">
                {invitations.map((inv) => (
                  <li key={inv.invitationId} className="admin-compact-list-item admin-compact-list-item--col">
                    <div className="admin-compact-invite-row">
                      <span className="admin-invite-code-inline">{inv.code}</span>
                      <Button type="button" variant="ghost" size="sm" onClick={() => { void handleCopyCode(inv.code) }}>
                        Copiar
                      </Button>
                    </div>
                    <div className="admin-compact-invite-meta">
                      <span>Server ID: <strong>{inv.serverId ?? 'Cap'}</strong></span>
                      {inv.serverId && (
                        <span>
                          Server: <strong>{adminServers.find((server) => server.serverId === inv.serverId)?.name ?? 'Desconegut'}</strong>
                        </span>
                      )}
                      <span>Fets: <strong>{inv.usesCount}</strong></span>
                      <span>Restants: <strong>{inv.remainingUses === null ? '∞' : inv.remainingUses}</strong></span>
                      <span>Max: <strong>{inv.maxUses < 0 ? '∞' : inv.maxUses}</strong></span>
                      <span className={inv.isActive ? 'admin-badge-active' : 'admin-badge-inactive'}>
                        {inv.isActive ? 'Activa' : 'Inactiva'}
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
            <h4>Alta de servidor</h4>
            <form className="modal-inline-stack" onSubmit={handleCreateServer}>
              <div className="form-group">
                <label htmlFor="admin-server-create-name">Nom</label>
                <input
                  id="admin-server-create-name"
                  type="text"
                  value={newServerName}
                  onChange={(e) => setNewServerName(e.target.value)}
                  placeholder="Nou servidor"
                />
              </div>
              <div className="form-group">
                <label htmlFor="admin-server-create-icon">Icon URL (opcional)</label>
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
                  {creatingServer ? 'Creant...' : 'Crear servidor'}
                </Button>
              </div>
            </form>
          </section>

          <section className="device-keys-section">
            <h4>Servidors {loadingServers ? '...' : `(${adminServers.length})`}</h4>
            {!loadingServers && adminServers.length === 0 && <p>No hi ha servidors disponibles.</p>}
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
                        </div>

                        <div className="admin-server-row-actions">
                          <Button type="button" variant="secondary" size="sm" onClick={() => handleOpenConfig(server.serverId)}>
                            Configuració
                          </Button>
                          {isEditing ? (
                            <>
                              <Button type="button" variant="primary" size="sm" disabled={isSaving} onClick={() => { void handleSaveServer(server.serverId) }}>
                                {isSaving ? 'Desant...' : 'Desar'}
                              </Button>
                              <Button type="button" variant="ghost" size="sm" onClick={() => setEditingServerId(null)}>
                                Cancel·lar
                              </Button>
                            </>
                          ) : (
                            <Button type="button" variant="ghost" size="sm" onClick={() => handleStartEditServer(server)}>
                              Modificar
                            </Button>
                          )}
                          <Button
                            type="button"
                            variant="danger"
                            size="sm"
                            disabled={isDeleting}
                            onClick={() => setPendingDeleteServerId(server.serverId)}
                          >
                            {isDeleting ? 'Esborrant...' : 'Esborrar'}
                          </Button>
                        </div>
                      </div>

                      {isEditing && (
                        <div className="admin-server-edit-grid">
                          <div className="form-group">
                            <label htmlFor={`admin-server-name-${server.serverId}`}>Nom del servidor</label>
                            <input
                              id={`admin-server-name-${server.serverId}`}
                              type="text"
                              value={editingServerName}
                              onChange={(e) => setEditingServerName(e.target.value)}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-server-icon-${server.serverId}`}>Icon URL (opcional)</label>
                            <input
                              id={`admin-server-icon-${server.serverId}`}
                              type="text"
                              value={editingServerIconUrl}
                              onChange={(e) => setEditingServerIconUrl(e.target.value)}
                              placeholder="Deixa buit per eliminar"
                            />
                          </div>
                        </div>
                      )}

                      {pendingDeleteServerId === server.serverId && (
                        <div className="admin-server-delete-confirm">
                          <span>Segur que vols eliminar aquest servidor?</span>
                          <div className="admin-server-delete-actions">
                            <Button
                              type="button"
                              variant="danger"
                              size="sm"
                              disabled={isDeleting}
                              onClick={() => { void handleDeleteServer(server.serverId) }}
                            >
                              Confirmar
                            </Button>
                            <Button type="button" variant="ghost" size="sm" onClick={() => setPendingDeleteServerId(null)}>
                              Cancel·lar
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
            <h4>Crear pla</h4>
            <form className="modal-inline-stack" onSubmit={handleCreatePlan}>
              <div className="form-group">
                <label htmlFor="admin-plan-create-name">Nom intern</label>
                <input
                  id="admin-plan-create-name"
                  type="text"
                  value={newPlan.name}
                  onChange={(e) => handlePlanFieldChange('name', e.target.value)}
                  placeholder="team_plus"
                />
              </div>
              <div className="form-group">
                <label htmlFor="admin-plan-create-display-name">Nom visible</label>
                <input
                  id="admin-plan-create-display-name"
                  type="text"
                  value={newPlan.displayName}
                  onChange={(e) => handlePlanFieldChange('displayName', e.target.value)}
                  placeholder="Team Plus"
                />
              </div>
              <div className="form-group">
                <label htmlFor="admin-plan-create-description">Descripció (opcional)</label>
                <input
                  id="admin-plan-create-description"
                  type="text"
                  value={newPlan.description ?? ''}
                  onChange={(e) => handlePlanFieldChange('description', e.target.value)}
                  placeholder="Pla per equips"
                />
              </div>
              <div className="admin-plan-limits-grid">
                <div className="form-group">
                  <label htmlFor="admin-plan-create-max-servers">Max servidors</label>
                  <input
                    id="admin-plan-create-max-servers"
                    type="number"
                    min={-1}
                    value={newPlan.maxServers}
                    onChange={(e) => handlePlanFieldChange('maxServers', Number(e.target.value || '0'))}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="admin-plan-create-max-text">Max canals text</label>
                  <input
                    id="admin-plan-create-max-text"
                    type="number"
                    min={-1}
                    value={newPlan.maxChannelsTextPerServer}
                    onChange={(e) => handlePlanFieldChange('maxChannelsTextPerServer', Number(e.target.value || '0'))}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="admin-plan-create-max-voice">Max canals veu</label>
                  <input
                    id="admin-plan-create-max-voice"
                    type="number"
                    min={-1}
                    value={newPlan.maxChannelsVoicePerServer}
                    onChange={(e) => handlePlanFieldChange('maxChannelsVoicePerServer', Number(e.target.value || '0'))}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="admin-plan-create-max-members">Max membres</label>
                  <input
                    id="admin-plan-create-max-members"
                    type="number"
                    min={-1}
                    value={newPlan.maxMembersPerServer}
                    onChange={(e) => handlePlanFieldChange('maxMembersPerServer', Number(e.target.value || '0'))}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="admin-plan-create-max-api">API calls/min</label>
                  <input
                    id="admin-plan-create-max-api"
                    type="number"
                    min={-1}
                    value={newPlan.apiCallsPerMinute}
                    onChange={(e) => handlePlanFieldChange('apiCallsPerMinute', Number(e.target.value || '0'))}
                  />
                </div>
                <div className="form-group">
                  <label htmlFor="admin-plan-create-max-messages">Msgs/dia</label>
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
                  {creatingPlan ? 'Creant...' : 'Crear pla'}
                </Button>
              </div>
            </form>
          </section>

          <section className="device-keys-section">
            <h4>Plans {loadingPlans ? '...' : `(${plans.length})`}</h4>
            {!loadingPlans && plans.length === 0 && <p>No hi ha plans disponibles.</p>}
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
                          {isSystem && <span className="admin-badge-active">Sistema</span>}
                          {isEditing ? (
                            <>
                              <Button type="button" variant="primary" size="sm" disabled={isSaving} onClick={() => { void handleUpdatePlan(plan.id) }}>
                                {isSaving ? 'Desant...' : 'Desar'}
                              </Button>
                              <Button type="button" variant="ghost" size="sm" onClick={() => setEditingPlanId(null)}>
                                Cancel·lar
                              </Button>
                            </>
                          ) : (
                            <Button type="button" variant="ghost" size="sm" disabled={isSystem} onClick={() => startEditPlan(plan)}>
                              Modificar
                            </Button>
                          )}
                          <Button
                            type="button"
                            variant="danger"
                            size="sm"
                            disabled={isDeleting || isSystem}
                            onClick={() => setPendingDeletePlanId(plan.id)}
                          >
                            {isDeleting ? 'Esborrant...' : 'Esborrar'}
                          </Button>
                        </div>
                      </div>

                      {isEditing ? (
                        <div className="admin-plan-limits-grid">
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-name-${plan.id}`}>Nom intern</label>
                            <input
                              id={`admin-plan-edit-name-${plan.id}`}
                              type="text"
                              value={editingPlan.name}
                              onChange={(e) => handleEditPlanFieldChange('name', e.target.value)}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-display-${plan.id}`}>Nom visible</label>
                            <input
                              id={`admin-plan-edit-display-${plan.id}`}
                              type="text"
                              value={editingPlan.displayName}
                              onChange={(e) => handleEditPlanFieldChange('displayName', e.target.value)}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-description-${plan.id}`}>Descripció</label>
                            <input
                              id={`admin-plan-edit-description-${plan.id}`}
                              type="text"
                              value={editingPlan.description ?? ''}
                              onChange={(e) => handleEditPlanFieldChange('description', e.target.value)}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-max-servers-${plan.id}`}>Max servidors</label>
                            <input
                              id={`admin-plan-edit-max-servers-${plan.id}`}
                              type="number"
                              min={-1}
                              value={editingPlan.maxServers}
                              onChange={(e) => handleEditPlanFieldChange('maxServers', Number(e.target.value || '0'))}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-max-text-${plan.id}`}>Max canals text</label>
                            <input
                              id={`admin-plan-edit-max-text-${plan.id}`}
                              type="number"
                              min={-1}
                              value={editingPlan.maxChannelsTextPerServer}
                              onChange={(e) => handleEditPlanFieldChange('maxChannelsTextPerServer', Number(e.target.value || '0'))}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-max-voice-${plan.id}`}>Max canals veu</label>
                            <input
                              id={`admin-plan-edit-max-voice-${plan.id}`}
                              type="number"
                              min={-1}
                              value={editingPlan.maxChannelsVoicePerServer}
                              onChange={(e) => handleEditPlanFieldChange('maxChannelsVoicePerServer', Number(e.target.value || '0'))}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-max-members-${plan.id}`}>Max membres</label>
                            <input
                              id={`admin-plan-edit-max-members-${plan.id}`}
                              type="number"
                              min={-1}
                              value={editingPlan.maxMembersPerServer}
                              onChange={(e) => handleEditPlanFieldChange('maxMembersPerServer', Number(e.target.value || '0'))}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-max-api-${plan.id}`}>API calls/min</label>
                            <input
                              id={`admin-plan-edit-max-api-${plan.id}`}
                              type="number"
                              min={-1}
                              value={editingPlan.apiCallsPerMinute}
                              onChange={(e) => handleEditPlanFieldChange('apiCallsPerMinute', Number(e.target.value || '0'))}
                            />
                          </div>
                          <div className="form-group">
                            <label htmlFor={`admin-plan-edit-max-messages-${plan.id}`}>Msgs/dia</label>
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
                          <span>Servidors: <strong>{plan.maxServers === -1 ? '∞' : plan.maxServers}</strong></span>
                          <span>Text: <strong>{plan.maxChannelsTextPerServer === -1 ? '∞' : plan.maxChannelsTextPerServer}</strong></span>
                          <span>Veu: <strong>{plan.maxChannelsVoicePerServer === -1 ? '∞' : plan.maxChannelsVoicePerServer}</strong></span>
                          <span>Membres: <strong>{plan.maxMembersPerServer === -1 ? '∞' : plan.maxMembersPerServer}</strong></span>
                          <span>API/min: <strong>{plan.apiCallsPerMinute === -1 ? '∞' : plan.apiCallsPerMinute}</strong></span>
                          <span>Msgs/dia: <strong>{plan.messagesPerDay === -1 ? '∞' : plan.messagesPerDay}</strong></span>
                        </div>
                      )}

                      {pendingDeletePlanId === plan.id && (
                        <div className="admin-server-delete-confirm">
                          <span>Segur que vols eliminar aquest pla?</span>
                          <div className="admin-server-delete-actions">
                            <Button
                              type="button"
                              variant="danger"
                              size="sm"
                              disabled={isDeleting}
                              onClick={() => { void handleDeletePlan(plan.id) }}
                            >
                              Confirmar
                            </Button>
                            <Button type="button" variant="ghost" size="sm" onClick={() => setPendingDeletePlanId(null)}>
                              Cancel·lar
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
    </div>
  )
}
