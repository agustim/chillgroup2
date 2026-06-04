import React, { useEffect, useState } from 'react'
import { Button } from './Button'
import type { UserSearchResult } from '../../types'

interface InviteUserSearchProps {
  onSearchUsers: (query: string) => Promise<UserSearchResult[]>
  onInvite: (username: string) => Promise<void>
}

export function InviteUserSearch({ onSearchUsers, onInvite }: InviteUserSearchProps) {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<UserSearchResult[]>([])
  const [isSearching, setIsSearching] = useState(false)
  const [inviting, setInviting] = useState<string | null>(null)
  const [error, setError] = useState('')

  useEffect(() => {
    const trimmed = query.trim()
    if (trimmed.length < 2) {
      setResults([])
      setError('')
      setIsSearching(false)
      return
    }

    let cancelled = false
    const timer = window.setTimeout(() => {
      setIsSearching(true)
      setError('')
      void onSearchUsers(trimmed)
        .then((nextResults) => {
          if (!cancelled) setResults(nextResults)
        })
        .catch((err) => {
          if (!cancelled) {
            setResults([])
            setError(err instanceof Error ? err.message : 'No s\'ha pogut buscar usuaris')
          }
        })
        .finally(() => {
          if (!cancelled) setIsSearching(false)
        })
    }, 250)

    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [onSearchUsers, query])

  const handleInvite = async (username: string) => {
    setInviting(username)
    try {
      await onInvite(username)
      setQuery('')
      setResults([])
    } finally {
      setInviting(null)
    }
  }

  return (
    <div>
      <div className="form-group friends-search-group">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Cerca per nom d'usuari"
          autoComplete="off"
          autoFocus
        />
      </div>

      {error && <div className="modal-error">{error}</div>}
      {isSearching && <p style={{ color: 'var(--text-secondary)', fontSize: '14px' }}>Buscant...</p>}

      {results.length > 0 ? (
        <ul className="device-keys-list">
          {results.map((user) => (
            <li key={user.userId} className="device-keys-list-item">
              <div className="device-keys-list-main">
                <strong>{user.username}</strong>
                <span className={`friend-status-pill ${user.status}`}>
                  {user.status === 'online' ? 'Actiu' : 'Inactiu'}
                </span>
              </div>
              <div className="device-keys-list-actions">
                <Button
                  type="button"
                  variant="primary"
                  onClick={() => { void handleInvite(user.username) }}
                  disabled={inviting === user.username}
                >
                  {inviting === user.username ? 'Enviant...' : 'Convidar'}
                </Button>
              </div>
            </li>
          ))}
        </ul>
      ) : query.trim().length >= 2 && !isSearching ? (
        <p style={{ color: 'var(--text-secondary)', fontSize: '14px' }}>No s&apos;han trobat coincidències.</p>
      ) : (
        <p style={{ color: 'var(--text-secondary)', fontSize: '14px' }}>Escriu almenys 2 caràcters per buscar.</p>
      )}
    </div>
  )
}
