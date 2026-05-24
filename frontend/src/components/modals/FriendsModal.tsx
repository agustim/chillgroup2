import React, { useEffect, useMemo, useState } from 'react'

import { Modal } from '../ui/Modal'
import { Button } from '../shared/Button'
import type { Friend, FriendPresence } from '../../types'

interface FriendsModalProps {
  isOpen: boolean
  onClose: () => void
  friends: FriendPresence[]
  knownUsers: FriendPresence[]
  onAddFriend: (friend: Friend) => void
  onRemoveFriend: (userId: string) => void
}

function normalizeName(name: string): string {
  return name.trim().toLowerCase()
}

export function FriendsModal({
  isOpen,
  onClose,
  friends,
  knownUsers,
  onAddFriend,
  onRemoveFriend,
}: FriendsModalProps) {
  const [query, setQuery] = useState('')

  useEffect(() => {
    if (!isOpen) {
      setQuery('')
    }
  }, [isOpen])

  const normalizedQuery = normalizeName(query)

  const matchingUsers = useMemo(() => {
    if (!normalizedQuery) return []
    const friendIds = new Set(friends.map((friend) => friend.userId))
    return knownUsers.filter((user) => {
      if (friendIds.has(user.userId)) return false
      return user.username.toLowerCase().includes(normalizedQuery)
    })
  }, [friends, knownUsers, normalizedQuery])

  const canCreateLocalFriend = normalizedQuery.length >= 3
    && !friends.some((friend) => friend.username.toLowerCase() === normalizedQuery)
    && !knownUsers.some((user) => user.username.toLowerCase() === normalizedQuery)

  const handleAddFriend = (friend: Friend) => {
    onAddFriend(friend)
    setQuery('')
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
                    <span className={`friend-status-pill ${friend.isOnline ? 'online' : 'offline'}`}>
                      {friend.isOnline ? 'Actiu' : 'Inactiu'}
                    </span>
                  </div>
                  <div className="device-keys-list-actions">
                    <Button type="button" variant="ghost" onClick={() => onRemoveFriend(friend.userId)}>
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
          <p>Escriu un nom per buscar gent del servidor i afegir-la a la llista.</p>

          <div className="form-group friends-search-group">
            <label htmlFor="friend-search">Cerca</label>
            <input
              id="friend-search"
              type="text"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="Cerca per nom"
              autoComplete="off"
              autoFocus
            />
          </div>

          {matchingUsers.length > 0 ? (
            <ul className="device-keys-list">
              {matchingUsers.map((user) => (
                <li key={user.userId} className="device-keys-list-item">
                  <div className="device-keys-list-main">
                    <strong>{user.username}</strong>
                    <span className={`friend-status-pill ${user.isOnline ? 'online' : 'offline'}`}>
                      {user.isOnline ? 'Actiu' : 'Inactiu'}
                    </span>
                  </div>
                  <div className="device-keys-list-actions">
                    <Button type="button" variant="primary" onClick={() => handleAddFriend({ userId: user.userId, username: user.username })}>
                      Afegir
                    </Button>
                  </div>
                </li>
              ))}
            </ul>
          ) : normalizedQuery ? (
            <p>No s'han trobat coincidències entre els amics coneguts.</p>
          ) : (
            <p>Busca un nom per veure suggeriments.</p>
          )}

          {canCreateLocalFriend && (
            <div className="friends-search-empty">
              <p>Vols afegir <strong>{query.trim()}</strong> com a amic local?</p>
              <Button
                type="button"
                variant="secondary"
                onClick={() => handleAddFriend({ userId: `local:${normalizeName(query)}`, username: query.trim() })}
              >
                Afegir contacte
              </Button>
            </div>
          )}
        </section>
      </div>
    </Modal>
  )
}