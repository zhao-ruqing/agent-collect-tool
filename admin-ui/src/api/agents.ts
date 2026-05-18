import client from './client'
import type { PaginatedResponse, Agent } from '../types'

export function fetchAgents(params?: {
  page?: number
  page_size?: number
}): Promise<{ data: PaginatedResponse<Agent> }> {
  return client.get('/admin/agents', { params })
}
