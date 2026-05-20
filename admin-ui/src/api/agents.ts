import client from './client'
import type { ApiResponse, PaginatedList, Agent, Session } from '../types'

export async function fetchAgents(params?: {
  page?: number
  page_size?: number
}): Promise<PaginatedList<Agent>> {
  const res = await client.get<ApiResponse<PaginatedList<Agent>>>('/admin/agents', { params })
  return res.data.data
}

export async function fetchAgentDetail(agentId: string): Promise<{
  agent: Agent
  recent_sessions: Session[]
  stats: { total_sessions: number; total_messages: number; total_edits: number }
}> {
  const res = await client.get<ApiResponse<any>>(`/admin/agents/${agentId}`)
  return res.data.data
}
