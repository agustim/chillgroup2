import {
  app,
  BrowserWindow,
  Tray,
  Menu,
  nativeImage,
  ipcMain,
  protocol,
} from 'electron'
import { readFileSync, writeFileSync, existsSync, statSync, mkdirSync } from 'fs'
import { join, extname } from 'path'
import { autoUpdater } from 'electron-updater'

// Must be called before app is ready
protocol.registerSchemesAsPrivileged([
  {
    scheme: 'app',
    privileges: { secure: true, standard: true, corsEnabled: true, supportFetchAPI: true },
  },
])

const IS_DEV = !app.isPackaged
const FRONTEND_DIST = join(__dirname, '../../frontend/dist')

function getConfigPath(): string {
  return join(app.getPath('userData'), 'config.json')
}

function readConfig(): Record<string, string> {
  try {
    return JSON.parse(readFileSync(getConfigPath(), 'utf-8'))
  } catch {
    return {}
  }
}

function saveConfig(patch: Record<string, string>): void {
  const dir = app.getPath('userData')
  mkdirSync(dir, { recursive: true })
  writeFileSync(getConfigPath(), JSON.stringify({ ...readConfig(), ...patch }, null, 2))
}

function getIconPath(): string {
  if (IS_DEV) return join(__dirname, '../../src-tauri/icons/icon.png')
  return join(process.resourcesPath, 'icon.png')
}

const MIME: Record<string, string> = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'application/javascript',
  '.mjs': 'application/javascript',
  '.css': 'text/css',
  '.json': 'application/json',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.webp': 'image/webp',
}

function serveFile(urlPath: string): Response {
  let filePath = join(FRONTEND_DIST, urlPath === '/' ? 'index.html' : urlPath)
  if (!existsSync(filePath) || statSync(filePath).isDirectory()) {
    filePath = join(FRONTEND_DIST, 'index.html')
  }
  try {
    return new Response(readFileSync(filePath), {
      headers: {
        'Content-Type': MIME[extname(filePath).toLowerCase()] ?? 'application/octet-stream',
        'Cross-Origin-Opener-Policy': 'same-origin',
        'Cross-Origin-Embedder-Policy': 'require-corp',
      },
    })
  } catch {
    return new Response('Not Found', { status: 404 })
  }
}

let mainWindow: BrowserWindow | null = null
let setupWindow: BrowserWindow | null = null
let tray: Tray | null = null

function windowOpts(extra: Electron.BrowserWindowConstructorOptions = {}): Electron.BrowserWindowConstructorOptions {
  return {
    show: false,
    icon: getIconPath(),
    webPreferences: {
      preload: join(__dirname, 'preload.js'),
      nodeIntegration: false,
      contextIsolation: true,
      sandbox: false,
    },
    ...extra,
  }
}

function openMain(): void {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.show()
    mainWindow.focus()
    return
  }
  mainWindow = new BrowserWindow(
    windowOpts({ width: 1200, height: 800, minWidth: 800, minHeight: 600, title: 'ChillGroup' })
  )
  const url = IS_DEV ? 'http://localhost:5173' : 'app://localhost/'
  mainWindow.loadURL(url)
  mainWindow.once('ready-to-show', () => mainWindow?.show())
  mainWindow.on('close', (e) => { e.preventDefault(); mainWindow?.hide() })
}

function openSetup(): void {
  if (setupWindow && !setupWindow.isDestroyed()) {
    setupWindow.show()
    setupWindow.focus()
    return
  }
  setupWindow = new BrowserWindow(
    windowOpts({ width: 440, height: 300, resizable: false, title: 'Configurar ChillGroup' })
  )
  const url = IS_DEV ? 'http://localhost:5173/setup.html' : 'app://localhost/setup.html'
  setupWindow.loadURL(url)
  setupWindow.once('ready-to-show', () => setupWindow?.show())
}

function buildTray(): void {
  const iconPath = getIconPath()
  const icon = existsSync(iconPath)
    ? nativeImage.createFromPath(iconPath)
    : nativeImage.createEmpty()

  tray = new Tray(icon)
  tray.setToolTip('ChillGroup')
  tray.setContextMenu(
    Menu.buildFromTemplate([
      { label: 'Obrir ChillGroup', click: () => openMain() },
      { label: 'Canviar servidor', click: () => openSetup() },
      { type: 'separator' },
      { label: 'Sortir', click: () => app.exit(0) },
    ])
  )
  tray.on('click', () => openMain())
}

// IPC
ipcMain.handle('get-server-url', () => readConfig()['server_url'] ?? '')
ipcMain.handle('set-server-url', (_e, url: string) => saveConfig({ server_url: url }))
ipcMain.handle('open-setup-window', () => openSetup())
ipcMain.on('setup-complete', () => {
  if (setupWindow && !setupWindow.isDestroyed()) setupWindow.close()
  openMain()
  if (mainWindow && !mainWindow.isDestroyed()) mainWindow.webContents.reload()
})

app.whenReady().then(() => {
  protocol.handle('app', (req) => serveFile(new URL(req.url).pathname))

  buildTray()

  if (!readConfig()['server_url']) {
    openSetup()
  } else {
    openMain()
  }

  if (!IS_DEV) {
    autoUpdater.checkForUpdatesAndNotify().catch(() => {})
  }
})

// Keep running in tray when all windows are closed
app.on('window-all-closed', () => {})
