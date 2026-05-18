import client from './client'
import type { PaginatedResponse, CodeEdit } from '../types'

export function fetchCodeEdits(params?: {
  page?: number
  page_size?: number
  session_id?: string
}): Promise<{ data: PaginatedResponse<CodeEdit> }> {
  return client.get('/admin/edits', { params })
}
