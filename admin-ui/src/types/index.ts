// ============================================================
// 数据模型类型定义（与后端模型对应）
// ============================================================

export interface Agent {
  id: string
  hostname_hash: string
  os_info: string | null
  version: string | null
  last_seen_at: string | null
  created_at: string | null
}

export interface Session {
  id: string
  agent_id: string
  project_path_hash: string | null
  git_branch: string | null
  started_at: string
  ended_at: string | null
}

export interface Message {
  id: number
  session_id: string
  role: 'user' | 'assistant'
  content_hash: string | null
  model: string | null
  tokens_input: number | null
  tokens_output: number | null
  created_at: string | null
}

export interface CodeEdit {
  id: number
  session_id: string
  file_path_hash: string | null
  edit_type: string | null
  lines_added: number | null
  lines_removed: number | null
  diff_skeleton: string | null
  created_at: string | null
}

export interface ActionEventItem {
  id: number
  session_id: string
  event_type: string
  event_data: Record<string, unknown> | null
  created_at: string | null
}

export interface DailyStat {
  id: number
  agent_id: string
  stat_date: string
  total_sessions: number
  total_messages: number
  total_tokens: number
  total_edits: number
}

// ============================================================
// API 响应类型
// ============================================================

export interface DashboardStats {
  total_agents: number
  total_sessions: number
  total_messages?: number
  total_tokens?: number
  acceptance_rate?: number
  daily_stats?: DailyStat[]
}

export interface PaginatedResponse<T> {
  data: T[]
  total: number
  page: number
  page_size: number
}

export interface ApiResponse<T> {
  status: string
  data?: T
  error?: string
}
