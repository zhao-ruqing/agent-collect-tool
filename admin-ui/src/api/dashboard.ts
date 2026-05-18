import client from './client'
import type { DashboardStats } from '../types'

export function fetchDashboardStats(): Promise<{ data: DashboardStats }> {
  return client.get('/admin/dashboard')
}

export function fetchDailyStats(params?: {
  start_date?: string
  end_date?: string
}): Promise<{ data: { daily_stats: Array<Record<string, unknown>> } }> {
  return client.get('/admin/daily-stats', { params })
}
