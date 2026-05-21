import client from './client'
import type { ApiResponse, PaginatedList, CodeEdit } from '../types'

export async function fetchCodeEdits(params?: {
  page?: number
  page_size?: number
  session_id?: string
  tool_type?: string
}): Promise<PaginatedList<CodeEdit>> {
  const res = await client.get<ApiResponse<PaginatedList<CodeEdit>>>('/admin/edits', { params })
  return res.data.data
}
