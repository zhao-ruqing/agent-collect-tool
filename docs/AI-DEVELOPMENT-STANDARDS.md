# AI 操作文档与开发规范

> 目的：统一 AI 辅助开发的行为边界、代码风格、数据协议，确保多人/多轮开发不跑偏、不返工、可追溯。

---

## 一、AI 操作准则

### 1.0 开发进度记录（强制）

**每次编辑/提交代码后，必须更新开发进度文件。**

- **存放位置：** `docs/AI-Development-Progress/`
- **文件命名：** `YYYY-MM-DD_HH-标题.md`（例：`2026-05-18_14-Claude采集器实现.md`）
- **文件周期：** 每 3 小时创建一个新文件，3 小时内的改动写在同一个文件中
- **文件结构：**
  ```markdown
  # [开发标题] — YYYY-MM-DD HH:MM

  ## 过往实现功能介绍
  （简述此文件之前已完成的功能，2~3 句话即可）

  ---

  ## 当次已实现功能
  - [功能点 1]：简要描述
  - [功能点 2]：简要描述

  ## 待实现功能
  - [待做 1]
  - [待做 2]
  ```
- **原则：** 精简扼要，每项一句话，不写长篇分析，只写结论和状态。
- **时机：** 每次 `git commit` 后立即追加；如果 3 小时内无 commit 但有关键决策，也追加记录。

### 1.1 基本原则

| 原则 | 说明 |
|------|------|
| **先读后写** | 修改任何文件前，必须先用 Read 工具阅读该文件，禁止盲改 |
| **先问后做** | 遇到模糊需求，先用 AskUserQuestion 确认，不要自行脑补 |
| **小步提交** | 每完成一个独立功能点就提交，不攒一个大 commit |
| **写前规划** | 非 trivial 任务（跨 3 个以上文件、新模块、架构变更）必须先写 Plan |
| **写完验证** | 代码修改后必须通过 `cargo check` / `cargo test` / `cargo clippy` |
| **不改无关代码** | 只修改任务直接相关的文件，不做顺手重构、修旧 bug、优化无关模块 |

### 1.2 禁止行为

- ❌ 禁止未经用户确认就修改项目配置（Cargo.toml 依赖、数据库 schema、API 接口签名）
- ❌ 禁止引入未经评审的第三方 crate（需说明选型理由 + 替代方案对比）
- ❌ 禁止在客户端做数据脱敏（脱敏是服务端职责，客户端只做采集）
- ❌ 禁止在采集逻辑中写死路径（必须从 config 读取）
- ❌ 禁止在代码中硬编码密钥、token、密码
- ❌ 禁止跳过测试直接提交（核心模块必须有单测覆盖）
- ❌ 禁止 `git push --force` 到 main/master 分支
- ❌ 禁止 `unwrap()` / `expect()` 在采集核心路径上使用（采集失败不应导致服务崩溃）

### 1.3 提交规范

```
<type>(<scope>): <简短描述>

类型: feat / fix / refactor / test / docs / chore
范围: agent / backend / admin / docs

示例:
feat(agent): 实现 Claude Code history.jsonl 增量解析
fix(agent): 修复 sled 缓冲队列阻塞导致数据丢失
refactor(backend): 重构脱敏模块，抽象 Desensitizer trait
```

---

## 二、Rust Agent 开发规范

### 2.1 目录结构（不可随意变动）

