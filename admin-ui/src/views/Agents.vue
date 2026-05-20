<template>
  <div class="page-view">
    <div class="panel">
      <div class="panel-header">
        <h3 class="panel-title">客户端管理</h3>
        <span class="panel-badge">共 {{ pagination.total }} 台</span>
      </div>
      <el-table :data="tableData" border stripe v-loading="loading">
        <el-table-column label="Agent ID" min-width="240">
          <template #default="{ row }">
            <span class="text-mono">{{ row.id }}</span>
          </template>
        </el-table-column>
        <el-table-column label="主机哈希" width="200">
          <template #default="{ row }">
            <span class="text-mono">{{ row.hostname_hash }}</span>
          </template>
        </el-table-column>
        <el-table-column label="操作系统" width="150">
          <template #default="{ row }">
            <span v-if="row.os_info">{{ row.os_info }}</span>
            <span v-else class="text-dim">—</span>
          </template>
        </el-table-column>
        <el-table-column label="版本" width="110">
          <template #default="{ row }">
            <span v-if="row.version" class="text-mono">{{ row.version }}</span>
            <span v-else class="text-dim">—</span>
          </template>
        </el-table-column>
        <el-table-column label="在线状态" width="100" align="center">
          <template #default="{ row }">
            <div class="online-indicator">
              <span class="online-dot" :class="{ online: isOnline(row.last_seen_at) }"></span>
              <span>{{ isOnline(row.last_seen_at) ? '在线' : '离线' }}</span>
            </div>
          </template>
        </el-table-column>
        <el-table-column label="最后上报" width="180">
          <template #default="{ row }">{{ formatTime(row.last_seen_at) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="100" align="center">
          <template #default="{ row }">
            <el-button type="primary" link @click="showDetail(row.id)">查看详情</el-button>
          </template>
        </el-table-column>
      </el-table>
      <div class="panel-footer">
        <el-pagination
          v-model:current-page="pagination.page"
          v-model:page-size="pagination.pageSize"
          :total="pagination.total"
          :page-sizes="[10, 20, 50]"
          layout="total, sizes, prev, pager, next"
          @size-change="loadData"
          @current-change="loadData"
        />
      </div>
    </div>

    <!-- Agent 详情弹窗 -->
    <el-dialog v-model="detailVisible" title="客户端详情" width="700px" destroy-on-close>
      <template v-if="detail">
        <!-- 基本信息 -->
        <div class="detail-section">
          <h4 class="detail-title">基本信息</h4>
          <div class="detail-grid">
            <div class="detail-item">
              <span class="detail-label">Agent ID</span>
              <span class="detail-value text-mono">{{ detail.agent.id }}</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">主机哈希</span>
              <span class="detail-value text-mono">{{ detail.agent.hostname_hash }}</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">操作系统</span>
              <span class="detail-value">{{ detail.agent.os_info || '—' }}</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">版本</span>
              <span class="detail-value text-mono">{{ detail.agent.version || '—' }}</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">最后上报</span>
              <span class="detail-value">{{ formatTime(detail.agent.last_seen_at) }}</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">注册时间</span>
              <span class="detail-value">{{ formatTime(detail.agent.created_at) }}</span>
            </div>
          </div>
        </div>

        <!-- 统计 -->
        <div class="detail-section">
          <h4 class="detail-title">采集统计</h4>
          <div class="stat-mini-grid">
            <div class="stat-mini">
              <span class="stat-mini-value">{{ formatNumber(detail.stats.total_sessions) }}</span>
              <span class="stat-mini-label">总会话</span>
            </div>
            <div class="stat-mini">
              <span class="stat-mini-value">{{ formatNumber(detail.stats.total_messages) }}</span>
              <span class="stat-mini-label">总消息</span>
            </div>
            <div class="stat-mini">
              <span class="stat-mini-value">{{ formatNumber(detail.stats.total_edits) }}</span>
              <span class="stat-mini-label">总编辑</span>
            </div>
          </div>
        </div>

        <!-- 最近会话 -->
        <div class="detail-section" v-if="detail.recent_sessions.length">
          <h4 class="detail-title">最近会话</h4>
          <el-table :data="detail.recent_sessions" size="small" stripe>
            <el-table-column label="Session ID" min-width="200">
              <template #default="{ row }">
                <span class="text-mono">{{ row.id.substring(0, 20) }}...</span>
              </template>
            </el-table-column>
            <el-table-column label="分支" width="140">
              <template #default="{ row }">{{ row.git_branch || '—' }}</template>
            </el-table-column>
            <el-table-column label="开始时间" width="170">
              <template #default="{ row }">{{ formatTime(row.started_at) }}</template>
            </el-table-column>
            <el-table-column label="状态" width="90">
              <template #default="{ row }">
                <el-tag :type="row.ended_at ? 'success' : 'warning'" size="small" effect="dark">
                  {{ row.ended_at ? '结束' : '进行' }}
                </el-tag>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </template>
      <template #footer>
        <el-button @click="detailVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { fetchAgents, fetchAgentDetail } from '../api/agents'
