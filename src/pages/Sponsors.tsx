import { useRealtimePool } from '../hooks/useRealtimePool'
import { useRealtimeLoan } from '../hooks/useRealtimeLoan'

function formatUnits(value: string, decimals = 7): string {
  const num = Number(value) / 10 ** decimals
  if (num >= 1_000_000) return (num / 1_000_000).toFixed(2) + 'M'
  if (num >= 1_000) return (num / 1_000).toFixed(2) + 'K'
  return num.toFixed(2)
}

interface StatCardProps {
  title: string
  value: string
  changed: boolean
}

function StatCard({ title, value, changed }: StatCardProps) {
  return (
    <div
      style={{
        padding: '16px 20px',
        borderRadius: 12,
        border: `1px solid ${changed ? '#22c55e' : '#e5e7eb'}`,
        background: changed ? '#f0fdf4' : '#fff',
        transition: 'all 0.3s ease',
      }}
    >
      <div style={{ fontSize: 12, color: '#6b7280', marginBottom: 4 }}>{title}</div>
      <div style={{ fontSize: 22, fontWeight: 700 }}>{value}</div>
      {changed && (
        <div style={{ fontSize: 11, color: '#16a34a', marginTop: 4 }}>
          ● Updated
        </div>
      )}
    </div>
  )
}

export function Sponsors() {
  const {
    stats,
    loading: poolLoading,
    error: poolError,
    lastUpdated: poolUpdated,
    connectionStatus: poolConn,
  } = useRealtimePool(10_000)

  const {
    loans,
    loading: loanLoading,
    error: loanError,
    lastUpdated: loanUpdated,
    totalActiveLoans,
    totalOutstanding,
  } = useRealtimeLoan(10_000)

  const offline = poolConn === 'disconnected'

  if (poolLoading && loanLoading) {
    return (
      <div style={{ padding: 32, textAlign: 'center', color: '#6b7280' }}>
        Loading pool data…
      </div>
    )
  }

  return (
    <div style={{ padding: 32, maxWidth: 960, margin: '0 auto' }}>
      {offline && (
        <div
          style={{
            padding: '12px 16px',
            marginBottom: 24,
            borderRadius: 8,
            background: '#fef2f2',
            border: '1px solid #fecaca',
            color: '#dc2626',
            fontSize: 14,
          }}
        >
          Unable to reach the Stellar network — data may be stale.
          Reconnecting…
        </div>
      )}

      <h1 style={{ fontSize: 24, fontWeight: 700, marginBottom: 24 }}>
        Sponsor Dashboard
      </h1>

      <div
        style={{
          display: 'grid',
          gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 1fr))',
          gap: 16,
          marginBottom: 32,
        }}
      >
        <StatCard
          title="Total Liquidity"
          value={`${formatUnits(stats?.totalLiquidity ?? '0')} XLM`}
          changed={!!poolUpdated}
        />
        <StatCard
          title="Available Liquidity"
          value={`${formatUnits(stats?.availableLiquidity ?? '0')} XLM`}
          changed={!!poolUpdated}
        />
        <StatCard
          title="Locked Liquidity"
          value={`${formatUnits(stats?.lockedLiquidity ?? '0')} XLM`}
          changed={!!poolUpdated}
        />
        <StatCard
          title="Share Price"
          value={`${formatUnits(stats?.sharePrice ?? '0', 4)} XLM`}
          changed={!!poolUpdated}
        />
        <StatCard
          title="Active Loans"
          value={String(totalActiveLoans)}
          changed={!!loanUpdated}
        />
        <StatCard
          title="Total Outstanding"
          value={`${formatUnits(totalOutstanding)} XLM`}
          changed={!!loanUpdated}
        />
      </div>

      {poolError && (
        <p style={{ color: '#dc2626', fontSize: 13, marginBottom: 16 }}>
          Pool error: {poolError}
        </p>
      )}
      {loanError && (
        <p style={{ color: '#dc2626', fontSize: 13, marginBottom: 16 }}>
          Loan error: {loanError}
        </p>
      )}

      <div style={{ fontSize: 12, color: '#9ca3af', marginBottom: 16 }}>
        Auto-updates every 10s
        {poolUpdated && <> · Pool: {poolUpdated.toLocaleTimeString()}</>}
        {loanUpdated && <> · Loans: {loanUpdated.toLocaleTimeString()}</>}
      </div>

      <h2 style={{ fontSize: 18, fontWeight: 600, marginBottom: 12 }}>
        Active Loans
      </h2>

      {loans.length === 0 ? (
        <p style={{ color: '#6b7280' }}>No loans found.</p>
      ) : (
        <div style={{ overflowX: 'auto' }}>
          <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: 14 }}>
            <thead>
              <tr style={{ borderBottom: '2px solid #e5e7eb', textAlign: 'left' }}>
                <th style={{ padding: '8px 12px' }}>ID</th>
                <th style={{ padding: '8px 12px' }}>Status</th>
                <th style={{ padding: '8px 12px' }}>Amount</th>
                <th style={{ padding: '8px 12px' }}>Remaining</th>
                <th style={{ padding: '8px 12px' }}>Borrower</th>
              </tr>
            </thead>
            <tbody>
              {loans.map((loan) => (
                <tr
                  key={loan.loanId}
                  style={{ borderBottom: '1px solid #f3f4f6' }}
                >
                  <td style={{ padding: '8px 12px' }}>{loan.loanId}</td>
                  <td style={{ padding: '8px 12px' }}>
                    <span
                      style={{
                        display: 'inline-block',
                        padding: '2px 8px',
                        borderRadius: 999,
                        fontSize: 12,
                        fontWeight: 600,
                        background:
                          loan.status === 'Active'
                            ? '#dcfce7'
                            : loan.status === 'Paid'
                              ? '#e0f2fe'
                              : loan.status === 'Defaulted'
                                ? '#fecaca'
                                : '#f3f4f6',
                        color:
                          loan.status === 'Active'
                            ? '#16a34a'
                            : loan.status === 'Paid'
                              ? '#0284c7'
                              : loan.status === 'Defaulted'
                                ? '#dc2626'
                                : '#6b7280',
                      }}
                    >
                      {loan.status}
                    </span>
                  </td>
                  <td style={{ padding: '8px 12px' }}>
                    {formatUnits(loan.totalAmount)} XLM
                  </td>
                  <td style={{ padding: '8px 12px' }}>
                    {formatUnits(loan.remainingBalance)} XLM
                  </td>
                  <td style={{ padding: '8px 12px', fontFamily: 'monospace', fontSize: 12 }}>
                    {loan.borrower.slice(0, 8)}…
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  )
}
