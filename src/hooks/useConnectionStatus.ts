import { useEffect, useState } from 'react'
import { checkConnection } from '../lib/soroban'
import type { ConnectionStatus } from '../types'

export function useConnectionStatus(): ConnectionStatus {
  const [status, setStatus] = useState<ConnectionStatus>('connecting')

  useEffect(() => {
    let cancelled = false

    const poll = async () => {
      setStatus('connecting')
      try {
        const ok = await checkConnection()
        if (!cancelled) {
          setStatus(ok ? 'connected' : 'disconnected')
        }
      } catch {
        if (!cancelled) {
          setStatus('disconnected')
        }
      }
    }

    poll()
    const id = setInterval(poll, 15_000)
    return () => {
      cancelled = true
      clearInterval(id)
    }
  }, [])

  return status
}
