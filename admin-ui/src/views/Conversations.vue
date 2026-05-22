<template>
  <div class="page-view" v-animate>
    <!-- 筛选栏 -->
    <div class="panel filter-panel">
      <div class="panel-header">
        <h3 class="panel-title">筛选条件</h3>
      </div>
      <div class="panel-body">
        <div class="filter-row">
          <div class="filter-item filter-item--keyword">
            <label class="filter-label">搜索</label>
            <el-input
              v-model="filters.keyword"
              placeholder="搜索 Session ID 或分支名..."
              clearable
              @keyup.enter="handleSearch"
            >
              <template #prefix>
                <el-icon><Search /></el-icon>
              </template>
            </el-input>
          </div>
          <div class="filter-item filter-item--date">
            <label class="filter-label">时间范围</label>
            <el-date-picker
              v-model="filters.dateRange"
              type="daterange"
              range-separator="至"
              start-placeholder="开始"
              end-placeholder="结束"
              format="YYYY-MM-DD"
              value-format="YYYY-MM-DD"
            />
          </div>
          <div class="filter-item filter-item--actions">
            <label class="filter-label">&nbsp;</label>
            <div class="filter-btns">
              <el-button type="primary" @click="handleSearch">
                <el-icon><Search /></el-icon>
                查询
              </el-button>
              <el-button @click="handleReset">重置</el-button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 数据表格 -->
    <div class="panel">
      <div class="panel-header">
        <h3 class="panel-title">对话记录</h3>
        <span class="panel-badge">共 {{ pagination.total }} 条</span>
      </div>
      <el-table :data="tableData" border stripe v-loading="loading">
        <el-table-column label="Session ID" min-width="240">
          <template #default="{ row }">
            <span class="text-mono">{{ row.id }}</span>
          </template>
        </el-table-column>
        <el-table-column label="AI 工具" width="110">
          <template #default="{ row }">
            <el-tag :type="tagType(row.tool_type)" size="small" effect="dark">
              {{ toolLabel(row.tool_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="Git 分支" min-width="160">
          <template #default="{ row }">
            <span class="text-mono">{{ row.git_branch || '—' }}</span>
          </template>
        </el-table-column>
        <el-table-column label="客户端" width="140">
          <template #default="{ row }">
            <span class="text-mono">{{ row.agent_id }}</span>
          </template>
        </el-table-column>
        <el-table-column label="开始时间" width="170">
          <template #default="{ row }">{{ formatTime(row.started_at) }}</template>
        </el-table-column>
        <el-table-column label="结束时间" width="170">
          <template #default="{ row }">{{ formatTime(row.ended_at) }}</template>
        </el-table-column>
        <el-table-column label="操作" width="100" align="center">
          <template #default="{ row }">
            <el-button type="primary" link @click="goDetail(row.id)">详情</el-button>
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
          @size-change="handleSearch"
          @current-change="handleSearch"
        />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { Search } from '@element-plus/icons-vue'
import { fetchConversations } from '../api/conversations'
import { useFilterStore } from '../stores/filter'
import type { Session } from '../types'

const router = useRouter()
const filterStore = useFilterStore()
const loading = ref(false)
const filters = reactive({ dateRange: null as [string, string] | null, keyword: '' })
const pagination = reactive({ page: 1, pageSize: 10, total: 0 })
const tableData = ref<Session[]>([])

function formatTime(iso: string | null | undefined): string {
  if (!iso) return '—'
  return new Date(iso).toLocaleString('zh-CN', { hour12: false })
}

function toolLabel(t: string | null | undefined): string {
  if (t === 'claude-code') return 'Claude'
  if (t === 'cursor') return 'Cursor'
  if (t === 'trae') return 'Trae'
  return t || '—'
}

function tagType(t: string | null | undefined): string {
  if (t === 'claude-code') return ''
  if (t === 'cursor') return 'success'
  if (t === 'trae') return 'warning'
  return ''
}

async function handleSearch() {
  loading.value = true
  try {
    const params: Record<string, unknown> = { page: pagination.page, page_size: pagination.pageSize }
    if (filters.dateRange) { params.date_from = filters.dateRange[0]; params.date_to = filters.dateRange[1] }
    if (filters.keyword) params.keyword = filters.keyword
    if (filterStore.toolType) params.tool_type = filterStore.toolType
    const res = await fetchConversations(params as any)
    tableData.value = res.list || []
    pagination.total = res.total || 0
  } catch { tableData.value = []; pagination.total = 0 }
  finally { loading.value = false }
}

function goDetail(sessionId: string) {
  router.push({ name: 'conversation-detail', params: { sessionId } })
}

function handleReset() {
  filters.dateRange = null; filters.keyword = ''; pagination.page = 1
  handleSearch()
}

watch(() => filterStore.toolType, () => { pagination.page = 1; handleSearch() })

onMounted(() => handleSearch())
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
.panel-body { padding: 16px 20px; }
.panel-footer { display: flex; justify-content: flex-end; padding: 14px 20px; border-top: 1px solid var(--c-border); }

/* Filter layout */
.filter-row {
  display: flex; align-items: flex-end; gap: 20px; flex-wrap: wrap;
}
.filter-label {
  display: block; font-size: 11px; color: var(--c-text-muted);
  text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px;
}
.filter-item--keyword { flex: 1; min-width: 220px; }
.filter-item--date { flex-shrink: 0; }
.filter-item--actions { flex-shrink: 0; }
.filter-btns { display: flex; gap: 8px; }

.text-mono { font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace; font-size: 12px; }

@media (max-width: 768px) {
  .filter-row { flex-direction: column; align-items: stretch; }
  .filter-item--keyword { min-width: 0; }
}
</style>
