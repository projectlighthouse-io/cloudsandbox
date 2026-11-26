// The props contract between the Rust handlers (src/slipway/handlers.rs) and the pages.

export interface User { id: number; name: string; email: string; avatar: string | null }
export interface Team { id: number; name: string }

export interface Shared {
  app_name: string
  auth: { user: User }
  team: Team
  teams: Team[]
  flash: { success: string | null; error: string | null }
  csrf_token: string | null
  errors: Record<string, string>
  [key: string]: unknown
}

export type ServerStatus = 'healthy' | 'degraded' | 'provisioning' | 'offline'
export type DeploymentStatus = 'queued' | 'running' | 'finished' | 'failed' | 'cancelled'
export type Tone = 'ok' | 'warn' | 'bad' | 'info' | 'idle'

export interface Server {
  id: number; name: string; provider: string; region: string; ip: string
  status: ServerStatus; cpu: number | null; memory: number | null; disk: number
  vcpus: number; memoryGb: number; sitesCount: number
}

export interface Site {
  id: number; serverId: number; domain: string; repository: string; branch: string
  runtime: string; status: 'deployed' | 'deploying' | 'failed' | 'suspended'; lastDeployedAt: number | null
}

export interface Deployment {
  id: number; siteId: number; site: string; commit: string; message: string; author: string
  branch: string; status: DeploymentStatus; startedAt: number; durationMs: number | null
}

export interface Tile {
  label: string
  value: number
  delta: { text: string; good: boolean }
  series: number[]
  color: 'sky' | 'indigo' | 'red'
  note: { text: string; tone: Tone; at: number | null }
}

export interface Usage { serverId: number; name: string; cpu: number | null; memory: number | null; trend: number[] }

export interface Paginated<T> { data: T[]; meta: { page: number; perPage: number; total: number } }

export interface Activity { id: number; at: number; actor: string; action: string; target: string }
export interface Day { date: string; label: string; entries: Activity[] }
