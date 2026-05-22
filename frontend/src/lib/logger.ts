type LogLevel = 'debug' | 'info' | 'warn' | 'error'

const LOG_PRIORITY: Record<LogLevel, number> = {
  debug: 10,
  info: 20,
  warn: 30,
  error: 40,
}

function normalizeLevel(value: string | undefined): LogLevel {
  switch (value?.trim().toLowerCase()) {
    case 'debug':
      return 'debug'
    case 'warn':
      return 'warn'
    case 'error':
      return 'error'
    case 'info':
    default:
      return 'info'
  }
}

const currentLevel = normalizeLevel(
  typeof __FRONTEND_DEBUG__ === 'undefined' ? undefined : __FRONTEND_DEBUG__
)

function shouldLog(level: LogLevel): boolean {
  return LOG_PRIORITY[level] >= LOG_PRIORITY[currentLevel]
}

function write(level: LogLevel, args: unknown[]): void {
  if (!shouldLog(level)) {
    return
  }

  switch (level) {
    case 'debug':
      console.debug(...args)
      break
    case 'info':
      console.info(...args)
      break
    case 'warn':
      console.warn(...args)
      break
    case 'error':
      console.error(...args)
      break
  }
}

export const logger = {
  debug: (...args: unknown[]) => write('debug', args),
  info: (...args: unknown[]) => write('info', args),
  warn: (...args: unknown[]) => write('warn', args),
  error: (...args: unknown[]) => write('error', args),
}

export function getFrontendDebugLevel(): LogLevel {
  return currentLevel
}