import React, { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '../shared/Button'
import type { FriendPresence, UserSearchResult } from '../../types'

interface FriendsPanelProps {
  friends: FriendPresence[]
  onAddFriend: (username: string) => Promise<void>
  onRemoveFriend: (friendUserId: string) => Promise<void>
  onSearchUsers: (query: string) => Promise<UserSearchResult[]>
}

function FriendsContent({
  friends,
  onAddFriend,
  onRemoveFriend,
  onSearchUsers,
}: FriendsPanelProps) {
  const { t } = useTranslation()
  const [query, setQuery] = useState('')
  const [results, setResults] = useState<UserSearchResult[]>([])
  const [isSearching, setIsSearching] = useState(false)
  const [error, setError] = useState('')

  const friendIds = useMemo(() => new Set(friends.map((friend) => friend.userId)), [friends])

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
          if (!cancelled) {
            setResults(nextResults)
          }
        })
        .catch((err) => {
          if (!cancelled) {
            setResults([])
            setError(err instanceof Error ? err.message : t('friends.errSearch'))
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
  }, [onSearchUsers, query])

  const handleAddFriend = async (username: string) => {
    await onAddFriend(username)
    setQuery('')
    setResults([])
  }

  return (
    <div className="modal-inline-stack friends-modal">
      <section className="device-keys-section">
        <h4>{t('friends.yourFriends')}</h4>
        {friends.length > 0 ? (
          <ul className="device-keys-list">
            {friends.map((friend) => (
              <li key={friend.userId} className="device-keys-list-item">
                <div className="device-keys-list-main">
                  <strong>{friend.username}</strong>
                  <span className={`friend-status-pill ${friend.status}`}>
                    {friend.status === 'online' ? t('channels.online') : t('channels.offline')}
                  </span>
                </div>
                <div className="device-keys-list-actions">
                  <Button type="button" variant="ghost" onClick={() => { void onRemoveFriend(friend.userId) }}>
                    {t('friends.remove')}
                  </Button>
                </div>
              </li>
            ))}
          </ul>
        ) : (
          <p>{t('friends.noFriends')}</p>
        )}
      </section>

      <section className="device-keys-section">
        <h4>{t('friends.searchFriends')}</h4>
        <p>{t('friends.searchDesc')}</p>

        <div className="form-group friends-search-group">
          <label htmlFor="friend-search">{t('friends.searchLabel')}</label>
          <input
            id="friend-search"
            type="text"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={t('friends.searchPlaceholder')}
            autoComplete="off"
            autoFocus
          />
        </div>

        {error && <div className="modal-error">{error}</div>}

        {isSearching && <p>{t('friends.searching')}</p>}

        {results.length > 0 ? (
          <ul className="device-keys-list">
            {results.map((user) => {
              const alreadyFriend = user.isFriend || friendIds.has(user.userId)
              return (
                <li key={user.userId} className="device-keys-list-item">
                  <div className="device-keys-list-main">
                    <strong>{user.username}</strong>
                    <span className={`friend-status-pill ${user.status}`}>
                      {user.status === 'online' ? t('channels.online') : t('channels.offline')}
                    </span>
                  </div>
                  <div className="device-keys-list-actions">
                    <Button
                      type="button"
                      variant={alreadyFriend ? 'ghost' : 'primary'}
                      onClick={() => { if (!alreadyFriend) { void handleAddFriend(user.username) } }}
                      disabled={alreadyFriend}
                    >
                      {alreadyFriend ? t('friends.alreadyFriend') : t('friends.add')}
                    </Button>
                  </div>
                </li>
              )
            })}
          </ul>
        ) : query.trim().length >= 2 && !isSearching ? (
          <p>{t('friends.noResults')}</p>
        ) : (
          <p>{t('friends.minChars')}</p>
        )}
      </section>
    </div>
  )
}

export function FriendsPanel(props: FriendsPanelProps) {
  return <FriendsContent {...props} />
}
