<template>
  <div class="page-view">
    <div class="panel">
      <div class="panel-header">
        <h3 class="panel-title">代码编辑记录</h3>
        <span class="panel-badge">共 {{ pagination.total }} 条</span>
      </div>
      <el-table :data="tableData" border stripe v-loading="loading">
        <el-table-column label="ID" width="80" align="center">
          <template #default="{ row }">
            <span class="text-mono">{{ row.id }}</span>
          </template>
        </el-table-column>
        <el-table-column label="Session ID" min-width="220">
          <template #default="{ row }">
            <span class="text-mono">{{ row.session_id }}</span>
          </template>
        </el-table-column>
        <el-table-column label="文件哈希" width="200">
          <template #default="{ row }">
            <span class="text-mono"
              >{{ (row.file_path_hash || "—").substring(0, 12) }}...</span
            >
          </template>
        </el-table-column>
        <el-table-column label="编辑类型" width="110">
          <template #default="{ row }">
            <el-tag
              :type="editTagType(row.edit_type)"
              size="small"
              effect="dark"
            >
              {{ editLabel(row.edit_type) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="+行" width="70" align="center">
          <template #default="{ row }">
            <span class="num-add">{{ row.lines_added ?? 0 }}</span>
          </template>
        </el-table-column>
        <el-table-column label="-行" width="70" align="center">
          <template #default="{ row }">
            <span class="num-rm">{{ row.lines_removed ?? 0 }}</span>
          </template>
        </el-table-column>
        <el-table-column label="时间" width="180">
          <template #default="{ row }">{{ row.created_at || "—" }}</template>
        </el-table-column>
        <el-table-column label="操作" width="100" align="center">
          <template #default="{ row }">
            <el-button type="primary" link @click="showDiff(row)"
              >查看 Diff</el-button
            >
          </template>
        </el-table-column>
      </el-table>
      <div class="panel-footer">
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
    </div>

    <!-- Diff 查看弹窗 -->
    <el-dialog
      v-model="diffVisible"
      title="Diff 骨架"
      width="800px"
      top="20px"
      destroy-on-close
    >
      <div class="diff-box">
        <pre class="diff-content" v-html="renderDiff(diffContent)"></pre>
      </div>
      <template #footer>
        <el-button @click="diffVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from "vue";
import { fetchCodeEdits } from "../api/edits";
import type { CodeEdit } from "../types";

const loading = ref(false);
const pagination = reactive({ page: 1, pageSize: 10, total: 0 });
const tableData = ref<CodeEdit[]>([]);

function editTagType(
  type: string | null,
): "" | "success" | "warning" | "danger" | "info" {
  switch (type) {
    case "create":
      return "success";
    case "modify":
      return "warning";
    case "delete":
      return "danger";
    case "rename":
      return "info";
    default:
      return "";
  }
}
function editLabel(type: string | null): string {
  switch (type) {
    case "create":
      return "新建";
    case "modify":
      return "修改";
    case "delete":
      return "删除";
    case "rename":
      return "重命名";
    default:
      return type || "—";
  }
}

const diffVisible = ref(false);
const diffContent = ref("");

function showDiff(row: CodeEdit) {
  diffContent.value = row.diff_skeleton || "暂无 diff 数据";
  diffVisible.value = true;
}

function renderDiff(raw: string): string {
  if (!raw) return '<span class="diff-empty">暂无 diff 内容</span>';
  return raw
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .split("\n")
    .map((line) => {
      if (line.startsWith("@@"))
        return `<span class="diff-hunk">${line}</span>`;
      if (line.startsWith("+")) return `<span class="diff-add">${line}</span>`;
      if (line.startsWith("-")) return `<span class="diff-del">${line}</span>`;
      return `<span class="diff-ctx">${line}</span>`;
    })
    .join("\n");
}

async function loadData() {
  loading.value = true;
  try {
    const res = await fetchCodeEdits({
      page: pagination.page,
      page_size: pagination.pageSize,
    });
    tableData.value = res.list || [];
    pagination.total = res.total || 0;
  } catch {
    tableData.value = [];
    pagination.total = 0;
  } finally {
    loading.value = false;
  }
}

onMounted(() => loadData());
</script>

<style scoped>
.page-view {
  animation: fadeInUp 0.45s ease both;
}
.panel {
  background: var(--c-bg-card);
  border: 1px solid var(--c-border);
  border-radius: var(--radius-lg);
  backdrop-filter: blur(12px);
  -webkit-backdrop-filter: blur(12px);
  margin-bottom: 18px;
}
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 20px;
  border-bottom: 1px solid var(--c-border);
}
.panel-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--c-text);
  margin: 0;
}
.panel-badge {
  font-size: 11px;
  color: var(--c-text-muted);
  background: var(--c-bg-surface);
  padding: 3px 10px;
  border-radius: 20px;
  border: 1px solid var(--c-border);
}
.panel-footer {
  display: flex;
  justify-content: flex-end;
  padding: 14px 20px;
  border-top: 1px solid var(--c-border);
}
.text-mono {
  font-family: "JetBrains Mono", "Fira Code", "Consolas", monospace;
  font-size: 12px;
}
.num-add {
  color: var(--c-emerald);
  font-weight: 600;
  font-family: "JetBrains Mono", monospace;
}
.num-rm {
  color: var(--c-rose);
  font-weight: 600;
  font-family: "JetBrains Mono", monospace;
}

/* Diff 查看器 */
.diff-box {
  max-height: 500px;
  overflow: auto;
  border-radius: var(--radius-md);
  background: #0d1117;
}
.diff-content {
  margin: 0;
  padding: 16px;
  font-family: "JetBrains Mono", "Fira Code", "Consolas", monospace;
  font-size: 13px;
  line-height: 1.7;
  color: #c9d1d9;
  white-space: pre-wrap;
  word-break: break-all;
}
.diff-content :deep(.diff-hunk) {
  color: #58a6ff;
}
.diff-content :deep(.diff-add) {
  color: #7ee787;
  background: rgba(63, 185, 80, 0.15);
  display: inline-block;
  width: 100%;
}
.diff-content :deep(.diff-del) {
  color: #f85149;
  background: rgba(248, 81, 73, 0.15);
  display: inline-block;
  width: 100%;
}
.diff-content :deep(.diff-ctx) {
  color: #8b949e;
}
.diff-content :deep(.diff-empty) {
  color: #484f58;
  font-style: italic;
}
</style>
