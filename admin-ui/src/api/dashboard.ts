import client from './client'
import type { ApiResponse, DashboardStats, PaginatedList, DailyStat } from '../types'

export async function fetchDashboardStats(): Promise<DashboardStats> {
  const res = await client.get<ApiResponse<DashboardStats>>('/admin/dashboard')
  return res.data.data
}

export async function fetchDailyStats(params?: {
  page?: number
  page_size?: number
  agent_id?: string
  date_from?: string
  date_to?: string
}): Promise<PaginatedList<DailyStat>> {
  const res = await client.get<ApiResponse<PaginatedList<DailyStat>>>('/admin/daily-stats', { params })
  return res.data.data
}
