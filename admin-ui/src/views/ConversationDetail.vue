<template>
  <div class="page-view" v-animate>
    <!-- 返回按钮 -->
    <div class="back-bar">
      <el-button text @click="router.push({ name: 'conversations' })">
        <el-icon><ArrowLeft /></el-icon>
        返回对话列表
      </el-button>
    </div>

    <!-- 会话信息 -->
    <div class="panel">
      <div class="panel-header">
        <h3 class="panel-title">会话信息</h3>
        <el-tag
          :type="detail.session?.ended_at ? 'success' : 'warning'"
          size="small"
          effect="dark"
        >
          {{ detail.session?.ended_at ? '已结束' : '进行中' }}
        </el-tag>
      </div>
      <div class="panel-body">
        <div class="info-grid" v-if="detail.session">
          <div class="info-item">
            <span class="info-label">Session ID</span>
            <span class="info-value text-mono">{{ detail.session.id }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">客户端</span>
            <span class="info-value text-mono">{{ detail.session.agent_id }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">Git 分支</span>
            <span class="info-value text-mono">{{ detail.session.git_branch || '—' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">开始时间</span>
            <span class="info-value">{{ formatTime(detail.session.started_at) }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">结束时间</span>
            <span class="info-value">{{ detail.session.ended_at ? formatTime(detail.session.ended_at) : '—' }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">消息数</span>
            <span class="info-value">{{ detail.messages?.length ?? 0 }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 消息列表 -->
    <div class="panel">
      <div class="panel-header">
        <h3 class="panel-title">对话消息</h3>
        <span class="panel-badge">{{ detail.messages?.length ?? 0 }} 条</span>
      </div>
      <div class="panel-body">
        <div class="msg-list" v-loading="loading">
          <div
            v-for="msg in detail.messages"
            :key="msg.id"
            class="msg-item"
            :class="`msg--${msg.role}`"
          >
            <div class="msg-avatar">
              <el-icon :size="16">
                <User v-if="msg.role === 'user'" />
                <ChatDotRound v-else />
              </el-icon>
            </div>
            <div class="msg-body">
              <div class="msg-header">
                <el-tag
                  :type="msg.role === 'user' ? 'info' : 'success'"
                  size="small"
                  effect="dark"
                >
                  {{ msg.role === 'user' ? '用户' : 'AI 助手' }}
                </el-tag>
                <span v-if="msg.model" class="msg-model">{{ msg.model }}</span>
                <span class="msg-time">{{ msg.created_at ? formatTime(msg.created_at) : '—' }}</span>
              </div>
              <div class="msg-content">
                <div class="msg-hash">
                  <span class="info-label">内容哈希</span>
                  <code class="text-mono">{{ msg.content_hash || '—' }}</code>
                </div>
                <div class="msg-tokens" v-if="msg.tokens_input || msg.tokens_output">
                  <span v-if="msg.tokens_input" class="token-badge">
                    输入: {{ formatNumber(msg.tokens_input) }} tokens
                  </span>
                  <span v-if="msg.tokens_output" class="token-badge">
                    输出: {{ formatNumber(msg.tokens_output) }} tokens
                  </span>
                </div>
              </div>
            </div>
          </div>
          <el-empty v-if="!loading && (!detail.messages || detail.messages.length === 0)" description="暂无消息" />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { ArrowLeft, User, ChatDotRound } from '@element-plus/icons-vue'
import { fetchConversationDetail } from '../api/conversations'
import type { Session } from '../types'

const router = useRouter()
const route = useRoute()
const loading = ref(false)

const detail = reactive<{
  session: Session | null
  messages: Array<{
    id: number
    role: string
    content_hash: string | null
    model: string | null
    tokens_input: number | null
    tokens_output: number | null
    created_at: string | null
  }>
}>({
  session: null,
  messages: [],
})

function formatNumber(n: number): string {
  if (n >= 10000) return (n / 1000).toFixed(1) + 'k'
  return n.toLocaleString()
}
function formatTime(iso: string): string {
  if (!iso) return '—'
  return new Date(iso).toLocaleString('zh-CN', { hour12: false })
}

onMounted(async () => {
  const sessionId = route.params.sessionId as string
  if (!sessionId) return
  loading.value = true
  try {
    const data = await fetchConversationDetail(sessionId)
    detail.session = data.session
    detail.messages = data.messages
  } catch { /* 使用默认值 */ }
  finally { loading.value = false }
})
</script>

<style scoped>
.page-view { animation: fadeInUp 0.45s ease both; max-width: 1000px; }

.back-bar { margin-bottom: 14px; }

/* Panel */
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
.panel-body { padding: 20px; }

/* Info Grid */
.info-grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; }
.info-item { display: flex; flex-direction: column; gap: 4px; }
.info-label { font-size: 11px; color: var(--c-text-muted); text-transform: uppercase; letter-spacing: 0.5px; }
.info-value { font-size: 13px; color: var(--c-text); word-break: break-all; }
.text-mono { font-family: 'JetBrains Mono', 'Fira Code', 'Consolas', monospace; font-size: 12px; }

/* Message List */
.msg-list { display: flex; flex-direction: column; gap: 16px; }
.msg-item { display: flex; gap: 14px; position: relative; }
.msg-avatar {
  width: 34px; height: 34px; border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0; margin-top: 2px;
}
.msg--user .msg-avatar { background: rgba(0, 216, 255, 0.15); color: var(--c-cyan); }
.msg--assistant .msg-avatar { background: rgba(52, 211, 153, 0.15); color: var(--c-emerald); }
.msg-body { flex: 1; min-width: 0; }
.msg-header { display: flex; align-items: center; gap: 10px; margin-bottom: 8px; }
.msg-model { font-size: 11px; color: var(--c-text-dim); }
.msg-time  { font-size: 11px; color: var(--c-text-muted); margin-left: auto; }
.msg-hash { display: flex; align-items: center; gap: 10px; margin-bottom: 6px; }
.msg-hash code {
  font-size: 11px; color: var(--c-text-dim); background: rgba(0,0,0,0.2);
  padding: 3px 8px; border-radius: var(--radius-sm);
  word-break: break-all; max-width: 400px; overflow: hidden; text-overflow: ellipsis;
  white-space: nowrap;
}
.msg-tokens { display: flex; gap: 10px; }
.token-badge {
  font-size: 11px; color: var(--c-text-dim); background: var(--c-bg-surface);
  padding: 2px 8px; border-radius: var(--radius-sm); border: 1px solid var(--c-border);
}

@media (max-width: 768px) {
  .info-grid { grid-template-columns: repeat(2, 1fr); }
}
</style>
