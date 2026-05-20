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
              <div class="stat-value">{{ stats.today_sessions }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon" style="background-color: #e6a23c">
              <el-icon><Message /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">累计消息</div>
              <div class="stat-value">{{ formatNumber(stats.total_messages) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon" style="background-color: #f56c6c">
              <el-icon><EditPen /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">代码编辑次数</div>
              <div class="stat-value">{{ formatNumber(stats.total_edits) }}</div>
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
          <div ref="trendChartRef" class="chart-container"></div>
        </el-card>
      </el-col>
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <span>数据总览</span>
          </template>
          <div ref="overviewChartRef" class="chart-container"></div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 最近会话 -->
    <el-row :gutter="20" class="table-row">
      <el-col :span="24">
        <el-card shadow="hover">
          <template #header>
            <span>最近会话</span>
          </template>
          <el-table :data="stats.recent_sessions" stripe style="width: 100%">
            <el-table-column prop="id" label="会话 ID" min-width="200">
              <template #default="{ row }">
                <el-tooltip :content="row.id" placement="top">
                  <span class="text-ellipsis">{{ row.id.substring(0, 16) }}...</span>
                </el-tooltip>
              </template>
            </el-table-column>
            <el-table-column prop="agent_id" label="Agent" width="200">
              <template #default="{ row }">
                <span class="text-ellipsis">{{ row.agent_id.substring(0, 12) }}...</span>
              </template>
            </el-table-column>
            <el-table-column prop="git_branch" label="分支" width="120">
              <template #default="{ row }">
                {{ row.git_branch || '-' }}
              </template>
            </el-table-column>
            <el-table-column label="开始时间" width="180">
              <template #default="{ row }">
                {{ formatTime(row.started_at) }}
              </template>
            </el-table-column>
            <el-table-column label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="row.ended_at ? 'success' : 'warning'" size="small">
                  {{ row.ended_at ? '已结束' : '进行中' }}
                </el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import { Monitor, ChatDotSquare, Message, EditPen } from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import { fetchDashboardStats } from '../api/dashboard'
import type { DashboardStats } from '../types'

const stats = ref<DashboardStats>({
  total_agents: 0,
  total_sessions: 0,
  today_sessions: 0,
  total_messages: 0,
  total_edits: 0,
  recent_sessions: [],
  daily_trend: [],
})

const trendChartRef = ref<HTMLElement | null>(null)
const overviewChartRef = ref<HTMLElement | null>(null)
let trendChart: echarts.ECharts | null = null
let overviewChart: echarts.ECharts | null = null

function formatNumber(n: number): string {
  if (n >= 10000) {
    return (n / 1000).toFixed(1) + 'k'
  }
  return n.toLocaleString()
}

function formatTime(iso: string): string {
  if (!iso) return '-'
  const d = new Date(iso)
  return d.toLocaleString('zh-CN', { hour12: false })
}

function initTrendChart() {
  if (!trendChartRef.value) return
  if (trendChart) trendChart.dispose()

  trendChart = echarts.init(trendChartRef.value)
  const dates = stats.value.daily_trend.map((d) => d.date)
  const counts = stats.value.daily_trend.map((d) => d.count)

  trendChart.setOption({
    tooltip: { trigger: 'axis' },
    grid: { left: '3%', right: '4%', bottom: '3%', containLabel: true },
    xAxis: {
      type: 'category',
      data: dates.length > 0 ? dates : ['暂无数据'],
      boundaryGap: false,
    },
    yAxis: { type: 'value', minInterval: 1 },
    series: [
      {
        name: '对话数',
        type: 'line',
        smooth: true,
        data: counts.length > 0 ? counts : [0],
        areaStyle: { opacity: 0.15 },
        itemStyle: { color: '#409eff' },
      },
    ],
  })
}

function initOverviewChart() {
  if (!overviewChartRef.value) return
  if (overviewChart) overviewChart.dispose()

  overviewChart = echarts.init(overviewChartRef.value)
  overviewChart.setOption({
    tooltip: { trigger: 'item' },
    legend: { bottom: '0%' },
    series: [
      {
        name: '数据总览',
        type: 'pie',
        radius: ['40%', '70%'],
        avoidLabelOverlap: false,
        itemStyle: { borderRadius: 6, borderColor: '#fff', borderWidth: 2 },
        label: { show: false },
        emphasis: { label: { show: true, fontSize: 16, fontWeight: 'bold' } },
        data: [
          { value: stats.value.total_sessions, name: '总会话', itemStyle: { color: '#409eff' } },
          { value: stats.value.today_sessions, name: '今日会话', itemStyle: { color: '#67c23a' } },
          { value: stats.value.total_messages, name: '累计消息', itemStyle: { color: '#e6a23c' } },
          { value: stats.value.total_edits, name: '代码编辑', itemStyle: { color: '#f56c6c' } },
        ],
      },
    ],
  })
}

function resizeCharts() {
  trendChart?.resize()
  overviewChart?.resize()
}

onMounted(async () => {
  try {
    stats.value = await fetchDashboardStats()
  } catch {
    // 后端未就绪时使用默认值
  }
  await nextTick()
  initTrendChart()
  initOverviewChart()
  window.addEventListener('resize', resizeCharts)
})

onBeforeUnmount(() => {
  trendChart?.dispose()
  overviewChart?.dispose()
  window.removeEventListener('resize', resizeCharts)
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
  flex-shrink: 0;
}

.stat-info {
  flex: 1;
  min-width: 0;
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

.chart-container {
  width: 100%;
  height: 320px;
}

.table-row {
  margin-bottom: 20px;
}

.text-ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
