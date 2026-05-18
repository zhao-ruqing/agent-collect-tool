<template>
  <div class="page">
    <h2>客户端管理</h2>

    <el-card shadow="never" class="table-card">
      <el-table :data="tableData" border stripe v-loading="loading">
        <el-table-column prop="id" label="Agent ID" min-width="200" show-overflow-tooltip />
        <el-table-column prop="hostname_hash" label="主机哈希" width="200" show-overflow-tooltip />
        <el-table-column prop="os_info" label="操作系统" width="150" />
        <el-table-column prop="version" label="版本" width="100" />
        <el-table-column label="在线状态" width="100">
          <template #default="{ row }">
            <el-tag :type="isOnline(row.last_seen_at) ? 'success' : 'info'" size="small">
              {{ isOnline(row.last_seen_at) ? '在线' : '离线' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="last_seen_at" label="最后上报" width="180" />
        <el-table-column label="操作" width="120">
          <template #default>
            <el-button type="primary" link>查看详情</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
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
import { fetchAgents } from '../api/agents'
import type { Agent } from '../types'

const loading = ref(false)

const pagination = reactive({
  page: 1,
  pageSize: 10,
  total: 0,
})

const tableData = ref<Agent[]>([])

function isOnline(lastSeen: string | null): boolean {
  if (!lastSeen) return false
  const delta = Date.now() - new Date(lastSeen).getTime()
  return delta < 5 * 60 * 1000 // 5 分钟内视为在线
}

async function loadData() {
  loading.value = true
  try {
    const res = await fetchAgents({
      page: pagination.page,
      page_size: pagination.pageSize,
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

.pagination-wrapper {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
