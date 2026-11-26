// The props contract between the Rust handlers (src/slipway/handlers.rs) and the pages.

export type Status = 'active' | 'provisioning' | 'stopped' | 'failed' | 'deployed' | 'deploying' | 'suspended' | 'queued' | 'running' | 'finished' | 'cancelled'

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

export interface Server {
  id: number; name: string; provider: string; region: string; ip: string
  status: Status; cpu: number; memory: number; disk: number; sitesCount: number
}

export interface Site {
  id: number; serverId: number; domain: string; repository: string; branch: string
  runtime: string; status: Status; lastDeployedAt: number | null
}

export interface Deployment {
  id: number; siteId: number; site: string; commit: string; message: string; author: string
  branch: string; status: Status; startedAt: number; durationMs: number | null
}

export interface Paginated<T> { data: T[]; meta: { page: number; perPage: number; total: number } }

export interface Activity { id: number; at: number; actor: string; action: string; target: string }
export interface Day { date: string; label: string; entries: Activity[] }
