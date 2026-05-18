<template>
  <div class="page">
    <h2>代码编辑记录</h2>

    <el-card shadow="never" class="table-card">
      <el-table :data="tableData" border stripe v-loading="loading">
        <el-table-column prop="id" label="ID" width="80" />
        <el-table-column prop="session_id" label="Session ID" min-width="200" show-overflow-tooltip />
        <el-table-column prop="file_path_hash" label="文件路径哈希" width="200" show-overflow-tooltip />
        <el-table-column prop="edit_type" label="编辑类型" width="100">
          <template #default="{ row }">
            <el-tag
              :type="editTypeColor(row.edit_type)"
              size="small"
            >
              {{ row.edit_type }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="lines_added" label="新增行" width="80" />
        <el-table-column prop="lines_removed" label="删除行" width="80" />
        <el-table-column prop="created_at" label="创建时间" width="180" />
        <el-table-column label="操作" width="120">
          <template #default>
            <el-button type="primary" link>查看 Diff</el-button>
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
import { fetchCodeEdits } from '../api/edits'
import type { CodeEdit } from '../types'

const loading = ref(false)

const pagination = reactive({
  page: 1,
  pageSize: 10,
  total: 0,
})

const tableData = ref<CodeEdit[]>([])

function editTypeColor(type: string | null): '' | 'success' | 'warning' | 'danger' | 'info' {
  switch (type) {
    case 'create': return 'success'
    case 'modify': return 'warning'
    case 'delete': return 'danger'
    case 'rename': return 'info'
    default: return ''
  }
}

async function loadData() {
  loading.value = true
  try {
    const res = await fetchCodeEdits({
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
