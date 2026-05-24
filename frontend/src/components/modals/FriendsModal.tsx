import React, { useEffect, useMemo, useState } from 'react'

import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'
import type { FriendPresence, UserSearchResult } from '../../types'

interface FriendsModalProps {
  isOpen: boolean
  onClose: () => void
  friends: FriendPresence[]
  onAddFriend: (username: string) => Promise<void>
  onRemoveFriend: (friendUserId: string) => Promise<void>
  onSearchUsers: (query: string) => Promise<UserSearchResult[]>
}

export function FriendsModal({
  isOpen,
  onClose,
  friends,
  onAddFriend,
  onRemoveFriend,
  onSearchUsers,
}: FriendsModalProps) {
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<UserSearchResult[]>([])
  const [isSearching, setIsSearching] = useState(false)
  const [error, setError] = useState('')

  useEffect(() => {
    if (!isOpen) {
      setQuery('')
      setResults([])
      setIsSearching(false)
      setError('')
    }
  }, [isOpen])

  const friendIds = useMemo(() => new Set(friends.map((friend) => friend.userId)), [friends])

  useEffect(() => {
    if (!isOpen) {
      return
    }

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
          if (!cancelled) {
            setResults(nextResults)
          }
        })
        .catch((err) => {
          if (!cancelled) {
            setResults([])
            setError(err instanceof Error ? err.message : 'No s\'ha pogut buscar usuaris')
          }
        })
        .finally(() => {
          if (!cancelled) {
            setIsSearching(false)
          }
        })
    }, 250)

    return () => {
      cancelled = true
      window.clearTimeout(timer)
    }
  }, [isOpen, onSearchUsers, query])

  const handleAddFriend = async (username: string) => {
    await onAddFriend(username)
    setQuery('')
    setResults([])
  }

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Gestió d'amics">
      <div className="modal-inline-stack friends-modal">
        <section className="device-keys-section">
          <h4>Els teus amics</h4>
          {friends.length > 0 ? (
            <ul className="device-keys-list">
              {friends.map((friend) => (
                <li key={friend.userId} className="device-keys-list-item">
                  <div className="device-keys-list-main">
                    <strong>{friend.username}</strong>
                    <span className={`friend-status-pill ${friend.status}`}>
                      {friend.status === 'online' ? 'Actiu' : 'Inactiu'}
                    </span>
                  </div>
                  <div className="device-keys-list-actions">
                    <Button type="button" variant="ghost" onClick={() => { void onRemoveFriend(friend.userId) }}>
                      Treure
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          ) : (
            <p>No tens cap amic desat encara.</p>
          )}
        </section>

        <section className="device-keys-section">
          <h4>Buscar amics</h4>
          <p>Busca qualsevol usuari de l'eina, encara que no comparteixi servidor amb tu.</p>

          <div className="form-group friends-search-group">
            <label htmlFor="friend-search">Cerca</label>
            <input
              id="friend-search"
              type="text"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Cerca per nom d'usuari"
              autoComplete="off"
              autoFocus
            />
          </div>

          {error && <div className="modal-error">{error}</div>}

          {isSearching && <p>Buscant...</p>}

          {results.length > 0 ? (
            <ul className="device-keys-list">
              {results.map((user) => {
                const alreadyFriend = user.isFriend || friendIds.has(user.userId)
                return (
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
                        variant={alreadyFriend ? 'ghost' : 'primary'}
                        onClick={() => { if (!alreadyFriend) { void handleAddFriend(user.username) } }}
                        disabled={alreadyFriend}
                      >
                        {alreadyFriend ? 'Ja és amic' : 'Afegir'}
                      </Button>
                    </div>
                  </li>
                )
              })}
            </ul>
          ) : query.trim().length >= 2 && !isSearching ? (
            <p>No s'han trobat coincidències globals.</p>
          ) : (
            <p>Escriu almenys 2 caràcters per buscar.</p>
          )}
        </section>
      </div>
    </Modal>
  )
}