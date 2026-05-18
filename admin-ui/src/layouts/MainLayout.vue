<template>
  <el-container class="main-layout">
    <!-- 侧边栏 -->
    <el-aside :width="isCollapse ? '64px' : '220px'" class="sidebar">
      <div class="logo">
        <span v-if="!isCollapse">Agent Collect</span>
        <span v-else class="logo-collapsed">AC</span>
      </div>
      <el-menu
        :default-active="activeMenu"
        router
        :collapse="isCollapse"
        background-color="#304156"
        text-color="#bfcbd9"
        active-text-color="#409eff"
      >
        <el-menu-item index="/dashboard">
          <el-icon><Monitor /></el-icon>
          <span>仪表盘</span>
        </el-menu-item>
        <el-menu-item index="/conversations">
          <el-icon><ChatDotSquare /></el-icon>
          <span>对话记录</span>
        </el-menu-item>
        <el-menu-item index="/edits">
          <el-icon><Edit /></el-icon>
          <span>代码编辑</span>
        </el-menu-item>
        <el-menu-item index="/events">
          <el-icon><Timer /></el-icon>
          <span>行为事件</span>
        </el-menu-item>
        <el-menu-item index="/agents">
          <el-icon><Monitor /></el-icon>
          <span>客户端管理</span>
        </el-menu-item>
      </el-menu>
    </el-aside>

    <!-- 内容区 -->
    <el-container class="content-container">
      <el-header class="header">
        <div class="header-left">
          <el-button
            text
            @click="toggleSidebar"
          >
            <el-icon><Fold v-if="!isCollapse" /><Expand v-else /></el-icon>
          </el-button>
          <span class="title">AI 编程工具数据采集系统</span>
        </div>
      </el-header>
      <el-main class="main-content">
        <router-view />
      </el-main>
    </el-container>
  </el-container>
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

const activeMenu = computed(() => {
  const path = route.path
  if (path === '/') return '/dashboard'
  return path
})

function toggleSidebar() {
  isCollapse.value = !isCollapse.value
}
</script>

<style scoped>
.main-layout {
  height: 100vh;
  width: 100vw;
}

.sidebar {
  background-color: #304156;
  overflow: hidden;
  transition: width 0.3s;
}

.logo {
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 18px;
  font-weight: bold;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.logo-collapsed {
  font-size: 14px;
}

.content-container {
  flex-direction: column;
}

.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  background: #fff;
  border-bottom: 1px solid #e6e6e6;
  padding: 0 20px;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.title {
  font-size: 16px;
  color: #303133;
}

.main-content {
  background-color: #f0f2f5;
}
</style>
