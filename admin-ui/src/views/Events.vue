<template>
  <div class="page">
    <h2>行为事件</h2>

    <el-card shadow="never" class="table-card">
      <el-timeline>
        <el-timeline-item
          v-for="item in tableData"
          :key="item.id"
          :timestamp="item.created_at || ''"
          placement="top"
          :type="eventTimelineType(item.event_type)"
        >
          <el-card shadow="hover">
            <div class="event-item">
              <el-tag
                :type="eventTagType(item.event_type)"
                size="small"
              >
                {{ item.event_type }}
              </el-tag>
              <span class="event-session">Session: {{ item.session_id }}</span>
            </div>
          </el-card>
        </el-timeline-item>
      </el-timeline>

      <el-empty v-if="!loading && tableData.length === 0" description="暂无行为事件数据" />

      <div class="pagination-wrapper" v-if="pagination.total > 0">
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
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import client from '../api/client'
import type { ActionEventItem } from '../types'

const loading = ref(false)

const pagination = reactive({
  page: 1,
  pageSize: 10,
  total: 0,
})

const tableData = ref<ActionEventItem[]>([])

function eventTagType(type: string): '' | 'success' | 'warning' | 'danger' | 'info' {
  switch (type) {
    case 'accept': return 'success'
    case 'reject': return 'danger'
    case 'modify': return 'warning'
    case 'regenerate': return 'info'
    default: return ''
  }
}

function eventTimelineType(type: string): 'primary' | 'success' | 'warning' | 'danger' | 'info' {
  switch (type) {
    case 'accept': return 'success'
    case 'reject': return 'danger'
    case 'modify': return 'warning'
    default: return 'primary'
  }
}

async function loadData() {
  loading.value = true
  try {
    const res = await client.get('/admin/events', {
      params: {
        page: pagination.page,
        page_size: pagination.pageSize,
      },
    })
    if (res.data) {
      tableData.value = res.data.data || []
      pagination.total = res.data.total || 0
    }
  } catch {
    tableData.value = []
    pagination.total = 0
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  loadData()
})
</script>

<style scoped>
.page {
  padding: 0;
}

.page h2 {
  margin: 0 0 20px 0;
  color: #303133;
}

.table-card {
  margin-bottom: 16px;
}

.event-item {
  display: flex;
  align-items: center;
  gap: 12px;
}

.event-session {
  color: #909399;
  font-size: 12px;
}

.pagination-wrapper {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
