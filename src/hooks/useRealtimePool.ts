import { useEffect, useRef, useState, useCallback } from 'react'
import { RealtimeChannel } from '../lib/realtime'
import { fetchPoolStats } from '../lib/soroban'
import type { PoolStats, ConnectionStatus } from '../types'

interface UseRealtimePoolReturn {
  stats: PoolStats | null
  loading: boolean
  error: string | null
  lastUpdated: Date | null
  connectionStatus: ConnectionStatus
}

export function useRealtimePool(pollIntervalMs = 10_000): UseRealtimePoolReturn {
  const [stats, setStats] = useState<PoolStats | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null)
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('disconnected')
  const channelRef = useRef<RealtimeChannel<PoolStats> | null>(null)

  const fetcher = useCallback(async () => {
    const raw = await fetchPoolStats()
    return {
      totalLiquidity: raw.totalLiquidity,
      lockedLiquidity: raw.lockedLiquidity,
      availableLiquidity: raw.availableLiquidity,
      totalShares: raw.totalShares,
      sharePrice: raw.sharePrice,
    }
  }, [])

  useEffect(() => {
    const channel = new RealtimeChannel<PoolStats>(fetcher, pollIntervalMs)
    channelRef.current = channel

    const unsubData = channel.subscribe((data) => {
      setStats(data)
      setLoading(false)
      setError(null)
      setLastUpdated(new Date())
    })

    const unsubStatus = channel.onStatusChange((status) => {
      setConnectionStatus(status)
      if (status === 'disconnected') {
        setError('Unable to reach the Stellar network')
      }
    })

    channel.start()

    return () => {
      channel.stop()
      unsubData()
      unsubStatus()
      channelRef.current = null
    }
  }, [fetcher, pollIntervalMs])

  return { stats, loading, error, lastUpdated, connectionStatus }
}
