import React, { useEffect, useState } from 'react'

import {
  adminUsersCreate,
  adminUsersDelete,
  adminUsersList,
  adminUsersUpdatePlan,
  adminUsersUpdateRole,
  type AdminUserItem,
  type AdminUserRole,
} from '../../lib/api'
import { Button } from '../shared/Button'

interface AdminUsersPanelProps {
  isOpen: boolean
  onClose: () => void
  onFeedback: (message: string) => void
}

interface PlanOption {
  id: string
  label: string
}

const PLAN_OPTIONS: PlanOption[] = [
  { id: '550e8400-e29b-41d4-a716-446655441001', label: 'Free' },
  { id: '550e8400-e29b-41d4-a716-446655441002', label: 'Pro' },
  { id: '550e8400-e29b-41d4-a716-446655441003', label: 'Enterprise' },
]

export function AdminUsersPanel({ isOpen, onClose, onFeedback }: AdminUsersPanelProps) {
  const [users, setUsers] = useState<AdminUserItem[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const [newUsername, setNewUsername] = useState('')
  const [newPassword, setNewPassword] = useState('')
  const [newRole, setNewRole] = useState<AdminUserRole>('user')
  const [newPlanId, setNewPlanId] = useState<string>(PLAN_OPTIONS[0].id)
  const [isCreating, setIsCreating] = useState(false)

  const loadUsers = async () => {
    setLoading(true)
    setError('')

    const result = await adminUsersList()
    if (result.success) {
      setUsers(result.data)
      setLoading(false)
      return
    }

    setLoading(false)
    setError(result.error.message)
  }

  useEffect(() => {
    if (!isOpen) {
      return
    }

    setNewUsername('')
    setNewPassword('')
    setNewRole('user')
    setNewPlanId(PLAN_OPTIONS[0].id)
    void loadUsers()
  }, [isOpen])

  const handleCreateUser = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    const username = newUsername.trim()

    if (!username || !newPassword.trim()) {
      setError('Usuari i contrasenya son obligatoris')
      return
    }

    setIsCreating(true)
    setError('')

    const result = await adminUsersCreate(username, newPassword, newRole, newPlanId)
    setIsCreating(false)

    if (!result.success) {
      setError(result.error.message)
      return
    }

    onFeedback(`Usuari ${result.data.username} creat`)
    setNewUsername('')
    setNewPassword('')
    setNewRole('user')
    setNewPlanId(PLAN_OPTIONS[0].id)
    await loadUsers()
  }

  const handleRoleChange = async (userId: string, role: AdminUserRole) => {
    const result = await adminUsersUpdateRole(userId, role)
    if (!result.success) {
      setError(result.error.message)
      return
    }

    setUsers((current) => current.map((user) => (user.userId === userId ? { ...user, role } : user)))
    onFeedback('Rol actualitzat')
  }

  const handlePlanChange = async (userId: string, planId: string) => {
    const result = await adminUsersUpdatePlan(userId, planId)
    if (!result.success) {
      setError(result.error.message)
      return
    }

    setUsers((current) => current.map((user) => (user.userId === userId ? { ...user, planId } : user)))
    onFeedback('Pla actualitzat')
  }

  const handleDeleteUser = async (userId: string, username: string) => {
    const result = await adminUsersDelete(userId)
    if (!result.success) {
      setError(result.error.message)
      return
    }

    setUsers((current) => current.filter((user) => user.userId !== userId))
    onFeedback(`Usuari ${username} eliminat`)
  }

  if (!isOpen) return null

  return (
    <div className="panel admin-users-panel">
      <div className="admin-users-panel-header">
        <h3>Gestio usuaris (admin)</h3>
        <Button type="button" variant="ghost" size="sm" onClick={onClose}>Tancar</Button>
      </div>

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
                onChange={(event) => setNewUsername(event.target.value)}
                placeholder="nou-usuari"
              />
            </div>

            <div className="form-group">
              <label htmlFor="admin-create-password">Contrasenya</label>
              <input
                id="admin-create-password"
                type="password"
                value={newPassword}
                onChange={(event) => setNewPassword(event.target.value)}
                placeholder="********"
              />
            </div>

            <div className="form-group">
              <label htmlFor="admin-create-role">Rol</label>
              <select
                id="admin-create-role"
                value={newRole}
                onChange={(event) => setNewRole((event.target.value === 'admin' ? 'admin' : 'user') as AdminUserRole)}
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
                onChange={(event) => setNewPlanId(event.target.value)}
              >
                {PLAN_OPTIONS.map((plan) => (
                  <option key={plan.id} value={plan.id}>{plan.label}</option>
                ))}
              </select>
            </div>

            <div className="modal-actions-row">
              <Button type="submit" disabled={isCreating}>
                {isCreating ? 'Creant...' : 'Crear usuari'}
              </Button>
            </div>
          </form>
        </section>

        <section className="device-keys-section">
          <h4>Usuaris</h4>
          {loading && <p>Carregant usuaris...</p>}
          {error && <div className="modal-error">{error}</div>}
          {!loading && users.length === 0 && <p>No hi ha usuaris.</p>}

          {users.length > 0 && (
            <ul className="device-keys-list">
              {users.map((user) => (
                <li key={user.userId} className="device-keys-list-item">
                  <div className="device-keys-list-main">
                    <strong>{user.username}</strong>
                    <span>ID: {user.userId}</span>
                  </div>
                  <div className="device-keys-list-actions">
                    <select
                      aria-label={`rol-${user.username}`}
                      value={user.role}
                      onChange={(event) => {
                        const role = event.target.value === 'admin' ? 'admin' : 'user'
                        void handleRoleChange(user.userId, role)
                      }}
                    >
                      <option value="user">User</option>
                      <option value="admin">Admin</option>
                    </select>
                    <select
                      aria-label={`pla-${user.username}`}
                      value={user.planId ?? PLAN_OPTIONS[0].id}
                      onChange={(event) => { void handlePlanChange(user.userId, event.target.value) }}
                    >
                      {PLAN_OPTIONS.map((plan) => (
                        <option key={plan.id} value={plan.id}>{plan.label}</option>
                      ))}
                    </select>
                    <Button type="button" variant="danger" onClick={() => { void handleDeleteUser(user.userId, user.username) }}>
                      Eliminar
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  )
}
