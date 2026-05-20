<template>
  <div class="page-view">
    <div class="panel">
      <div class="panel-header">
        <h3 class="panel-title">代码编辑记录</h3>
        <span class="panel-badge">共 {{ pagination.total }} 条</span>
      </div>
      <el-table :data="tableData" border stripe v-loading="loading">
        <el-table-column label="ID" width="80" align="center">
          <template #default="{ row }">
            <span class="text-mono">{{ row.id }}</span>
          </template>
        </el-table-column>
        <el-table-column label="Session ID" min-width="220">
          <template #default="{ row }">
            <span class="text-mono">{{ row.session_id }}</span>
          </template>
        </el-table-column>
        <el-table-column label="文件哈希" width="200">
          <template #default="{ row }">
            <span class="text-mono">{{ (row.file_path_hash || '—').substring(0, 12) }}...</span>
          </template>
        </el-table-column>
        <el-table-column label="编辑类型" width="110">
          <template #default="{ row }">
            <el-tag :type="editTagType(row.edit_type)" size="small" effect="dark">
              {{ editLabel(row.edit_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="+行" width="70" align="center">
          <template #default="{ row }">
            <span class="num-add">{{ row.lines_added ?? 0 }}</span>
          </template>
        </el-table-column>
        <el-table-column label="-行" width="70" align="center">
          <template #default="{ row }">
            <span class="num-rm">{{ row.lines_removed ?? 0 }}</span>
          </template>
        </el-table-column>
        <el-table-column label="时间" width="180">
          <template #default="{ row }">{{ row.created_at || '—' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="100" align="center">
          <template #default>
            <el-button type="primary" link>查看 Diff</el-button>
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
import { fetchCodeEdits } from '../api/edits'
import type { CodeEdit } from '../types'

const loading = ref(false)
const pagination = reactive({ page: 1, pageSize: 10, total: 0 })
const tableData = ref<CodeEdit[]>([])

function editTagType(type: string | null): '' | 'success' | 'warning' | 'danger' | 'info' {
  switch (type) { case 'create': return 'success'; case 'modify': return 'warning'; case 'delete': return 'danger'; case 'rename': return 'info'; default: return '' }
}
function editLabel(type: string | null): string {
  switch (type) { case 'create': return '新建'; case 'modify': return '修改'; case 'delete': return '删除'; case 'rename': return '重命名'; default: return type || '—' }
}

async function loadData() {
  loading.value = true
  try {
    const res = await fetchCodeEdits({ page: pagination.page, page_size: pagination.pageSize })
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
.num-add { color: var(--c-emerald); font-weight: 600; font-family: 'JetBrains Mono', monospace; }
.num-rm  { color: var(--c-rose); font-weight: 600; font-family: 'JetBrains Mono', monospace; }
</style>
