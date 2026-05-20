import { io, Socket } from 'socket.io-client'

let socket: Socket | null = null

function getToken(): string | null {
  try {
    return sessionStorage.getItem('chillgroup-token')
  } catch {
    return null
  }
}

export function getSocket(): Socket {
  if (!socket) {
    const token = getToken()
    socket = io('/', {
      auth: { token },
      transports: ['websocket', 'polling'],
      autoConnect: true,
    })

    socket.on('disconnect', () => {
      // Permet recrear la instància si el transport es tanca del tot
      socket = null
    })
  }

  return socket
}

export function disconnectSocket(): void {
  if (socket) {
    socket.disconnect()
    socket = null
  }
}
