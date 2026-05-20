<template>
  <div class="page-view" v-animate>
    <!-- 筛选栏 -->
    <div class="panel filter-panel">
      <div class="panel-header">
        <h3 class="panel-title">筛选条件</h3>
      </div>
      <div class="panel-body">
        <el-form :inline="true" :model="filters" size="default">
          <el-form-item label="时间范围">
            <el-date-picker
              v-model="filters.dateRange"
              type="daterange"
              range-separator="至"
              start-placeholder="开始日期"
              end-placeholder="结束日期"
              format="YYYY-MM-DD"
              value-format="YYYY-MM-DD"
            />
          </el-form-item>
          <el-form-item label="工具">
            <el-select v-model="filters.tool" placeholder="全部" clearable>
              <el-option label="Claude Code" value="claude-code" />
              <el-option label="Trae" value="trae" />
            </el-select>
          </el-form-item>
          <el-form-item label="模型">
            <el-input v-model="filters.model" placeholder="模型名称" clearable />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="handleSearch">查询</el-button>
            <el-button @click="handleReset">重置</el-button>
          </el-form-item>
        </el-form>
      </div>
    </div>

    <!-- 数据表格 -->
    <div class="panel">
      <div class="panel-header">
        <h3 class="panel-title">对话记录</h3>
        <span class="panel-badge">共 {{ pagination.total }} 条</span>
      </div>
      <el-table :data="tableData" border stripe v-loading="loading">
        <el-table-column label="Session ID" min-width="220">
          <template #default="{ row }">
            <span class="text-mono">{{ row.id }}</span>
          </template>
        </el-table-column>
        <el-table-column label="客户端" width="200">
          <template #default="{ row }">
            <span class="text-mono">{{ row.agent_id }}</span>
          </template>
        </el-table-column>
        <el-table-column label="开始时间" width="180">
          <template #default="{ row }">{{ row.started_at || '—' }}</template>
        </el-table-column>
        <el-table-column label="结束时间" width="180">
          <template #default="{ row }">{{ row.ended_at || '—' }}</template>
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
import { ref, reactive, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { fetchConversations } from '../api/conversations'
import type { Session } from '../types'

const router = useRouter()
const loading = ref(false)
const filters = reactive({ dateRange: null as [string, string] | null, tool: '', model: '' })
const pagination = reactive({ page: 1, pageSize: 10, total: 0 })
const tableData = ref<Session[]>([])

async function handleSearch() {
  loading.value = true
  try {
    const params: Record<string, unknown> = { page: pagination.page, page_size: pagination.pageSize }
    if (filters.dateRange) { params.date_from = filters.dateRange[0]; params.date_to = filters.dateRange[1] }
    if (filters.tool) params.tool = filters.tool
    if (filters.model) params.model = filters.model
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
  filters.dateRange = null; filters.tool = ''; filters.model = ''; pagination.page = 1
  handleSearch()
}

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
.text-mono { font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace; font-size: 12px; }
</style>