```
agent/
├── Cargo.toml
├── config.toml                  # 默认配置（不含密钥）
└── src/
    ├── main.rs                  # 入口：解析 CLI 参数，分发到 install/start/stop/run
    ├── config.rs                # 配置读取 + 校验（唯一配置入口）
    ├── service.rs               # Windows Service 生命周期管理
    ├── collector/               # 采集器模块
    │   ├── mod.rs               # Collector trait 定义
    │   ├── claude.rs            # Claude Code 采集实现
    │   ├── trae.rs              # Trae 采集实现
    │   └── file_watcher.rs      # 文件系统监控（可选）
    ├── engine/                  # 采集引擎
    │   ├── mod.rs               # Engine struct，编排采集→去重→缓冲→上报
    │   ├── dedup.rs             # 去重逻辑
    │   └── aggregator.rs        # 会话聚合逻辑
    ├── reporter/                # 上报模块
    │   ├── mod.rs               # Reporter trait 定义
    │   ├── http.rs              # HTTP 上报实现（reqwest）
    │   └── queue.rs             # 本地缓冲队列（sled）
    └── util/                    # 工具函数
        ├── mod.rs
        ├── crypto.rs            # 哈希/签名工具
        └── time.rs              # 时间戳规范化
```

### 2.2 命名规范

```rust
// 模块: snake_case
mod file_watcher;

// 类型: CamelCase (struct, enum, trait)
pub struct CodeEditRecord { ... }
pub enum ToolType { ... }
pub trait Collector { ... }

// 函数/方法: snake_case
pub fn parse_history_jsonl(path: &Path) -> Result<Vec<Record>> { ... }

// 常量: SCREAMING_SNAKE_CASE
const MAX_BATCH_SIZE: usize = 100;
const DEFAULT_INTERVAL_SECS: u64 = 60;

// 私有字段不加 pub，公开 API 必须加文档注释
/// 从 Claude Code 的 history.jsonl 中增量解析用户输入记录
pub fn incremental_parse(path: &Path, last_offset: &mut u64) -> Result<Vec<HistoryEntry>> {
```

### 2.3 错误处理

```rust
// 禁止在采集路径上 panic
// 使用 Result + anyhow/thiserror

#[derive(Debug, thiserror::Error)]
pub enum CollectorError {
    #[error("日志文件不存在: {0}")]
    FileNotFound(PathBuf),

    #[error("JSONL 解析失败，行 {line}: {source}")]
    ParseError { line: u64, source: serde_json::Error },

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
}

// 所有 public API 返回 Result，调用方统一处理
pub fn collect(&mut self) -> Result<Vec<CollectedData>, CollectorError> {
    // 单个采集源失败不传播，记录日志后继续下一个源
}
```

### 2.4 日志规范

```rust
// 使用 log crate + env_logger，按级别分类
use log::{info, warn, error, debug, trace};

// info: 服务启动/停止、成功上报批次大小
info!("Agent 启动，采集间隔: {}s，覆盖工具: {:?}", interval, tools);

// warn: 可恢复错误（单次采集失败、网络超时重试）
warn!("Claude Code history.jsonl 解析跳行 #{}，跳过该行", line_no);

// error: 不可恢复但不应崩溃（sled 损坏、配置无效）
error!("sled 数据库损坏，尝试重建缓冲队列");

// debug: 开发调试用（采集到的记录数、上报耗时）
debug!("本轮采集到 {} 条对话记录, {} 条编辑记录", msg_count, edit_count);
```

### 2.5 性能约束

| 指标 | 硬限制 | 说明 |
|------|--------|------|
| 内存峰值 | ≤ 100 MB | 含 sled 缓冲 + 运行时 |
| CPU 持续占用 | ≤ 1% | 非采集周期内应接近 0% |
| 磁盘写入 | ≤ 10 MB/天 | sled 缓冲 + 日志文件 |
| 网络上行 | ≤ 100 MB/天 | 批量压缩上报 |
| 启动时间 | ≤ 3 秒 | 冷启动到首轮采集完成 |

### 2.6 采集器接口规范

```rust
/// 所有 AI 工具采集器必须实现此 trait
#[async_trait]
pub trait Collector: Send + Sync {
    /// 采集器名称，如 "claude_code", "trae"
    fn name(&self) -> &'static str;

    /// 检测该工具是否已安装（数据目录是否存在）
    fn is_installed(&self) -> bool;

    /// 检测该工具当前是否在运行
    fn is_running(&self) -> bool;

    /// 执行增量采集，返回新产生的记录
    /// 内部需维护 offset/cursor，避免重复采集
    async fn collect_incremental(&mut self) -> Result<Vec<RawEvent>, CollectorError>;

    /// 重置采集位置（用于 clean start / 服务端下发重置指令）
    fn reset_cursor(&mut self);
}
```

