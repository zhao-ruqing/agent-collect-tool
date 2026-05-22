<template>
  <div class="page-view">
    <div class="panel">
      <div class="panel-header">
        <h3 class="panel-title">行为事件</h3>
        <span class="panel-badge">共 {{ pagination.total }} 条</span>
      </div>
      <div class="panel-body">
        <!-- 自定义时间线 -->
        <div class="timeline" v-loading="loading">
          <div
            v-for="item in tableData"
            :key="item.id"
            class="tl-item"
          >
            <div class="tl-dot" :class="`tl-dot--${item.event_type}`">
              <div class="tl-dot-inner"></div>
            </div>
            <div class="tl-card">
              <div class="tl-header">
                <el-tag :type="eventTagType(item.event_type)" size="small" effect="dark">
                  {{ eventLabel(item.event_type) }}
                </el-tag>
                <span class="tl-time">{{ item.created_at || '—' }}</span>
              </div>
              <div class="tl-body">
                <span class="text-mono tl-session">{{ item.session_id }}</span>
                <div v-if="item.event_data" class="tl-data">
                  <code>{{ JSON.stringify(item.event_data) }}</code>
                </div>
              </div>
            </div>
          </div>
          <el-empty v-if="!loading && tableData.length === 0" description="暂无行为事件数据" />
        </div>
      </div>
      <div class="panel-footer" v-if="pagination.total > 0">
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
import { ref, reactive, onMounted, watch } from 'vue'
import client from '../api/client'
import { useFilterStore } from '../stores/filter'
import type { ActionEventItem } from '../types'

const filterStore = useFilterStore()
const loading = ref(false)
const pagination = reactive({ page: 1, pageSize: 10, total: 0 })
const tableData = ref<ActionEventItem[]>([])

function eventTagType(type: string): '' | 'success' | 'warning' | 'danger' | 'info' {
  switch (type) { case 'accept': return 'success'; case 'reject': return 'danger'; case 'modify': return 'warning'; case 'regenerate': return 'info'; default: return '' }
}
function eventLabel(type: string): string {
  switch (type) { case 'accept': return '接受'; case 'reject': return '拒绝'; case 'modify': return '修改'; case 'regenerate': return '重新生成'; default: return type }
}

async function loadData() {
  loading.value = true
  try {
    const params: Record<string, unknown> = { page: pagination.page, page_size: pagination.pageSize }
    if (filterStore.toolType) params.tool_type = filterStore.toolType
    const res = await client.get('/admin/events', { params })
    const payload = res.data?.data
    tableData.value = payload?.list || []
    pagination.total = payload?.total || 0
  } catch { tableData.value = []; pagination.total = 0 }
  finally { loading.value = false }
}

watch(() => filterStore.toolType, () => { pagination.page = 1; loadData() })

onMounted(() => loadData())
</script>

<style scoped>
.page-view { animation: fadeInUp 0.45s ease both; }
/* Filter */
.filter-panel { margin-bottom: 18px; }
.panel-body { padding: 16px 20px; }
.filter-row { display: flex; align-items: flex-end; gap: 20px; }
.filter-label { display: block; font-size: 11px; color: var(--c-text-muted); text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 6px; }
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
.panel-body { padding: 20px 24px; }
.panel-footer { display: flex; justify-content: flex-end; padding: 14px 20px; border-top: 1px solid var(--c-border); }
.text-mono { font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace; font-size: 12px; }

/* 自定义时间线 */
.timeline { position: relative; padding-left: 24px; }
.timeline::before {
  content: ''; position: absolute; left: 5px; top: 0; bottom: 0;
  width: 1px; background: var(--c-border);
}
.tl-item { position: relative; margin-bottom: 18px; }
.tl-item:last-child { margin-bottom: 0; }
.tl-dot {
  position: absolute; left: -21px; top: 16px;
  width: 12px; height: 12px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  background: var(--c-border); border: 2px solid var(--c-bg-body);
  z-index: 1;
}
.tl-dot-inner {
  width: 5px; height: 5px; border-radius: 50%; background: var(--c-bg-body);
}
.tl-dot--accept      { background: rgba(52, 211, 153, 0.3); border-color: var(--c-emerald); }
.tl-dot--reject      { background: rgba(251, 113, 133, 0.3); border-color: var(--c-rose); }
.tl-dot--modify      { background: rgba(251, 191, 36, 0.3);  border-color: var(--c-amber); }
.tl-dot--regenerate  { background: rgba(167, 139, 250, 0.3);  border-color: var(--c-purple); }
.tl-card {
  background: var(--c-bg-surface); border: 1px solid var(--c-border);
  border-radius: var(--radius-md); padding: 14px 16px;
  transition: border-color var(--transition-fast);
}
.tl-card:hover { border-color: var(--c-border-active); }
.tl-header { display: flex; align-items: center; gap: 12px; margin-bottom: 8px; }
.tl-time  { font-size: 12px; color: var(--c-text-muted); }
.tl-session { color: var(--c-text-dim); }
.tl-data { margin-top: 8px; }
.tl-data code {
  font-size: 11px; color: var(--c-text-dim); background: rgba(0,0,0,0.2);
  padding: 6px 10px; border-radius: var(--radius-sm);
  display: block; overflow-x: auto; white-space: pre-wrap;
  word-break: break-all;
}
</style>
