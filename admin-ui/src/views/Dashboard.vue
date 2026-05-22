<template>
  <div class="dashboard">
    <!-- 统计卡片 -->
    <div class="stat-grid">
      <div
        v-for="(card, i) in statCards"
        :key="card.key"
        class="stat-card"
        :style="{ animationDelay: `${i * 0.06}s` }"
      >
        <div class="stat-icon" :style="{ background: card.gradient }">
          <el-icon :size="22"><component :is="card.icon" /></el-icon>
        </div>
        <div class="stat-body">
          <div class="stat-label">{{ card.label }}</div>
          <div class="stat-value">
            <span ref="countRefs" class="stat-number">{{ card.value }}</span>
          </div>
          <div class="stat-sub">{{ card.sub }}</div>
        </div>
        <!-- 装饰光效 -->
        <div class="stat-glow" :style="{ background: card.gradient }"></div>
      </div>
    </div>

    <!-- 图表区 -->
    <div class="chart-grid">
      <div class="panel chart-panel">
        <div class="panel-header">
          <h3 class="panel-title">对话趋势</h3>
          <span class="panel-badge">近7天</span>
        </div>
        <div ref="trendChartRef" class="chart-box"></div>
      </div>
      <div class="panel chart-panel">
        <div class="panel-header">
          <h3 class="panel-title">数据分布</h3>
          <span class="panel-badge">总览</span>
        </div>
        <div ref="overviewChartRef" class="chart-box"></div>
      </div>
    </div>

    <!-- 最近会话 -->
    <div class="panel">
      <div class="panel-header">
        <h3 class="panel-title">最近会话</h3>
        <span class="panel-badge">{{ stats.recent_sessions.length }} 条</span>
      </div>
      <el-table :data="stats.recent_sessions" stripe>
        <el-table-column label="会话 ID" min-width="220">
          <template #default="{ row }">
            <el-tooltip :content="row.id" placement="top">
              <span class="text-mono text-ellipsis"
                >{{ row.id.substring(0, 18) }}...</span
              >
            </el-tooltip>
          </template>
        </el-table-column>
        <el-table-column label="客户端" width="200">
          <template #default="{ row }">
            <span class="text-mono"
              >{{ row.agent_id.substring(0, 14) }}...</span
            >
          </template>
        </el-table-column>
        <el-table-column label="分支" width="140">
          <template #default="{ row }">
            <span v-if="row.git_branch" class="text-mono">{{
              row.git_branch
            }}</span>
            <span v-else class="text-dim">—</span>
          </template>
        </el-table-column>
        <el-table-column label="开始时间" width="180">
          <template #default="{ row }">{{
            formatTime(row.started_at)
          }}</template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag
              :type="row.ended_at ? 'success' : 'warning'"
              size="small"
              effect="dark"
            >
              {{ row.ended_at ? "已结束" : "进行中" }}
            </el-tag>
          </template>
        </el-table-column>
      </el-table>
    </div>
  </div>
</template>

<script setup lang="ts">
import {
  ChatDotSquare,
  EditPen,
  Message,
  Monitor,
} from "@element-plus/icons-vue";
import * as echarts from "echarts";
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { fetchDashboardStats } from "../api/dashboard";
import { useFilterStore } from "../stores/filter";
import type { DashboardStats } from "../types";

const filterStore = useFilterStore();
const stats = ref<DashboardStats>({
  total_agents: 0,
  total_sessions: 0,
  today_sessions: 0,
  total_messages: 0,
  total_edits: 0,
  recent_sessions: [],
  daily_trend: [],
});

const statCards = ref([
  {
    key: "agents",
    label: "活跃客户端",
    value: "0",
    sub: "已注册的采集终端",
    icon: Monitor,
    gradient: "linear-gradient(135deg, #00d8ff, #0ea5e9)",
  },
  {
    key: "today",
    label: "今日对话",
    value: "0",
    sub: "今日新增会话",
    icon: ChatDotSquare,
    gradient: "linear-gradient(135deg, #34d399, #10b981)",
  },
  {
    key: "messages",
    label: "累计消息",
    value: "0",
    sub: "AI 对话总条数",
    icon: Message,
    gradient: "linear-gradient(135deg, #a78bfa, #7c3aed)",
  },
  {
    key: "edits",
    label: "代码编辑",
    value: "0",
    sub: "AI 代码修改次数",
    icon: EditPen,
    gradient: "linear-gradient(135deg, #fb7185, #f43f5e)",
  },
]);

