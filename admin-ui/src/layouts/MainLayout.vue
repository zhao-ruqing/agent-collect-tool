<template>
  <div class="shell">
    <!-- 侧边栏 -->
    <aside class="sidebar" :class="{ collapsed: isCollapse }">
      <!-- Logo 区域 -->
      <div class="brand">
        <div class="brand-icon">
          <svg viewBox="0 0 32 32" fill="none">
            <rect width="32" height="32" rx="8" fill="url(#g-brand)" />
            <path d="M10 22V10l6 6 6-6v12" stroke="#fff" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
            <defs>
              <linearGradient id="g-brand" x1="0" y1="0" x2="32" y2="32">
                <stop stop-color="#00d8ff" />
                <stop offset="1" stop-color="#7c3aed" />
              </linearGradient>
            </defs>
          </svg>
        </div>
        <transition name="fade-slide">
          <div v-if="!isCollapse" class="brand-text">
            <span class="brand-title">AgentCollect</span>
            <span class="brand-sub">数据采集平台</span>
          </div>
        </transition>
      </div>

      <!-- 导航菜单 -->
      <nav class="nav">
        <router-link
          v-for="item in menuItems"
          :key="item.path"
          :to="item.path"
          class="nav-item"
          :class="{ active: activeMenu === item.path }"
        >
          <span class="nav-icon">
            <component :is="item.icon" />
          </span>
          <span v-if="!isCollapse" class="nav-label">{{ item.label }}</span>
          <span v-if="activeMenu === item.path" class="nav-indicator"></span>
        </router-link>
      </nav>

      <!-- 底部折叠按钮 -->
      <div class="sidebar-footer">
        <button class="collapse-btn" @click="toggleSidebar">
          <el-icon :size="18"><Fold v-if="!isCollapse" /><Expand v-else /></el-icon>
        </button>
      </div>
    </aside>

    <!-- 主区域 -->
    <div class="main">
      <!-- 顶栏 -->
      <header class="topbar">
        <div class="topbar-left">
          <div class="breadcrumb">
            <span class="breadcrumb-current">{{ currentPageTitle }}</span>
          </div>
        </div>
        <div class="topbar-right">
          <div class="status-dot" title="系统运行中"></div>
          <span class="status-text">系统正常</span>
        </div>
      </header>

      <!-- 内容区 -->
      <main class="content">
        <router-view v-slot="{ Component }">
          <transition name="page-fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </main>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRoute } from 'vue-router'
import {
  Monitor,
  ChatDotSquare,
  Edit,
  Timer,
  Fold,
  Expand,
} from '@element-plus/icons-vue'

const route = useRoute()
const isCollapse = ref(false)

const menuItems = [
  { path: '/dashboard',     label: '仪表盘',     icon: Monitor },
  { path: '/conversations', label: '对话记录',   icon: ChatDotSquare },
  { path: '/edits',         label: '代码编辑',   icon: Edit },
  { path: '/events',        label: '行为事件',   icon: Timer },
  { path: '/agents',        label: '客户端管理', icon: Monitor },
]

const activeMenu = computed(() => {
  const p = route.path
  return p === '/' ? '/dashboard' : p
})

const currentPageTitle = computed(() => {
  const item = menuItems.find((m) => m.path === activeMenu.value)
  return item?.label || ''
})

function toggleSidebar() {
  isCollapse.value = !isCollapse.value
}
</script>

<style scoped>
/* ============================================================
   Shell — 整体布局
   ============================================================ */
.shell {
  display: flex;
  height: 100vh;
  overflow: hidden;
  background: var(--c-bg-deep);
}

/* ============================================================
   Sidebar — 侧边栏（玻璃拟态 + 科技感）
   ============================================================ */