import type { Agent, Session } from '../types'

const loading = ref(false)
const pagination = reactive({ page: 1, pageSize: 10, total: 0 })
const tableData = ref<Agent[]>([])

const detailVisible = ref(false)
const detailLoading = ref(false)
const detail = ref<{
  agent: Agent
  recent_sessions: Session[]
  stats: { total_sessions: number; total_messages: number; total_edits: number }
} | null>(null)

function isOnline(lastSeen: string | null): boolean {
  if (!lastSeen) return false
  return Date.now() - new Date(lastSeen).getTime() < 5 * 60 * 1000
}

function formatNumber(n: number): string {
  if (n >= 10000) return (n / 1000).toFixed(1) + 'k'
  return n.toLocaleString()
}

function formatTime(iso: string | null): string {
  if (!iso) return '—'
  return new Date(iso).toLocaleString('zh-CN', { hour12: false })
}

async function showDetail(agentId: string) {
  detailVisible.value = true
  detailLoading.value = true
  detail.value = null
  try {
    detail.value = await fetchAgentDetail(agentId)
  } catch { /* 失败保持为 null */ }
  finally { detailLoading.value = false }
}

async function loadData() {
  loading.value = true
  try {
    const res = await fetchAgents({ page: pagination.page, page_size: pagination.pageSize })
    tableData.value = res.list || []
    pagination.total = res.total || 0
  } catch { tableData.value = []; pagination.total = 0 }
  finally { loading.value = false }
}

onMounted(() => loadData())
</script>

<style scoped>
.page-view { animation: fadeInUp 0.45s ease both; }
.panel {
  background: var(--c-bg-card); border: 1px solid var(--c-border);
  border-radius: var(--radius-lg); backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px); margin-bottom: 18px;
}
.panel-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 14px 20px; border-bottom: 1px solid var(--c-border);
}
.panel-title { font-size: 14px; font-weight: 600; color: var(--c-text); margin: 0; }
.panel-badge {
  font-size: 11px; color: var(--c-text-muted); background: var(--c-bg-surface);
  padding: 3px 10px; border-radius: 20px; border: 1px solid var(--c-border);
}
.panel-footer { display: flex; justify-content: flex-end; padding: 14px 20px; border-top: 1px solid var(--c-border); }
.text-mono { font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace; font-size: 12px; }
.text-dim { color: var(--c-text-muted); }
.online-indicator { display: flex; align-items: center; gap: 6px; justify-content: center; font-size: 13px; }
.online-dot {
  width: 7px; height: 7px; border-radius: 50%; background: rgba(100, 116, 139, 0.4);
  transition: background var(--transition-base);
}
.online-dot.online { background: var(--c-emerald); box-shadow: 0 0 6px rgba(52, 211, 153, 0.5); }

/* Detail dialog */
.detail-section { margin-bottom: 20px; }
.detail-section:last-child { margin-bottom: 0; }
.detail-title {
  font-size: 13px; font-weight: 600; color: var(--c-text);
  margin: 0 0 12px 0; padding-bottom: 8px; border-bottom: 1px solid var(--c-border);
}
.detail-grid { display: grid; grid-template-columns: repeat(2, 1fr); gap: 12px; }
.detail-item { display: flex; flex-direction: column; gap: 3px; }
.detail-label { font-size: 11px; color: var(--c-text-muted); text-transform: uppercase; letter-spacing: 0.3px; }
.detail-value { font-size: 13px; color: var(--c-text); word-break: break-all; }

.stat-mini-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; }
.stat-mini {
  text-align: center; padding: 14px 10px;
  background: var(--c-bg-surface); border: 1px solid var(--c-border);
  border-radius: var(--radius-md);
}
.stat-mini-value { display: block; font-size: 22px; font-weight: 700; color: var(--c-text); }
.stat-mini-label { font-size: 11px; color: var(--c-text-muted); margin-top: 4px; }
</style>
