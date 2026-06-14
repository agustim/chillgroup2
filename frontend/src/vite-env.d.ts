/// <reference types="vite/client" />

declare const __FRONTEND_DEBUG__: string
declare const __LIVEKIT_HOST__: string
declare const __OPEN_REGISTER__: string

interface ElectronAPI {
  getServerUrl: () => Promise<string>
  setServerUrl: (url: string) => Promise<void>
  openSetupWindow: () => Promise<void>
  emitSetupComplete: () => void
  notify: (title: string, body: string) => Promise<void>
  clearNotification: () => Promise<void>
}

interface Window {
  electronAPI?: ElectronAPI
}
