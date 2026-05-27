import React, { useEffect, useState } from 'react'

import {
  adminUserLimitsGet,
  adminUsersCreate,
  adminUsersDelete,
  adminUsersList,
  adminUsersUpdatePlan,
  adminUsersUpdateRole,
  invitationsCreate,
  invitationsList,
  type AdminUserLimitsInfo,
  type AdminUserItem,
  type AdminUserRole,
  type InvitationListItem,
} from '../../lib/api'
import { Button } from '../shared/Button'

interface AdminUsersPanelProps {
  isOpen: boolean
  onClose: () => void
  onFeedback: (message: string) => void
}

const PLAN_OPTIONS = [
  { id: '550e8400-e29b-41d4-a716-446655441001', label: 'Free' },
  { id: '550e8400-e29b-41d4-a716-446655441002', label: 'Pro' },
  { id: '550e8400-e29b-41d4-a716-446655441003', label: 'Enterprise' },
]

type ActiveTab = 'users' | 'invitations'

export function AdminUsersPanel({ isOpen, onClose, onFeedback }: AdminUsersPanelProps) {
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
  const [lastCreatedCode, setLastCreatedCode] = useState<string | null>(null)
  const [isCreatingInvite, setIsCreatingInvite] = useState(false)

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

  useEffect(() => {
    if (!isOpen) return
    setError('')
    setNewUsername('')
    setNewPassword('')
    setNewRole('user')
    setNewPlanId(PLAN_OPTIONS[0].id)
    setInviteMaxUses(1)
    setLastCreatedCode(null)
    void loadUsers()
    void loadInvitations()
  }, [isOpen])

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
    const result = await invitationsCreate(maxUses)
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
    </div>
  )
}
