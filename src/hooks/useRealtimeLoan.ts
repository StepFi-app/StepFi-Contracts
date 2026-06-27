import { useEffect, useRef, useState, useCallback } from 'react'
import { RealtimeChannel } from '../lib/realtime'
import { fetchLoans } from '../lib/soroban'
import type { Loan, ConnectionStatus } from '../types'

interface UseRealtimeLoanReturn {
  loans: Loan[]
  loading: boolean
  error: string | null
  lastUpdated: Date | null
  connectionStatus: ConnectionStatus
  totalActiveLoans: number
  totalOutstanding: string
}

function toLoan(raw: any): Loan {
  return {
    loanId: raw.loanId,
    borrower: raw.borrower,
    vendor: raw.vendor,
    totalAmount: raw.totalAmount,
    guaranteeAmount: raw.guaranteeAmount,
    interestRateBps: raw.interestRateBps,
    interestAmount: raw.interestAmount,
    serviceFeeAmount: raw.serviceFeeAmount,
    principalOutstanding: raw.principalOutstanding,
    interestOutstanding: raw.interestOutstanding,
    serviceFeeOutstanding: raw.serviceFeeOutstanding,
    remainingBalance: raw.remainingBalance,
    repaymentSchedule: raw.repaymentSchedule,
    status: raw.status,
    createdAt: raw.createdAt,
    fundedAt: raw.fundedAt,
    lateFeesOutstanding: raw.lateFeesOutstanding,
  }
}

export function useRealtimeLoan(pollIntervalMs = 10_000): UseRealtimeLoanReturn {
  const [loans, setLoans] = useState<Loan[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null)
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>('disconnected')
  const channelRef = useRef<RealtimeChannel<Loan[]> | null>(null)

  const fetcher = useCallback(async () => {
    const raw = await fetchLoans()
    return raw.map(toLoan)
  }, [])

  useEffect(() => {
    const channel = new RealtimeChannel<Loan[]>(fetcher, pollIntervalMs)
    channelRef.current = channel

    const unsubData = channel.subscribe((data) => {
      setLoans(data)
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

  const totalActiveLoans = loans.filter((l) => l.status === 'Active').length
  const totalOutstanding = loans
    .reduce((sum, l) => sum + BigInt(l.remainingBalance), BigInt(0))
    .toString()

  return {
    loans,
    loading,
    error,
    lastUpdated,
    connectionStatus,
    totalActiveLoans,
    totalOutstanding,
  }
}
