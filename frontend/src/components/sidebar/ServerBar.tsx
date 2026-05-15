import React from 'react'
import { Server } from '../../types'

interface ServerBarProps {
  servers: Server[]
  selectedServer: string | null
  onSelectServer: (serverId: string) => void
}

export function ServerBar({ servers, selectedServer, onSelectServer }: ServerBarProps) {
  return (
    <div className="server-bar">
      {servers.map((server) => (
        <button
          key={server.serverId}
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
      ))}
      <button className="server-icon add-server" title="Afegir servidor">
        +
      </button>
    </div>
  )
}