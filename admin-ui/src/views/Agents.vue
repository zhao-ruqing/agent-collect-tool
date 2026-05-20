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
          <template #default="{ row }">{{ row.last_seen_at || '—' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="100" align="center">
          <template #default>
            <el-button type="primary" link>查看详情</el-button>
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
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { fetchAgents } from '../api/agents'
import type { Agent } from '../types'

const loading = ref(false)
const pagination = reactive({ page: 1, pageSize: 10, total: 0 })
const tableData = ref<Agent[]>([])

function isOnline(lastSeen: string | null): boolean {
  if (!lastSeen) return false
  return Date.now() - new Date(lastSeen).getTime() < 5 * 60 * 1000
}

async function loadData() {
  loading.value = true
  try {
    const res = await fetchAgents({ page: pagination.page, page_size: pagination.pageSize })
    if (res.data) { tableData.value = res.data.data || []; pagination.total = res.data.total || 0 }
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
</style>
