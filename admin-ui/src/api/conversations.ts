import client from './client'
import type { ApiResponse, PaginatedList, Session } from '../types'

export async function fetchConversations(params?: {
  page?: number
  page_size?: number
  date_from?: string
  date_to?: string
  keyword?: string
  agent_id?: string
  tool_type?: string
}): Promise<PaginatedList<Session>> {
  const res = await client.get<ApiResponse<PaginatedList<Session>>>('/admin/conversations', { params })
  return res.data.data
}

export async function fetchConversationDetail(
  sessionId: string,
  page = 1,
  pageSize = 20,
): Promise<{
  session: Session
  messages: Array<{
    id: number
    role: string
    content_hash: string | null
    content: string | null
    model: string | null
    tokens_input: number | null
    tokens_output: number | null
    created_at: string | null
  }>
  total: number
  page: number
  page_size: number
}> {
  const res = await client.get<ApiResponse<any>>(
    `/admin/conversations/${sessionId}`,
    { params: { page, page_size: pageSize } },
  )
  return res.data.data
}