---

## 三、数据协议规范

### 3.1 客户端上报格式

```json
// POST /api/v1/collect
{
  "agent_id": "uuid-v4",                    // 安装时生成的唯一标识
  "agent_version": "0.1.0",
  "os": "windows",
  "os_version": "10.0.26200",
  "hostname_hash": "sha256(hostname+salt)",
  "collected_at": "2026-05-18T14:00:00Z",
  "sequence": 42,                           // 单调递增序号，用于去重
  "events": [
    {
      "event_type": "conversation",         // conversation | code_edit | action | session
      "tool": "claude_code",
      "tool_version": "2.1.143",
      "session_id": "969b0c09-...",
      "project_path_hash": "sha256(project_path+salt)",
      "data": { ... }                       // 具体事件数据，见下方
    }
  ]
}
```

### 3.2 事件类型定义

```json
// event_type = "conversation"
{
  "messages": [
    {
      "role": "user",
      "content_hash": "sha256(content)",    // 用于去重，不传原文
      "token_estimate": 150,
      "seq": 1,
      "timestamp": "2026-05-18T05:35:08Z"
    },
    {
      "role": "assistant",
      "content_hash": "sha256(content)",
      "token_estimate": 800,
      "seq": 2,
      "timestamp": "2026-05-18T05:35:12Z"
    }
  ],
  "message_count": 2,
  "started_at": "2026-05-18T05:35:08Z",
  "ended_at": "2026-05-18T05:35:12Z"
}

// event_type = "code_edit"
{
  "file_path_hash": "sha256(relative_path+salt)",
  "language": "rust",
  "edit_type": "modify",
  "diff_skeleton": "@@ -10,3 +10,5 @@\n fn main() {\n-...\n+...\n }",
  "accepted": true,
  "edited_at": "2026-05-18T05:35:10Z"
}

// event_type = "action"
{
  "action_type": "accept",
  "target_msg_seq": 2,
  "timestamp": "2026-05-18T05:35:15Z"
}

// event_type = "session"
{
  "status": "active",
  "cwd_hash": "sha256(cwd+salt)",
  "git_branch": "HEAD",
  "started_at": 1779082504687,
  "updated_at": 1779083311801
}
```

### 3.3 接口契约

```
POST /api/v1/collect
  Request:  CollectionPayload (JSON, gzip 压缩)
  Response: 200 OK { "accepted": 42, "rejected": 0 }
           401 Unauthorized (api_key 无效)
           422 Unprocessable Entity (数据格式错误，附带 detail)
           429 Too Many Requests (限流，附带 Retry-After 头)
           500 Internal Server Error

POST /api/v1/agent/register
  Request:  { "agent_id": "...", "hostname_hash": "...", "os": "windows" }
  Response: 200 { "api_key": "...", "config": { ... } }
            409 Conflict (agent_id 已注册，返回已有 api_key)

GET  /api/v1/agent/config?agent_id=xxx
  Response: 200 { "collect_interval_secs": 60, "tools": ["claude_code"], ... }
            404 Not Found
```

---

## 四、服务端开发规范

### 4.1 脱敏规则（不可绕过）

| 数据类型 | 处理方式 | 实现要求 |
|---------|---------|---------|
| 文件绝对路径 | 只保留项目名 + 相对路径，用户名替换为 `<user>` | 正则 `/Users/[^/]+/` → `<user>/`，`C:\\Users\\[^\\]+\\` → `<user>\\` |
| 对话原文 | **禁止入库**，只存 content_hash + 前 200 字摘要 | 入库前截断 + 哈希 |
| Diff 内容 | 保留语法结构，字符串字面量替换为 `<str>`，数字替换为 `<num>`，变量名保留 | 基于 tree-sitter 做 AST 级脱敏 |
| 主机名/IP | SHA256(hostname + salt)，不可逆 | salt 存储在环境变量，不落盘 |
| 项目名称 | 保留（用于区分不同项目的使用情况） | 无需脱敏 |

