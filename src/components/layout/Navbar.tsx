import { useConnectionStatus } from '../../hooks/useConnectionStatus'

const statusConfig = {
  connected: { label: 'Connected', dot: 'bg-green-500', ring: 'ring-green-400' },
  connecting: { label: 'Connecting', dot: 'bg-yellow-500', ring: 'ring-yellow-400' },
  disconnected: { label: 'Disconnected', dot: 'bg-red-500', ring: 'ring-red-400' },
} as const

export function ConnectionIndicator() {
  const status = useConnectionStatus()
  const cfg = statusConfig[status]

  return (
    <div
      className="flex items-center gap-2 px-3 py-1.5 rounded-full text-sm font-medium ring-1 ring-inset"
      style={{ backgroundColor: 'rgba(0,0,0,0.04)' }}
    >
      <span className={`inline-block w-2 h-2 rounded-full ${cfg.dot} ring-2 ${cfg.ring}`} />
      <span>{cfg.label}</span>
    </div>
  )
}

export function Navbar() {
  return (
    <nav
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        padding: '0 24px',
        height: 56,
        borderBottom: '1px solid #e5e7eb',
        background: '#fff',
      }}
    >
      <div style={{ fontWeight: 700, fontSize: 18 }}>StepFi</div>
      <ConnectionIndicator />
    </nav>
  )
}
