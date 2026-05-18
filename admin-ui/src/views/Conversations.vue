<template>
  <div class="page">
    <h2>对话记录</h2>

    <!-- 筛选栏 -->
    <el-card shadow="never" class="filter-card">
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
    </el-card>

    <!-- 数据表格 -->
    <el-card shadow="never" class="table-card">
      <el-table :data="tableData" border stripe v-loading="loading">
        <el-table-column prop="id" label="Session ID" min-width="200" show-overflow-tooltip />
        <el-table-column prop="agent_id" label="客户端" width="200" show-overflow-tooltip />
        <el-table-column prop="started_at" label="开始时间" width="180" />
        <el-table-column prop="ended_at" label="结束时间" width="180" />
        <el-table-column label="操作" width="120">
          <template #default>
            <el-button type="primary" link>详情</el-button>
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
          @size-change="handleSearch"
          @current-change="handleSearch"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { fetchConversations } from '../api/conversations'
import type { Session } from '../types'

const loading = ref(false)

const filters = reactive({
  dateRange: null as [string, string] | null,
  tool: '',
  model: '',
})

const pagination = reactive({
  page: 1,
  pageSize: 10,
  total: 0,
})

const tableData = ref<Session[]>([])

async function handleSearch() {
  loading.value = true
  try {
    const params: Record<string, unknown> = {
      page: pagination.page,
      page_size: pagination.pageSize,
    }
    if (filters.dateRange) {
      params.start_date = filters.dateRange[0]
      params.end_date = filters.dateRange[1]
    }
    if (filters.tool) params.tool = filters.tool
    if (filters.model) params.model = filters.model

    const res = await fetchConversations(params as never)
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

function handleReset() {
  filters.dateRange = null
  filters.tool = ''
  filters.model = ''
  pagination.page = 1
  handleSearch()
}

onMounted(() => {
  handleSearch()
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

.filter-card {
  margin-bottom: 16px;
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