### 4.2 数据库操作规范

```rust
// 必须使用参数化查询，禁止字符串拼接 SQL（sqlx）
sqlx::query("SELECT * FROM sessions WHERE agent_id = ? AND started_at > ?")
    .bind(agent_id)
    .bind(since)
    .fetch_all(&pool)

// 批量写入使用事务 + batch insert
let mut tx = pool.begin().await?;
sqlx::query("INSERT INTO messages (session_id, role, content_hash, seq) VALUES (?, ?, ?, ?)")
    .bind(sid).bind(role).bind(hash).bind(seq)
    .execute(&mut *tx).await?;
tx.commit().await?;

// 分区表按日期分区，查询必须带时间范围
// 所有外键必须建索引
// 敏感字段 NOT NULL 且设 DEFAULT
```

### 4.3 API 返回值规范

```json
// 成功
{ "code": 0, "data": { ... }, "message": "ok" }

// 业务错误
{ "code": 40001, "data": null, "message": "agent_id 不存在" }

// 系统错误
{ "code": 50000, "data": null, "message": "内部错误，已记录" }

// 分页列表
{
  "code": 0,
  "data": {
    "list": [...],
    "total": 1420,
    "page": 1,
    "page_size": 20
  }
}
```

---

## 五、管理后台开发规范

### 5.1 组件拆分标准

```
src/
├── pages/                       # 页面级组件（每个路由一个文件）
│   ├── Dashboard.tsx            # 仪表盘
│   ├── Conversations.tsx        # 对话列表
│   ├── ConversationDetail.tsx   # 对话详情
│   ├── CodeEdits.tsx            # 代码编辑列表
│   └── Events.tsx               # 行为事件
├── components/                  # 可复用组件
│   ├── FilterBar.tsx            # 筛选栏（时间、工具、模型、客户端）
│   ├── DataTable.tsx            # 通用分页表格
│   ├── StatCard.tsx             # 统计卡片
│   └── DiffViewer.tsx           # Diff 骨架渲染
├── hooks/                       # 自定义 Hook
│   ├── usePagination.ts
│   └── usePolling.ts            # 仪表盘自动刷新
├── api/                         # API 请求封装
│   ├── client.ts                # axios/fetch 实例，统一拦截
│   └── endpoints.ts             # 接口路径常量
└── types/                       # TypeScript 类型定义
    └── index.ts                 # 与后端数据模型同步
```

### 5.2 状态管理

```
全局状态: 使用 Context 或 Zustand，仅存放:
  - 当前用户信息（如有登录）
  - 全局筛选条件（时间范围、客户端选择）
  - 主题/语言

页面状态: 使用组件内 useState + useEffect，不提升到全局
  - 分页信息
  - 表格数据
  - 加载/错误状态

数据请求: 使用 React Query (TanStack Query) 管理缓存和自动刷新
```

### 5.3 UI 交互约定

- 所有表格支持排序 + 分页 + 列筛选
- 日期选择器默认为最近 7 天
- Diff 骨架使用等宽字体 + 添加/删除行着色（绿/红）
- 统计数据卡片支持点击下钻到明细列表
- 无数据时显示空状态提示，非空白页
- 加载中显示骨架屏，非全屏 spinner

---

## 六、Git 工作流

### 6.1 分支策略

```
main              ← 生产就绪分支，只允许 PR 合入
  └── develop     ← 开发主分支
        ├── feat/agent-xxx      ← 客户端功能分支
        ├── feat/backend-xxx    ← 后端功能分支
        ├── feat/admin-xxx      ← 管理后台功能分支
        ├── fix/xxx             ← Bug 修复
        └── chore/xxx           ← 工程化（依赖、CI、文档）
```

### 6.2 禁止的操作

