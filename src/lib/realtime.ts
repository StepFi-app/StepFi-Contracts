export type ConnectionStatus = 'connected' | 'connecting' | 'disconnected'

export type Listener<T> = (data: T) => void

export type StatusListener = (status: ConnectionStatus) => void

export class RealtimeChannel<T> {
  private pollInterval: number
  private timer: ReturnType<typeof setInterval> | null = null
  private fetcher: () => Promise<T>
  private listeners: Set<Listener<T>> = new Set()
  private statusListeners: Set<StatusListener> = new Set()
  private _connectionStatus: ConnectionStatus = 'disconnected'
  private lastData: T | null = null

  constructor(fetcher: () => Promise<T>, pollIntervalMs = 10_000) {
    this.fetcher = fetcher
    this.pollInterval = pollIntervalMs
  }

  get connectionStatus(): ConnectionStatus {
    return this._connectionStatus
  }

  get data(): T | null {
    return this.lastData
  }

  subscribe(listener: Listener<T>): () => void {
    this.listeners.add(listener)
    return () => {
      this.listeners.delete(listener)
    }
  }

  onStatusChange(listener: StatusListener): () => void {
    this.statusListeners.add(listener)
    return () => {
      this.statusListeners.delete(listener)
    }
  }

  private setConnectionStatus(status: ConnectionStatus) {
    if (this._connectionStatus === status) return
    this._connectionStatus = status
    this.statusListeners.forEach((fn) => fn(status))
  }

  private async poll() {
    this.setConnectionStatus('connecting')
    try {
      const data = await this.fetcher()
      this.lastData = data
      this.setConnectionStatus('connected')
      this.listeners.forEach((fn) => fn(data))
    } catch {
      this.setConnectionStatus('disconnected')
    }
  }

  start() {
    if (this.timer) return
    this.poll()
    this.timer = setInterval(() => this.poll(), this.pollInterval)
  }

  stop() {
    if (this.timer) {
      clearInterval(this.timer)
      this.timer = null
    }
    this.setConnectionStatus('disconnected')
  }
}
