import React, { useEffect, useState } from 'react'

import {
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

const PLAN_OPTIONS = [
  { id: '550e8400-e29b-41d4-a716-446655441001', label: 'Free' },
  { id: '550e8400-e29b-41d4-a716-446655441002', label: 'Pro' },
  { id: '550e8400-e29b-41d4-a716-446655441003', label: 'Enterprise' },
]

type ActiveTab = 'users' | 'invitations' | 'servers'

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
  const [newPlanId, setNewPlanId] = useState(PLAN_OPTIONS[0].id)
  const [isCreatingUser, setIsCreatingUser] = useState(false)
  const [expandedTierUserId, setExpandedTierUserId] = useState<string | null>(null)
  const [tierByUserId, setTierByUserId] = useState<Record<string, AdminUserLimitsInfo>>({})
  const [loadingTierUserId, setLoadingTierUserId] = useState<string | null>(null)

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
    setNewPlanId(PLAN_OPTIONS[0].id)
    setInviteMaxUses(1)
    setInviteServerId(selectedServerId ?? '')
    setLastCreatedCode(null)
    setNewServerName('')
    setNewServerIconUrl('')
    setEditingServerId(null)
    setEditingServerName('')
    setEditingServerIconUrl('')
    setPendingDeleteServerId(null)
    void loadUsers()
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
    setNewPlanId(PLAN_OPTIONS[0].id)
    await loadUsers()
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
                  onChange={(e) => setNewPlanId(e.target.value)}
                >
                  {PLAN_OPTIONS.map((p) => <option key={p.id} value={p.id}>{p.label}</option>)}
                </select>
              </div>
              <div className="modal-actions-row">
                <Button type="submit" disabled={isCreatingUser}>
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
                          value={user.planId ?? PLAN_OPTIONS[0].id}
                          onChange={(e) => { void handlePlanChange(user.userId, e.target.value) }}
                        >
                          {PLAN_OPTIONS.map((p) => <option key={p.id} value={p.id}>{p.label}</option>)}
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
    </div>
  )
}
