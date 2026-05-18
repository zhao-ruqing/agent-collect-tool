import client from './client'
import type { PaginatedResponse, Session } from '../types'

export function fetchConversations(params?: {
  page?: number
  page_size?: number
  start_date?: string
  end_date?: string
  tool?: string
  model?: string
  agent_id?: string
}): Promise<{ data: PaginatedResponse<Session> }> {
  return client.get('/admin/conversations', { params })
}
