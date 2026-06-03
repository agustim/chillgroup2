import React, { useState, useRef, useEffect, createContext, useContext, MutableRefObject, useCallback } from 'react'
import { Server } from '../../types'

type MenuContextType = {
  openMenuServerId: string | null
  setOpenMenuServerId: (id: string | null) => void
  menuButtonRef: MutableRefObject<HTMLButtonElement | null>
  setMenuButtonRef: (ref: HTMLButtonElement | null) => void
  menuPosition: { x: number; y: number } | null
  setMenuPosition: (pos: { x: number; y: number } | null) => void
}

const ServerMenuContext = createContext<MenuContextType>({
  openMenuServerId: null,
  setOpenMenuServerId: () => {},
  menuButtonRef: { current: null },
  setMenuButtonRef: () => {},
  menuPosition: null,
  setMenuPosition: () => {},
})

function useServerMenuContext() {
  return useContext(ServerMenuContext)
}

type ServerMenuAction = 'config' | 'invite' | 'createText' | 'createVoice' | 'leave' | null

interface ServerBarProps {
  servers: Server[]
  selectedServer: string | null
  onSelectServer: (serverId: string) => void
  onCreateServer: () => void
  canCreateServer?: boolean
  isChannelListCollapsed?: boolean
  onShowChannelList?: () => void
  onServerAction?: (action: ServerMenuAction) => void
}

export function ServerBar({
  servers,
  selectedServer,
  onSelectServer,
  onCreateServer,
  canCreateServer = true,
  isChannelListCollapsed = false,
  onShowChannelList,
  onServerAction,
}: ServerBarProps) {
  const [openMenuServerId, setOpenMenuServerId] = useState<string | null>(null)
  const menuButtonRef = useRef<HTMLButtonElement | null>(null)
  const [menuPosition, setMenuPosition] = useState<{ x: number; y: number } | null>(null)
  const menuContentRef = useRef<HTMLDivElement>(null)
  const serverBarRef = useRef<HTMLDivElement>(null)

  // Tancar menú quan es clica fora
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (
        menuContentRef.current &&
        !menuContentRef.current.contains(e.target as Node) &&
        menuButtonRef.current &&
        !menuButtonRef.current.contains(e.target as Node)
      ) {
        setOpenMenuServerId(null)
        setMenuPosition(null)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  const handleMenuToggle = useCallback((serverId: string, e: React.MouseEvent) => {
    e.stopPropagation()

    if (openMenuServerId === serverId) {
      setOpenMenuServerId(null)
      setMenuPosition(null)
      return
    }

    // Calcular posició del botó
    const rect = (e.target as HTMLElement).getBoundingClientRect()
    const serverBarRect = serverBarRef.current?.getBoundingClientRect()

    if (!serverBarRect) return

    // Posicionar el menú a la dreta del server-bar
    const x = serverBarRect.right + 4
    const y = rect.top + rect.height / 2 - 60 // centrat respecte al botó

    setMenuPosition({ x, y })
    setOpenMenuServerId(serverId)
  }, [openMenuServerId])

  const handleMenuAction = (action: ServerMenuAction) => {
    setOpenMenuServerId(null)
    setMenuPosition(null)
    onServerAction?.(action)
  }

  return (
    <div className="server-bar" ref={serverBarRef}>
      {selectedServer && isChannelListCollapsed && (
        <button
          className="server-bar-channel-toggle"
          onClick={onShowChannelList}
          title="Mostrar panell de canals"
          aria-label="Mostrar panell de canals"
        >
          ▶
        </button>
      )}
      {servers.map((server) => (
        <div key={server.serverId} style={{ position: 'relative' }}>
          <button
            className={`server-icon ${selectedServer === server.serverId ? 'active' : ''}`}
            onClick={() => onSelectServer(server.serverId)}
            title={server.name}
          >
            {server.iconUrl ? (
              <img src={server.iconUrl} alt={server.name} />
            ) : (
              <span>{server.name.charAt(0).toUpperCase()}</span>
            )}
          </button>
          {/* Botó de configuració (sempre visible quan hi ha servers) */}
          <button
            ref={openMenuServerId === server.serverId ? menuButtonRef : null}
            className={`server-config-btn ${selectedServer === server.serverId ? 'active' : ''}`}
            onClick={(e) => handleMenuToggle(server.serverId, e)}
            title="Configurar servidor"
          >
            ⚙️
          </button>
        </div>
      ))}
      {canCreateServer && (
        <button className="server-icon add-server" title="Afegir servidor" onClick={onCreateServer}>
          +
        </button>
      )}

      {/* Menú desplegable amb Portal */}
      {openMenuServerId && menuPosition && (
        <div
          ref={menuContentRef}
          className="server-menu"
          style={{
            position: 'fixed',
            left: menuPosition.x,
            top: menuPosition.y,
          }}
        >
          <div className="server-menu-header">
            {servers.find((s) => s.serverId === openMenuServerId)?.name || 'Server'}
          </div>
          <button className="server-menu-item" onClick={() => handleMenuAction('config')}>
            ⚙️ Configurar servidor
          </button>
          <button className="server-menu-item" onClick={() => handleMenuAction('invite')}>
            👥 Convidar al servidor
          </button>
          <button className="server-menu-item" onClick={() => handleMenuAction('createText')}>
            # Crear canal de text
          </button>
          <button className="server-menu-item" onClick={() => handleMenuAction('createVoice')}>
            🔊 Crear canal de veu
          </button>
          {servers.find((s) => s.serverId === openMenuServerId)?.myRole !== 'owner' && (
            <button className="server-menu-item server-menu-item--danger" onClick={() => handleMenuAction('leave')}>
              🚪 Sortir del servidor
            </button>
          )}
        </div>
      )}
    </div>
  )
}