```bash
# 禁止
git push --force origin main
git push --force origin develop
git commit --no-verify           # 跳过 pre-commit hook
git commit --amend (已推送到远程的 commit)

# 允许
git commit --amend (仅本地未推送的 commit)
git rebase -i (整理本地分支)
```

### 6.3 PR 要求

- PR 标题遵循 commit 规范格式
- PR 描述包含：改动说明 + 测试方式 + 截图（前端改动）
- 至少 1 个 approve 后方可合入
- CI 全部通过（lint + test + build）
- 无未解决的冲突

---

## 七、测试要求

### 7.1 覆盖率底线

| 模块 | 单测覆盖率 | 说明 |
|------|-----------|------|
| collector/*（采集器） | ≥ 80% | 日志解析逻辑必须充分覆盖 |
| engine/dedup（去重） | ≥ 90% | 纯逻辑模块，容易覆盖 |
| reporter/queue（队列） | ≥ 80% | 缓冲 + 重试逻辑 |
| desensitize（脱敏） | ≥ 95% | 核心安全模块 |
| API handler | ≥ 70% | 业务逻辑 |

### 7.2 测试数据规范

```rust
// Rust: 测试数据放在 tests/fixtures/ 下
#[test]
fn test_parse_history_jsonl() {
    let path = Path::new("tests/fixtures/claude/history_sample.jsonl");
    let records = parse_history_jsonl(path).unwrap();
    assert_eq!(records.len(), 3);
}

// Go: 测试数据用 embed 或 testdata/
//go:embed testdata/conversation.json
var samplePayload []byte
```

---

## 八、安全清单

| 检查项 | 要求 |
|--------|------|
| API 鉴权 | Agent 使用 api_key，管理后台使用 JWT |
| 传输加密 | 全链路 HTTPS |
| SQL 注入 | 100% 参数化查询，静态检查通过 |
| 敏感数据 | 密钥存环境变量，脱敏在服务端，日志不打印原文 |
| 依赖审计 | 定期 `cargo audit` / `go mod tidy` / `npm audit` |
| 采集限流 | 单 Agent 每分钟最多 1 次上报，超频返回 429 |
| 数据留存 | 原始对话 content 不入库；日志文件保留 ≤ 30 天 |

---

## 九、新增工具的扩展流程

当需要新增一个 AI 工具的采集支持时（如 Copilot、Cursor），按以下步骤操作：

1. **逆向数据源**：定位工具的本地数据目录 → 分析日志/存储格式 → 确认可采集字段
2. **更新协议**：如果需要新的 event_type 或字段，先更新本文档第三章的数据协议
3. **实现 Collector trait**：在 `collector/` 下新建模块，实现 `Collector` trait
4. **注册采集器**：在 `config.rs` 的 `ToolType` 枚举中新增变体 + 在 engine 中注册
5. **更新脱敏规则**：如果新工具有不同的路径格式，补充第四章脱敏规则
6. **更新文档**：在 README 数据源章节补充新工具的格式说明
7. **提 PR**：独立分支 `feat/agent-xxx-collector`，单测覆盖率 ≥ 80%

---

## 十、附录：常用命令速查

```bash
# Rust Agent
cargo check              # 快速检查编译错误
cargo test               # 运行全部测试
cargo test -- --nocapture    # 显示日志输出
cargo clippy -- -D warnings  # 严格 lint
cargo fmt -- --check         # 检查格式
cargo build --release        # 生产构建

# Rust 后端
cargo check              # 快速检查编译错误
cargo test               # 运行测试
cargo run                # 启动服务（开发模式）
cargo build --release    # 生产构建
sqlx migrate run         # 执行数据库迁移

# 管理后台
npm run dev               # 开发模式
npm run build             # 生产构建
npm run lint              # ESLint
npx vue-tsc --noEmit      # Vue TypeScript 类型检查

# 数据库迁移
cd backend && sqlx migrate run    # 执行迁移
cd backend && sqlx migrate revert # 回滚
```
