export interface PoolStats {
  totalLiquidity: string
  lockedLiquidity: string
  availableLiquidity: string
  totalShares: string
  sharePrice: string
}

export interface RepaymentInstallment {
  dueDate: number
  amount: string
  paid: boolean
  paidAt: number
}

export type LoanStatus = 'Pending' | 'Active' | 'Paid' | 'Defaulted' | 'Cancelled'

export interface Loan {
  loanId: number
  borrower: string
  vendor: string
  totalAmount: string
  guaranteeAmount: string
  interestRateBps: number
  interestAmount: string
  serviceFeeAmount: string
  principalOutstanding: string
  interestOutstanding: string
  serviceFeeOutstanding: string
  remainingBalance: string
  repaymentSchedule: RepaymentInstallment[]
  status: LoanStatus
  createdAt: number
  fundedAt: number
  lateFeesOutstanding: string
}

export type ConnectionStatus = 'connected' | 'connecting' | 'disconnected'

export interface RealtimeState<T> {
  data: T | null
  loading: boolean
  error: string | null
  lastUpdated: Date | null
  connectionStatus: ConnectionStatus
}