function syncStatCards() {
  statCards.value[0].value = formatNumber(stats.value.total_agents);
  statCards.value[1].value = formatNumber(stats.value.today_sessions);
  statCards.value[2].value = formatNumber(stats.value.total_messages);
  statCards.value[3].value = formatNumber(stats.value.total_edits);
}

const trendChartRef = ref<HTMLElement | null>(null);
const overviewChartRef = ref<HTMLElement | null>(null);
let trendChart: echarts.ECharts | null = null;
let overviewChart: echarts.ECharts | null = null;

function formatNumber(n: number): string {
  if (n >= 10000) return (n / 1000).toFixed(1) + "k";
  return n.toLocaleString();
}
function formatTime(iso: string): string {
  if (!iso) return "-";
  return new Date(iso).toLocaleString("zh-CN", { hour12: false });
}

function initTrendChart() {
  if (!trendChartRef.value) return;
  trendChart?.dispose();
  trendChart = echarts.init(trendChartRef.value);
  const dates = stats.value.daily_trend.map((d) => d.date);
  const counts = stats.value.daily_trend.map((d) => d.count);

  trendChart.setOption({
    tooltip: {
      trigger: "axis",
      backgroundColor: "rgba(15,17,32,0.95)",
      borderColor: "rgba(255,255,255,0.1)",
      textStyle: { color: "#e2e8f0", fontSize: 12 },
      axisPointer: { lineStyle: { color: "rgba(255,255,255,0.08)" } },
    },
    grid: { left: 10, right: 30, top: 8, bottom: 5, containLabel: true },
    xAxis: {
      type: "category",
      data: dates.length > 0 ? dates : ["暂无数据"],
      boundaryGap: false,
      axisLine: { lineStyle: { color: "rgba(255,255,255,0.1)" } },
      axisTick: { show: false },
      axisLabel: { color: "#94a3b8", fontSize: 11 },
    },
    yAxis: {
      type: "value",
      minInterval: 1,
      splitLine: { lineStyle: { color: "rgba(255,255,255,0.04)" } },
      axisLabel: { color: "#64748b", fontSize: 11 },
    },
    series: [
      {
        name: "对话数",
        type: "line",
        smooth: true,
        symbol: "circle",
        symbolSize: 6,
        data: counts.length > 0 ? counts : [0],
        lineStyle: { color: "#00d8ff", width: 2.5 },
        itemStyle: { color: "#00d8ff", borderColor: "#0b0d1a", borderWidth: 2 },
        areaStyle: {
          color: new echarts.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: "rgba(0,216,255,0.2)" },
            { offset: 1, color: "rgba(0,216,255,0.0)" },
          ]),
        },
      },
    ],
  });
}

function initOverviewChart() {
  if (!overviewChartRef.value) return;
  overviewChart?.dispose();
  overviewChart = echarts.init(overviewChartRef.value);

  const colors = ["#00d8ff", "#34d399", "#a78bfa", "#fb7185"];
  overviewChart.setOption({
    tooltip: {
      trigger: "item",
      backgroundColor: "rgba(15,17,32,0.95)",
      borderColor: "rgba(255,255,255,0.1)",
      textStyle: { color: "#e2e8f0", fontSize: 12 },
    },
    series: [
      {
        type: "pie",
        radius: ["55%", "80%"],
        center: ["50%", "48%"],
        avoidLabelOverlap: false,
        itemStyle: {
          borderRadius: 8,
          borderColor: "#0b0d1a",
          borderWidth: 4,
        },
        label: { show: false },
        emphasis: {
          label: {
            show: true,
            fontSize: 16,
            fontWeight: "bold",
            color: "#e2e8f0",
          },
          scaleSize: 8,
        },
        data: [
          {
            value: stats.value.total_sessions,
            name: "总会话",
            itemStyle: { color: colors[0] },
          },
          {
            value: stats.value.today_sessions,
            name: "今日会话",
            itemStyle: { color: colors[1] },
          },
          {
            value: stats.value.total_messages,
            name: "累计消息",
            itemStyle: { color: colors[2] },
          },
          {
            value: stats.value.total_edits,
            name: "代码编辑",
            itemStyle: { color: colors[3] },
          },
        ],
      },
    ],
  });
}

