<template>
  <div class="dashboard">
    <h2>仪表盘</h2>

    <!-- 统计卡片 -->
    <el-row :gutter="20" class="stat-row">
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon" style="background-color: #409eff">
              <el-icon><Monitor /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">活跃客户端</div>
              <div class="stat-value">{{ stats.total_agents }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon" style="background-color: #67c23a">
              <el-icon><ChatDotSquare /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">今日对话</div>
              <div class="stat-value">{{ stats.total_sessions }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon" style="background-color: #e6a23c">
              <el-icon><Coin /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">Token 消耗</div>
              <div class="stat-value">{{ formatNumber(stats.total_tokens || 0) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon" style="background-color: #f56c6c">
              <el-icon><TrendCharts /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">代码接受率</div>
              <div class="stat-value">{{ (stats.acceptance_rate || 0).toFixed(1) }}%</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 图表区 -->
    <el-row :gutter="20" class="chart-row">
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <span>每日对话趋势（近7天）</span>
          </template>
          <div class="chart-placeholder">
            <p>图表区域 — 待接入 ECharts</p>
          </div>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <span>模型使用占比</span>
          </template>
          <div class="chart-placeholder">
            <p>图表区域 — 待接入 ECharts</p>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Monitor, ChatDotSquare, Coin, TrendCharts } from '@element-plus/icons-vue'
import { fetchDashboardStats } from '../api/dashboard'

const stats = ref({
  total_agents: 0,
  total_sessions: 0,
  total_tokens: 0,
  acceptance_rate: 0,
})

function formatNumber(n: number): string {
  if (n >= 10000) {
    return (n / 1000).toFixed(1) + 'k'
  }
  return n.toLocaleString()
}

onMounted(async () => {
  try {
    const res = await fetchDashboardStats()
    if (res.data) {
      stats.value = { ...stats.value, ...res.data }
    }
  } catch {
    // 后端未就绪时使用默认值
  }
})
</script>

<style scoped>
.dashboard {
  padding: 0;
}

.dashboard h2 {
  margin: 0 0 20px 0;
  color: #303133;
}

.stat-row {
  margin-bottom: 20px;
}

.stat-card {
  cursor: pointer;
}

.stat-content {
  display: flex;
  align-items: center;
  gap: 16px;
}

.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 24px;
}

.stat-info {
  flex: 1;
}

.stat-label {
  font-size: 13px;
  color: #909399;
  margin-bottom: 4px;
}

.stat-value {
  font-size: 24px;
  font-weight: bold;
  color: #303133;
}

.chart-row {
  margin-bottom: 20px;
}

.chart-placeholder {
  height: 300px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #909399;
  background: #fafafa;
  border: 1px dashed #dcdfe6;
  border-radius: 4px;
}
</style>
