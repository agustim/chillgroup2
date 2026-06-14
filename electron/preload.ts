import { contextBridge, ipcRenderer } from 'electron'

contextBridge.exposeInMainWorld('electronAPI', {
  getServerUrl: (): Promise<string> => ipcRenderer.invoke('get-server-url'),
  setServerUrl: (url: string): Promise<void> => ipcRenderer.invoke('set-server-url', url),
  openSetupWindow: (): Promise<void> => ipcRenderer.invoke('open-setup-window'),
  emitSetupComplete: (): void => { ipcRenderer.send('setup-complete') },
})