.sidebar {
  width: 240px;
  min-width: 240px;
  display: flex;
  flex-direction: column;
  background: var(--c-bg-sidebar);
  border-right: 1px solid var(--c-border);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  transition: width var(--transition-base), min-width var(--transition-base);
  position: relative;
  z-index: 10;
}
.sidebar::after {
  content: '';
  position: absolute;
  top: 0; right: -1px; bottom: 0;
  width: 1px;
  background: linear-gradient(180deg,
    transparent 0%,
    var(--c-accent) 10%,
    transparent 50%,
    var(--c-purple) 90%,
    transparent 100%
  );
  opacity: 0.5;
}
.sidebar.collapsed {
  width: 72px;
  min-width: 72px;
}

/* Brand */
.brand {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 20px 18px;
  border-bottom: 1px solid var(--c-border);
}
.brand-icon {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
}
.brand-icon svg {
  width: 100%;
  height: 100%;
}
.brand-text {
  overflow: hidden;
  white-space: nowrap;
}
.brand-title {
  display: block;
  font-size: 15px;
  font-weight: 700;
  letter-spacing: -0.3px;
  color: var(--c-text);
  line-height: 1.2;
}
.brand-sub {
  display: block;
  font-size: 10px;
  font-weight: 400;
  color: var(--c-text-muted);
  letter-spacing: 1.5px;
  text-transform: uppercase;
}

/* Nav */
.nav {
  flex: 1;
  padding: 12px 10px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-y: auto;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 11px 14px;
  border-radius: var(--radius-md);
  color: var(--c-text-dim);
  text-decoration: none;
  font-size: 14px;
  font-weight: 450;
  transition: all var(--transition-base);
  position: relative;
  cursor: pointer;
}
.nav-item:hover {
  background: var(--c-bg-hover);
  color: var(--c-text);
}
.nav-item.active {
  background: var(--c-accent-dim);
  color: var(--c-accent);
  font-weight: 550;
}
.nav-item.active .nav-icon {
  color: var(--c-accent);
}
.nav-icon {
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  font-size: 18px;
  transition: color var(--transition-base);
}
.nav-label {
  white-space: nowrap;
  overflow: hidden;
}
.nav-indicator {
  position: absolute;
  left: 0; top: 50%;
  transform: translateY(-50%);
  width: 3px;
  height: 20px;
  background: var(--c-accent);
  border-radius: 0 4px 4px 0;
  box-shadow: 0 0 10px var(--c-accent-glow);
}

/* Sidebar footer */
.sidebar-footer {
  padding: 12px 18px;
  border-top: 1px solid var(--c-border);
}
.collapse-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  padding: 8px;
  border: 1px solid var(--c-border);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--c-text-dim);
  cursor: pointer;
  transition: all var(--transition-fast);
}
.collapse-btn:hover {
  background: var(--c-bg-hover);
  color: var(--c-text);
  border-color: var(--c-border-active);
}

/* ============================================================
   Main — 主区域
   ============================================================ */
.main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  position: relative;
}

/* Topbar */
.topbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 56px;
  padding: 0 28px;
  border-bottom: 1px solid var(--c-border);
  background: var(--c-bg-sidebar);
  backdrop-filter: blur(16px);
  -webkit-backdrop-filter: blur(16px);
  flex-shrink: 0;
}
.breadcrumb-current {
  font-size: 15px;
  font-weight: 600;
  color: var(--c-text);
  letter-spacing: -0.2px;
}
.topbar-right {
  display: flex;
  align-items: center;
  gap: 10px;
}
.status-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--c-emerald);
  box-shadow: 0 0 8px rgba(52, 211, 153, 0.5);
  animation: glowPulse 2s ease-in-out infinite;
}
.status-text {
  font-size: 12px;
  color: var(--c-text-dim);
  font-weight: 450;
}

/* Content */
.content {
  flex: 1;
  overflow-y: auto;
  padding: 24px 28px;
}

/* ============================================================
   Transitions
   ============================================================ */
.fade-slide-enter-active,
.fade-slide-leave-active {
  transition: all 0.25s ease;
}
.fade-slide-enter-from,
.fade-slide-leave-to {
  opacity: 0;
  transform: translateX(-6px);
}

.page-fade-enter-active,
.page-fade-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.page-fade-enter-from {
  opacity: 0;
  transform: translateY(8px);
}
.page-fade-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
