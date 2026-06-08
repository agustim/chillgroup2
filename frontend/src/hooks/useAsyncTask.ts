import { useState, useCallback } from 'react'

interface RunOptions {
  fallbackError?: string
  onError?: (err: unknown) => boolean
}

export function useAsyncTask() {
  const [isBusy, setIsBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [success, setSuccess] = useState<string | null>(null)

  const run = useCallback(async (
    fn: () => Promise<string | void>,
    options?: string | RunOptions
  ) => {
    const opts: RunOptions = typeof options === 'string'
      ? { fallbackError: options }
      : (options ?? {})

    setIsBusy(true)
    setError(null)
    setSuccess(null)
    try {
      const msg = await fn()
      if (typeof msg === 'string') setSuccess(msg)
    } catch (err) {
      if (opts.onError) {
        const handled = opts.onError(err)
        if (handled) return
      }
      setError(opts.fallbackError ?? (err instanceof Error ? err.message : 'Error desconegut'))
    } finally {
      setIsBusy(false)
    }
  }, [])

  return { isBusy, error, success, run, setError, setSuccess }
}
