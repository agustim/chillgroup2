import React, { useState, useRef, useEffect } from 'react'
import { Server } from '../../types'

type ServerMenuAction = 'config' | 'invite' | 'createText' | 'createVoice' | 'devices' | null

interface ServerBarProps {
  servers: Server[]
  selectedServer: string | null
  onSelectServer: (serverId: string) => void
  onCreateServer: () => void
  onServerAction?: (action: ServerMenuAction) => void
}

export function ServerBar({ servers, selectedServer, onSelectServer, onCreateServer, onServerAction }: ServerBarProps) {
  const [menuOpen, setMenuOpen] = useState(false)
  const [menuServerId, setMenuServerId] = useState<string | null>(null)
  const menuRef = useRef<HTMLDivElement>(null)

  // Tancar menú quan es clica fora
  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpen(false)
        setMenuServerId(null)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [])

  const handleMenuToggle = (serverId: string, e: React.MouseEvent) => {
    e.stopPropagation()
    if (menuServerId === serverId && menuOpen) {
      setMenuOpen(false)
      setMenuServerId(null)
    } else {
      setMenuServerId(serverId)
      setMenuOpen(true)
    }
  }

  const handleMenuAction = (action: ServerMenuAction) => {
    setMenuOpen(false)
    setMenuServerId(null)
    onServerAction?.(action)
  }

  const selectedServerData = servers.find((s) => s.serverId === selectedServer)

  return (
    <div className="server-bar">
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
          {/* Botó de configuració (visible al fer hover o si està actiu) */}
          {selectedServer === server.serverId && (
            <button
              className="server-config-btn"
              onClick={(e) => handleMenuToggle(server.serverId, e)}
              title="Configurar servidor"
            >
              ⚙️
            </button>
          )}
          {/* Menú desplegable */}
          {menuOpen && menuServerId === server.serverId && (
            <div ref={menuRef} className="server-menu">
              <div className="server-menu-header">{server.name}</div>
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
              <button className="server-menu-item" onClick={() => handleMenuAction('devices')}>
                🖥️ Gestió de dispositius
              </button>
            </div>
          )}
        </div>
      ))}
      <button className="server-icon add-server" title="Afegir servidor" onClick={onCreateServer}>
        +
      </button>
    </div>
  )
}