function resizeCharts() {
  trendChart?.resize();
  overviewChart?.resize();
}

async function loadStats() {
  try {
    const params: Record<string, string> = {};
    if (filterStore.toolType) params.tool_type = filterStore.toolType;
    stats.value = await fetchDashboardStats(params);
  } catch {
    /* 使用默认值 */
  }
  syncStatCards();
  await nextTick();
  initTrendChart();
  initOverviewChart();
}

watch(() => filterStore.toolType, () => loadStats());

onMounted(async () => {
  await loadStats();
  window.addEventListener("resize", resizeCharts);
});

onBeforeUnmount(() => {
  trendChart?.dispose();
  overviewChart?.dispose();
  window.removeEventListener("resize", resizeCharts);
});
</script>

<style scoped>
/* ============================================================
   Stat Grid
   ============================================================ */
.stat-grid {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 18px;
  margin-bottom: 22px;
}
.stat-card {
  position: relative;
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 22px 20px;
  border-radius: var(--radius-lg);
  background: var(--c-bg-card);
  border: 1px solid var(--c-border);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  overflow: hidden;
  cursor: default;
  animation: fadeInUp 0.5s ease both;
  transition: all var(--transition-base);
}
.stat-card:hover {
  border-color: var(--c-border-active);
  transform: translateY(-2px);
  box-shadow: var(--shadow-card-hover);
}
.stat-icon {
  width: 48px;
  height: 48px;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  flex-shrink: 0;
  position: relative;
  z-index: 1;
}
.stat-body {
  flex: 1;
  min-width: 0;
  position: relative;
  z-index: 1;
}
.stat-label {
  font-size: 12px;
  color: var(--c-text-muted);
  margin-bottom: 4px;
  font-weight: 450;
  letter-spacing: 0.3px;
}
.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: var(--c-text);
  line-height: 1.2;
  letter-spacing: -0.5px;
}
.stat-sub {
  font-size: 11px;
  color: var(--c-text-muted);
  margin-top: 2px;
}
.stat-glow {
  position: absolute;
  top: -30%;
  right: -20%;
  width: 120px;
  height: 120px;
  border-radius: 50%;
  filter: blur(40px);
  opacity: 0.08;
  pointer-events: none;
}

/* ============================================================
   Panels
   ============================================================ */
.panel {
  background: var(--c-bg-card);
  border: 1px solid var(--c-border);
  border-radius: var(--radius-lg);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  margin-bottom: 20px;
  animation: fadeInUp 0.5s ease both;
}
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--c-border);
}
.panel-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--c-text);
  margin: 0;
  letter-spacing: -0.2px;
}
.panel-badge {
  font-size: 11px;
  color: var(--c-text-muted);
  background: var(--c-bg-surface);
  padding: 3px 10px;
  border-radius: 20px;
  border: 1px solid var(--c-border);
}

/* Chart */
.chart-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 20px;
  margin-bottom: 20px;
}
.chart-panel {
  margin-bottom: 0;
}
.chart-box {
  width: 100%;
  height: 300px;
  padding: 10px 12px;
}

/* ============================================================
   Helpers
   ============================================================ */
.text-mono {
  font-family: "JetBrains Mono", "Fira Code", "Consolas", monospace;
  font-size: 12px;
}
.text-ellipsis {
  display: inline-block;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.text-dim {
  color: var(--c-text-muted);
}

@media (max-width: 1200px) {
  .stat-grid {
    grid-template-columns: repeat(2, 1fr);
  }
  .chart-grid {
    grid-template-columns: 1fr;
  }
}
</style>
