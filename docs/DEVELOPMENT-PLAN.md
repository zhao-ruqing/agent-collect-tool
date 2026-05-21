# 渐进式开发计划

> 后端：Rust (Axum) | 前端：Vue3 + Element Plus | Agent：Rust (Windows Service) | 数据库：MySQL 8.0

---

## 项目总览

```
agent-collect-tool/
├── agent/              # Rust Agent（Windows 服务）
├── backend/            # Rust 后端（Axum）
├── admin-ui/           # Vue3 管理后台
├── docs/               # 文档 & 开发进度
│   ├── AI-DEVELOPMENT-STANDARDS.md
│   ├── DEVELOPMENT-PLAN.md          ← 本文件
│   └── AI-Development-Progress/     ← 每次提交后的开发进度
└── README.md
```

---

## Phase 1：项目脚手架 & 基础设施（预计 3～5 天）

### 1.1 Agent 项目初始化

**目标：** 搭建 Rust Agent 项目骨架，可编译、可注册为 Windows 服务。

**文件列表：**
```
agent/
├── Cargo.toml                       # [dependencies] tokio, serde, serde_json, anyhow, thiserror, log, env_logger, reqwest, sled, notify, sysinfo, windows-service, toml, chrono, uuid
├── config.toml                      # 默认配置模板
└── src/
    ├── main.rs                      # CLI 入口：解析 install/start/stop/status/run 子命令
    ├── config.rs                    # 配置加载 + 校验（从 config.toml 和环境变量读取）
    ├── service.rs                   # Windows Service 生命周期（ServiceMain, ServiceControlHandler）
    └── util/
        ├── mod.rs
        ├── crypto.rs                # agent_id 生成：UUID v4，hostname 哈希：SHA256
        └── time.rs                  # 时间戳规范化工具（毫秒↔DateTime）
```

**具体步骤：**
1. `cargo init agent` 初始化项目
2. 配置 `Cargo.toml` — 添加所有基础依赖，版本锁定
3. 实现 `config.rs` — `AgentConfig` 结构体，实现 `serde::Deserialize`，包含 `load()` 和 `validate()` 方法
4. 实现 `util/crypto.rs` — `generate_agent_id()` 返回 UUID v4，`hash_hostname(salt: &str) -> String`
5. 实现 `util/time.rs` — `ms_to_datetime(ts: i64) -> DateTime<Utc>`，`iso_to_datetime(s: &str) -> Result<DateTime<Utc>>`
6. 实现 `service.rs` — Win32 Service 注册/启动/停止逻辑
7. 实现 `main.rs` — 子命令分发：`install`（注册服务）、`start`（启动服务）、`stop`、`status`、`run`（前台调试模式）
8. 编写 `config.toml` 模板

**验收标准：**
- `cargo build --release` 编译通过
- `agent.exe install` 注册服务成功（services.msc 可见）
- `agent.exe run` 前台模式正常启动，日志输出到文件
- 配置文件缺失时有明确的错误提示

---

### 1.2 后端项目初始化

**目标：** 搭建 Rust Axum 后端项目骨架，基础路由、数据库连接池、迁移系统就绪。

**文件列表：**
```
backend/
├── Cargo.toml                       # [dependencies] axum, tokio, serde, serde_json, sqlx, anyhow, thiserror, tracing, tracing-subscriber, uuid, chrono, sha2, tower-http
├── .env.example                     # DATABASE_URL, SALT, JWT_SECRET 示例
├── migrations/
│   └── 001_init.sql                 # 初始表结构（agents, sessions, messages, code_edits, action_events, daily_stats）
└── src/
    ├── main.rs                      # 入口：初始化 tracing、连接池、注册路由、绑定端口
    ├── config.rs                    # 从环境变量读取配置
    ├── router.rs                    # 路由注册：/api/v1/collect, /api/v1/agent/*, /api/v1/admin/*
    ├── db.rs                        # 数据库连接池初始化（sqlx::MySqlPool）
    ├── model/
    │   ├── mod.rs
    │   ├── agent.rs                 # Agent struct
    │   ├── session.rs               # Session struct
    │   ├── message.rs               # Message struct
    │   ├── code_edit.rs             # CodeEdit struct
    │   ├── action_event.rs          # ActionEvent struct
    │   └── daily_stat.rs            # DailyStat struct
    ├── handler/
    │   ├── mod.rs
    │   ├── collect.rs               # POST /api/v1/collect 处理器（暂时返回 mock 200）
    │   ├── agent.rs                 # Agent 注册/配置 处理器
    │   └── admin.rs                 # Admin 查询接口（暂时空实现）
    └── error.rs                     # 统一错误类型 + IntoResponse
```

**具体步骤：**
1. `cargo init backend` 初始化项目
2. 配置 `Cargo.toml` — Axum + sqlx (MySQL) + tower-http (CORS/compression)
3. 编写 `.env.example` — 数据库连接、盐值、JWT 密钥
4. 编写 `migrations/001_init.sql` — 全部 6 张表的 DDL
5. 实现 `config.rs` — `BackendConfig` 从环境变量读取
6. 实现 `db.rs` — `create_pool()` 返回 `MySqlPool`
7. 实现 `model/` — 所有数据模型的 struct + sqlx::FromRow 派生
8. 实现 `error.rs` — `ApiError` enum 实现 `IntoResponse`
9. 实现 `router.rs` — 挂载所有路由
10. 实现 `main.rs` — 启动 HTTP 服务

**验收标准：**
- `cargo build` 编译通过
- `cargo run` 启动后 `curl http://localhost:8080/api/v1/collect` 返回 200
- 数据库迁移成功执行（6 张表全部创建）
- 健康检查端点可用

---

### 1.3 管理后台项目初始化

**目标：** Vue3 + Element Plus 项目骨架，路由、API 封装、布局框架就绪。

**文件列表：**
```
admin-ui/
├── package.json
├── vite.config.ts
├── tsconfig.json
├── index.html
└── src/
    ├── main.ts                      # 入口：注册 Element Plus、Router、全局样式
    ├── App.vue                      # 根组件（layout）
    ├── router/
    │   └── index.ts                 # 路由定义：/dashboard, /conversations, /edits, /events, /agents
    ├── api/
    │   ├── client.ts                # Axios 实例（baseURL、拦截器、错误处理）
    │   ├── dashboard.ts             # 仪表盘 API
    │   ├── conversations.ts         # 对话列表 API
    │   ├── edits.ts                 # 编辑记录 API
    │   └── agents.ts               # 客户端管理 API
    ├── types/
    │   └── index.ts                 # TypeScript 类型定义（与后端数据模型对应）
    ├── layouts/
    │   └── MainLayout.vue           # 主布局：侧边栏 + Header + 内容区
    └── pages/
        ├── Dashboard.vue            # 占位页面
        ├── Conversations.vue        # 占位页面
        ├── CodeEdits.vue            # 占位页面
        ├── Events.vue               # 占位页面
        └── Agents.vue               # 占位页面
```

**具体步骤：**
1. `npm create vite@latest admin-ui -- --template vue-ts` 创建项目
2. 安装依赖：`element-plus`, `vue-router`, `axios`, `pinia`, `dayjs`
3. 配置 `vite.config.ts` — 代理 `/api` 到后端 `localhost:8080`
4. 实现 `types/index.ts` — 完整的数据模型类型定义
5. 实现 `api/client.ts` — Axios 实例 + 请求/响应拦截
6. 实现 `router/index.ts` — 5 个路由
7. 实现 `layouts/MainLayout.vue` — el-container + el-menu 侧边栏
8. 创建 5 个占位页面（仅标题文字）
9. 配置 `main.ts` — use router, use Element Plus

**验收标准：**
- `npm run dev` 启动无报错
- 浏览器访问 5 个路由均可正常切换
- 侧边栏导航正常工作
- API client 拦截器就绪（通过 console.log 验证）

---

## Phase 2：Agent 核心 — Claude Code 采集器（预计 3～4 天）

### 2.1 Collector Trait + 数据模型

**目标：** 定义采集器接口标准和内部数据结构。

**文件列表：**
```
agent/src/
├── collector/
│   ├── mod.rs                      # Collector trait + RawEvent enum + ToolType enum
│   └── claude.rs                   # ClaudeCodeCollector struct（先空壳）
└── engine/
    └── mod.rs                      # Engine struct（先空壳）
```

**具体步骤：**
1. 实现 `collector/mod.rs`：
   - `ToolType` 枚举：`ClaudeCode`, `Trae`
   - `RawEvent` 枚举：`Conversation(ConversationRecord)`, `CodeEdit(CodeEditRecord)`, `Action(ActionEvent)`, `Session(SessionRecord)`
   - `Collector` trait：`name()`, `is_installed()`, `is_running()`, `collect_incremental()`, `reset_cursor()`
2. 更新 `config.rs` — `tools` 字段解析为 `Vec<ToolType>`
3. 创建 `engine/mod.rs` — `Engine` 结构体持有所有 Collector 实例

**验收标准：**
- 编译通过
- trait 定义清晰，可被外部模块引用

---

### 2.2 Claude Code 日志解析器

**目标：** 实现 Claude Code 的 history.jsonl、sessions/*.json、projects/*/*.jsonl 增量解析。

**文件列表：**
```
agent/src/collector/
├── claude.rs                       # ClaudeCodeCollector 完整实现
└── claude/
    ├── mod.rs                      # 子模块入口
    ├── history.rs                  # history.jsonl 增量解析器
    ├── session.rs                  # sessions/<pid>.json 解析器
    └── conversation.rs             # projects/<hash>/<session>.jsonl 解析器
```

**具体步骤：**
1. 实现 `claude/history.rs`：
   - `HistoryEntry` struct（display, timestamp, project, sessionId）
   - `HistoryParser` struct 持有 `file_path: PathBuf` + `last_offset: u64`
   - `parse_incremental(&mut self) -> Result<Vec<HistoryEntry>>`：从 last_offset 处开始读取新行
   - 单元测试：用 fixtures/claude/history_sample.jsonl 验证
2. 实现 `claude/session.rs`：
   - `SessionMeta` struct（pid, sessionId, cwd, startedAt, version, status）
   - `parse_current_sessions(base_dir: &Path) -> Result<Vec<SessionMeta>>`
   - 遍历 `~/.claude/sessions/` 目录下所有 `*.json`
3. 实现 `claude/conversation.rs`：
   - `ConversationEvent` struct（完整字段映射）
   - `parse_session_jsonl(path: &Path, last_offset: &mut u64) -> Result<Vec<ConversationEvent>>`
   - 按 session_id 聚合事件为对话记录
4. 实现 `claude.rs` — `ClaudeCodeCollector`：
   - 实现 `Collector` trait
   - `is_installed()` — 检查 `~/.claude/` 是否存在
   - `is_running()` — 通过 sysinfo 检测进程名包含 "claude" 的进程
   - `collect_incremental()` — 依次调用 history/session/conversation 解析器
   - 内部维护 `ClaudeCursor { history_offset, session_cursors: HashMap<String, u64> }` 持久化到 sled
5. 准备测试 fixtures：
   - `agent/tests/fixtures/claude/history_sample.jsonl`
   - `agent/tests/fixtures/claude/session_sample.json`
   - `agent/tests/fixtures/claude/conversation_sample.jsonl`

**验收标准：**
- `cargo test` 所有单测通过，覆盖率 ≥ 80%
- 用本机真实 `~/.claude/` 目录测试，正确解析出对话记录
- 增量解析正确（连续两次调用不返回重复数据）

---

### 2.3 文件监控集成（可选）

**目标：** 通过 notify 实时监听项目目录文件变更。

**文件列表：**
```
agent/src/collector/
└── file_watcher.rs                 # FileWatcher struct
```

**具体步骤：**
1. 实现 `FileWatcher` 结构体：
   - `start(paths: Vec<PathBuf>)` — 使用 notify 的 `RecommendedWatcher`
   - 通过 tokio channel 发送 `FileChangeEvent { path, kind, timestamp }`
   - 事件去重：同一文件在 1 秒内的多次变化合并为一条
2. 集成到 Engine：可选启动，通过配置开关控制

**验收标准：**
- 修改测试项目中的文件，1 秒内收到事件
- 去重逻辑正确

---

### 2.4 Trae 数据采集器

**目标：** 实现 Trae IDE 的 AI 对话数据采集，从 workspaceStorage 的 state.vscdb（SQLite Key-Value 库）中增量提取会话元信息。

**数据源说明：**

```
C:\Users\<user>\AppData\Roaming\Trae\User\
├── workspaceStorage/
│   └── <hash>/                      # 每个工作区一个目录
│       ├── state.vscdb              # SQLite K-V 数据库（核心数据源）
│       ├── state.vscdb.backup       # 备份文件
│       └── workspace.json           # 项目路径映射（如: file:///d%3A/Project/xxx）
└── globalStorage/
    ├── state.vscdb                  # 全局 K-V 数据库（模型配置等）
    └── storage.json                 # 全局存储元信息
```

**state.vscdb 中可采集的 AI 相关 Key：**

| Key | 内容 | 采集用途 |
|-----|------|---------|
| `memento/icube-ai-agent-storage` | 会话列表 + sessionId | 获取所有历史会话 ID 列表 |
| `icube_session_agent_map` | sessionId → agent 类型映射 | 分辨 builder / dev_agent / solo_agent |
| `{userId}_ai-chat:sessionRelation:modelMap` | sessionId → 模型名映射 | 获取每会话使用的模型 |
| `icube-ai-agent-storage-input-history` | 用户输入历史（含文本 + 文件引用） | 提取 user prompt 文本和引用文件路径 |
| `ChatStore` | UI 状态（对话轮次高度等） | 辅助统计对话轮次数 |
| `currentAgentData_{userId}` | 当前 Agent 配置 | 获取 Agent 名称、类型、工具列表 |
| `workspace.json`（独立文件） | 项目文件夹路径 | 获取项目路径信息 |

**可采集字段 vs Claude Code 对比：**

| 字段 | Claude Code 采集 | Trae 采集 | 说明 |
|------|:---:|:---:|------|
| 对话统计（daily_count/model/tokens） | ✓ | △ | Trae 无精确 token 统计，可从模型+轮次估算 |
| 用户输入内容 | ✓ | ✓ | 从 input-history 提取 |
| 助手回复内容 | ✓ | ✗ | Trae 仅云端存储，本地未留存 |
| 代码变更（diff skeleton） | ✓ | ✓ | 从 Git 快照 before→after tag diff 提取 |
| 代码接受/拒绝行为 | ✓ | △ | 从 toolcall tag 推断文件变更，无显式 accept/reject 标记 |
| 项目路径 | ✓ | ✓ | 从 workspace.json 获取 |
| 会话记录（session 元信息） | ✓ | ✓ | sessionId / agent 类型 / 模型 |
| Git 分支 | ✓ | ✗ | 快照为独立 Git 仓库，无源仓库分支信息 |

**文件列表：**
```
agent/src/collector/
├── trae.rs                          # TraeCollector struct + Collector trait 实现
└── trae/
    ├── mod.rs                       # 子模块入口
    ├── workspace.rs                 # workspaceStorage 目录遍历 + 工作区发现
    ├── vscdb.rs                     # state.vscdb 增量读取（SQLite K-V 查询）
    ├── snapshot.rs                  # Git 快照解析（tags diff 提取代码变更）
    └── parser.rs                    # JSON 数据解析 + 标准化为 RawEvent
```

**具体步骤：**
1. 实现 `trae/workspace.rs`：
   - `discover_workspaces(base_dir: &Path) -> Result<Vec<WorkspaceInfo>>`
   - 遍历 `workspaceStorage/` 下所有目录，读取 `workspace.json` 获取项目路径
   - `WorkspaceInfo { hash: String, project_path: String, vscdb_path: PathBuf, snapshot_path: PathBuf }`
   - 过滤：只返回 vscdb 存在且最近 N 天有修改的工作区
2. 实现 `trae/vscdb.rs`：
   - `VscDbReader` struct 持有 `rusqlite::Connection` + 各 key 的 `last_read_timestamp`
   - 使用 `rusqlite` crate 读取 SQLite 数据库（只读模式，不影响 Trae 正常运行）
   - `read_key_incremental(key: &str) -> Result<Vec<u8>>` — 读取指定 key 的 value
   - 比较 value 的修改时间戳判断是否有新数据
   - 出错时（数据库被 Trae 锁定）直接返回空，不影响其他采集
3. 实现 `trae/snapshot.rs`：
   - `SnapshotReader` struct，使用 `git2` crate 打开 snapshot Git 仓库
   - `list_chain_tags(repo: &Repository) -> Result<Vec<TagInfo>>` — 列出 `chain-start-*` / `before-chat-turn-*` / `after-chat-turn-*` / `toolcall-*` tags
   - `get_turn_diff(repo: &Repository, before_tag: &str, after_tag: &str) -> Result<DiffSkeleton>` — 获取单轮对话的代码变更
   - `get_toolcall_files(repo: &Repository, tag: &str) -> Result<Vec<ChangedFile>>` — 获取单次工具调用的变更文件列表
   - 增量：通过 sled 记录已处理的 tag 列表，仅处理新 tag
   - 出错时（仓库损坏/不存在）返回空，warn 日志
4. 实现 `trae/parser.rs`：
   - `parse_session_list(value: &[u8]) -> Result<Vec<SessionMeta>>` — 解析会话列表
   - `parse_session_agent_map(value: &[u8]) -> Result<HashMap<String, String>>` — 解析会话→Agent 映射
   - `parse_model_map(value: &[u8]) -> Result<HashMap<String, HashMap<String, String>>>` — 解析会话→模型映射
   - `parse_input_history(value: &[u8]) -> Result<Vec<UserInput>>` — 解析用户输入历史
   - `parse_agent_data(value: &[u8]) -> Result<AgentInfo>` — 解析当前 Agent 配置
   - `parse_workspace_json(path: &Path) -> Result<String>` — 解析 workspace.json 获取项目路径
   - 所有解析函数返回标准化的 `RawEvent`（Conversation / CodeEdit / Session）
5. 实现 `trae.rs` — `TraeCollector`：
   - 实现 `Collector` trait
   - `name()` → `"trae"`
   - `is_installed()` — 检查 Trae 数据目录是否存在
   - `is_running()` — sysinfo 检测进程名包含 "Trae" 的进程
   - `collect_incremental()` — 遍历所有工作区 → 读取 vscdb → 解析 snapshot → 生成 RawEvent
   - 内部维护 `TraeCursor { workspace_hashes: HashMap<String, WorkspaceCursor> }` 持久化到 sled
   - 每个工作区的 cursor 记录 vscdb 各 key 和 snapshot tags 的最后读取状态
6. 注册到 Engine：
   - `ToolType::Trae` 已在枚举中定义，无需修改
   - Engine 根据 config.tools 配置创建 TraeCollector 实例
7. 准备测试 fixtures：
   - `agent/tests/fixtures/trae/state_sample.vscdb` — 模拟 vscdb 数据
   - `agent/tests/fixtures/trae/workspace_sample.json` — 模拟 workspace.json
   - `agent/tests/fixtures/trae/snapshot_sample/` — 模拟 Git 快照仓库（含 tags）

**依赖变更：**
- `agent/Cargo.toml` 添加 `rusqlite = { version = "0.31", features = ["bundled"] }`（bundled 模式，静态链接 SQLite，不依赖系统库）
- `agent/Cargo.toml` 添加 `git2 = { version = "0.18" }`（git 快照解析，读取 snapshot tags 和 diff）

**验收标准：**
- `cargo test` 所有单测通过，覆盖率 ≥ 80%
- 用本机真实 Trae 数据目录测试，正确解析出会话记录 + 用户输入 + 代码 diff
- 增量解析正确（连续两次调用不返回重复数据）
- Git 快照增量提取：仅处理新 tags，不重复生成同一条 diff
- 数据库被 Trae 锁定时不会崩溃（空返回 + warn 日志）
- 不修改 vscdb 文件和 snapshot 仓库（只读模式）

---

## Phase 3：Agent 核心 — 上报 & 服务化（预计 2～3 天）

### 3.1 去重 & 聚合引擎

**目标：** 将采集到的原始事件进行去重和聚合，减少上报量。

**文件列表：**
```
agent/src/engine/
├── mod.rs                          # Engine::run() 主循环
├── dedup.rs                        # 去重逻辑
└── aggregator.rs                   # 会话聚合
```

**具体步骤：**
1. 实现 `dedup.rs`：
   - `ContentHash` — 对事件关键字段做 SHA256，用于去重比较
   - `DedupFilter` — 基于 LRU 缓存（最近 1000 条 hash）的去重器
2. 实现 `aggregator.rs`：
   - 同一 session_id 的事件合并为一条 `AggregatedSession`
   - 合并规则：同一文件多次编辑只保留最终 diff，messages 按 seq 排序
3. 实现 `Engine::run()`：
   - 循环：遍历所有 Collector → collect_incremental → 去重 → 聚合 → 写入 sled 缓冲
   - 采集间隔由 config.interval_secs 控制

**验收标准：**
- 重复事件被正确过滤
- 同一会话的多次编辑被聚合为一条记录

---

### 3.2 本地缓冲队列

**目标：** 基于 sled 实现可靠本地缓冲，断网不丢数据。

**文件列表：**
```
agent/src/reporter/
├── mod.rs                          # Reporter trait
└── queue.rs                        # LocalQueue struct
```

**具体步骤：**
1. 实现 `queue.rs`：
   - `LocalQueue::new(path: &Path) -> Result<Self>` — 打开 sled db
   - `push(batch: Vec<AggregatedEvent>) -> Result<()>` — 序列化为 JSON 写入 sled
   - `pop_batch(max_size: usize) -> Result<Vec<AggregatedEvent>>` — FIFO 批量取出
   - `len() -> usize` — 当前队列长度
   - `clear_sent(upto: u64) -> Result<()>` — 删除已成功上报的记录
   - 队列上限 10000 条（超出时丢弃最旧数据并记录 error 日志）

**验收标准：**
- 写入 100 条 → 取出 100 条，顺序正确
- 模拟进程重启后，队列数据不丢失
- 超出上限时正确丢弃旧数据

---

### 3.3 HTTP 上报

**目标：** 批量上报到后端，支持压缩和重试。

**文件列表：**
```
agent/src/reporter/
└── http.rs                         # HttpReporter struct
```

**具体步骤：**
1. 实现 `http.rs`：
   - `HttpReporter::new(config: ServerConfig) -> Self`
   - `report(batch: Vec<AggregatedEvent>) -> Result<ReportResponse>`
   - 请求体：`CollectionPayload` JSON → gzip 压缩 → POST
   - 重试策略：失败后 1s / 2s / 4s / 8s / 16s 退避，最多 5 次
   - 非 200/409/422 的响应码视为临时错误，触发重试
   - 4xx 响应（除 429 外）不重试，记录 error 并丢弃

**验收标准：**
- 单测通过（mock server）
- 手动启动后端验证端到端上报成功

---

### 3.4 采集主循环整合

**目标：** 将采集→去重→聚合→缓冲→上报串成完整流程。

**文件列表：**
```
agent/src/
├── main.rs                          # run 命令：启动 Engine 主循环
└── engine/mod.rs                    # 完整的主循环逻辑
```

**具体步骤：**
1. 实现 Engine 主循环：
   - 启动时：加载配置 → 根据 tools 创建 Collector 实例 → 打开 sled 缓冲 → 注册 Agent（POST /api/v1/agent/register）
   - 采集 tick（每 interval_secs）：运行所有 Collector → 去重 + 聚合 → push 到队列
   - 上报 tick（独立线程）：从队列 pop → HTTP 上报 → 成功则 clear_sent
   - 关闭信号（Ctrl+C / Service Stop）：flush 队列 → 保存 cursor → 退出

**验收标准：**
- `agent.exe run` 前台运行，日志输出采集和上报流程
- 停止后重启，cursor 从上次位置继续

---

## Phase 4：后端 — 数据接入 & 存储（预计 2～3 天）

### 4.1 采集数据接入

**目标：** `POST /api/v1/collect` 完整实现，接收数据、校验、写入 MySQL。

**文件列表：**
```
backend/src/
├── handler/collect.rs              # 采集接口完整实现
├── service/
│   ├── mod.rs
│   ├── collect.rs                  # 采集业务逻辑（校验→写入）
│   └── agent.rs                    # Agent 注册逻辑
└── store/
    ├── mod.rs
    ├── session.rs                  # Session 表操作
    ├── message.rs                  # Message 表操作
    ├── code_edit.rs                # CodeEdit 表操作
    ├── action_event.rs             # ActionEvent 表操作
    └── agent.rs                    # Agent 表操作
```

**具体步骤：**
1. 实现 `handler/collect.rs`：
   - 接收 `CollectionPayload` JSON
   - 校验：agent_id 是否已注册、sequence 是否递增（去重）
   - 调用 `service/collect.rs` 处理
2. 实现 `service/collect.rs`：
   - 遍历 events 数组，按 event_type 分发到不同的 store 方法
   - 使用事务批量写入
   - 返回 `{ accepted: N, rejected: M }`
3. 实现 `store/` 下各文件的数据访问层（sqlx 参数化查询）
4. 实现限流中间件：同一 agent_id 每分钟最多 1 次请求

**验收标准：**
- curl 发送模拟数据，数据库正确写入
- 限流中间件生效，超频返回 429
- 事务回滚正确（部分写入失败不产生脏数据）

---

### 4.2 Agent 注册 & 配置下发

**目标：** 实现 Agent 注册和配置获取接口。

**文件列表：**
```
backend/src/
├── handler/agent.rs                # POST /register, GET /config
└── service/agent.rs                # 注册逻辑 + api_key 生成
```

**具体步骤：**
1. `POST /api/v1/agent/register`：
   - 校验 agent_id 是否合法
   - 生成 api_key（HMAC-SHA256）
   - 写入 agents 表
   - 返回 api_key + 默认配置
2. `GET /api/v1/agent/config?agent_id=xxx`：
   - 校验 agent_id 存在
   - 返回该 Agent 的采集配置（可从数据库读取，初始为默认值）

**验收标准：**
- 新增 agent 成功注册并获取 api_key
- 重复注册返回已有 api_key

---

## Phase 5：后端 — 脱敏 & 管理 API（预计 3～4 天）

### 5.1 脱敏模块

**目标：** 对入库数据进行脱敏处理。

**文件列表：**
```
backend/src/
├── desensitize/
│   ├── mod.rs                      # Desensitizer trait + 路由
│   ├── path.rs                     # 文件路径脱敏：C:\Users\<user>\Project\... → <user>/Project/...
│   ├── diff.rs                     # Diff 脱敏：基于 tree-sitter AST，字面量→<str>/<num>
│   └── content.rs                  # 对话内容：只存 SHA256 + 前200字摘要
```

**具体步骤：**
1. 实现 `path.rs`：正则替换绝对路径中的用户名
2. 实现 `diff.rs`：使用 tree-sitter 解析 diff 中的代码，替换字符串/数字字面量
3. 实现 `content.rs`：`sha256(content)` + 截断前 200 字符
4. 串联到 `service/collect.rs` 中，写入数据库前调用

**验收标准：**
- 单测覆盖率 ≥ 95%
- 各类敏感数据脱敏后不包含原始信息

---

### 5.2 管理端查询 API

**目标：** 为管理后台提供数据查询接口。

**文件列表：**
```
backend/src/
├── handler/admin.rs                # Dashboard / 列表 / 导出 处理器
└── service/admin.rs                # 查询逻辑 + 聚合 SQL
```

**具体步骤：**
1. `GET /api/v1/admin/dashboard`：返回总览统计（活跃 agent 数、今日对话量、token 消耗、接受率）
2. `GET /api/v1/admin/conversations`：分页查询对话记录，支持按时间/工具/模型/agent 筛选
3. `GET /api/v1/admin/edits`：分页查询代码编辑记录
4. `GET /api/v1/admin/events`：分页查询行为事件
5. `GET /api/v1/admin/daily-stats`：每日聚合统计（供图表使用）
6. `GET /api/v1/admin/export`：CSV 导出（数据流式返回）

**验收标准：**
- 所有接口返回正确的分页数据结构
- 筛选条件生效
- 导出文件格式正确

---

## Phase 6：管理后台 UI（预计 4～5 天）

### 6.1 仪表盘

**目标：** 首页概览——统计卡片 + 趋势图表 + 模型分布。

**文件列表：**
```
admin-ui/src/
├── pages/
│   └── Dashboard.vue               # 仪表盘完整实现
├── components/
│   ├── StatCard.vue                # 统计卡片组件
│   ├── TrendChart.vue              # 折线图（ECharts）
│   └── PieChart.vue                # 饼图
└── hooks/
    └── usePolling.ts               # 定时刷新 Hook（30s）
```

**具体步骤：**
1. 安装 `echarts` + `vue-echarts`
2. 实现 `StatCard.vue`：图标 + 数值 + 标题
3. 实现 `TrendChart.vue`：封装 ECharts 折线图
4. 实现 `PieChart.vue`：封装 ECharts 饼图
5. 实现 `Dashboard.vue`：
   - 顶部 4 个 StatCard（活跃客户端、今日对话、token 消耗、接受率）
   - 中部：每日对话趋势折线图（近 7 天）
   - 中部：模型使用占比饼图
   - 底部：代码接受率趋势（近 7 天）
6. 实现 `usePolling.ts` — 30 秒自动刷新

---

### 6.2 对话记录列表 & 详情

**目标：** 表格展示对话记录，支持筛选和详情查看。

**文件列表：**
```
admin-ui/src/
├── pages/
│   ├── Conversations.vue           # 对话列表页
│   └── ConversationDetail.vue      # 对话详情页
└── components/
    ├── FilterBar.vue               # 通用筛选栏
    └── DataTable.vue               # 通用分页表格
```

**具体步骤：**
1. 实现 `FilterBar.vue`：时间范围选择器、工具下拉、模型下拉、搜索框
2. 实现 `DataTable.vue`：el-table + el-pagination 封装
3. 实现 `Conversations.vue`：FilterBar + DataTable，列：时间、工具、模型、消息数、token 量
4. 实现 `ConversationDetail.vue`：对话消息列表（时序展示），每条显示角色、摘要、token、时间

---

### 6.3 代码编辑 & 行为事件

**目标：** 代码编辑记录表格 + Diff 查看器 + 行为事件时间线。

**文件列表：**
```
admin-ui/src/
├── pages/
│   ├── CodeEdits.vue               # 代码编辑列表
│   └── Events.vue                  # 行为事件列表
└── components/
    ├── DiffViewer.vue              # Diff 渲染组件
    └── Timeline.vue                # 时间线组件
```

**具体步骤：**
1. 实现 `DiffViewer.vue`：等宽字体、添加行绿色背景、删除行红色背景
2. 实现 `Timeline.vue`：el-timeline 封装
3. 实现 `CodeEdits.vue`：表格（文件路径、语言、编辑类型、是否接受、时间）+ 展开行显示 DiffViewer
4. 实现 `Events.vue`：Timeline 展示行为序列（接受/拒绝/修改/忽略/重新生成）

---

### 6.4 客户端管理

**目标：** Agent 列表，查看在线状态和配置。

**文件列表：**
```
admin-ui/src/pages/
└── Agents.vue                      # 客户端管理页
```

**具体步骤：**
1. 表格展示所有 Agent：ID、主机名哈希、OS、在线状态、最后上报时间、版本
2. 操作：查看详情、下发配置

---

## Phase 7：生产加固（预计 3～5 天）

### 7.1 Agent 加固

- [ ] 实现 Agent 静默安装器（NSIS 脚本）
- [ ] 自更新逻辑：轮询服务端获取最新版本下载链接
- [ ] 配置热更新：定期拉取服务端配置
- [ ] 异常恢复：panic hook + 自动重启

### 7.2 后端加固

- [ ] 请求签名验证（HMAC）
- [ ] 日志脱敏切面（确保不打印敏感字段）
- [ ] 慢查询监控（tracing span）
- [ ] 分区表自动创建（按天分区）
- [ ] 数据归档脚本（N 天前原始数据迁移到归档表）

### 7.3 管理后台加固

- [ ] 登录页面 + JWT 鉴权
- [ ] 路由守卫（未登录跳转登录页）
- [ ] 权限控制（管理员/查看者角色）
- [ ] 响应式布局（1920px / 1366px 适配）

### 7.4 部署

- [ ] Docker Compose 编排（MySQL + Backend + Admin UI）
- [ ] Nginx 反向代理配置
- [ ] HTTPS 证书配置
- [ ] 后端 API 文档（Swagger / OpenAPI）

---

## 附录：技术栈速查

| 层 | 技术 | 版本 |
|----|------|------|
| Agent | Rust + tokio + reqwest + sled | edition 2024 |
| 后端 | Rust + Axum + sqlx + tower-http | edition 2024 |
| 数据库 | MySQL | 8.0+ |
| 前端 | Vue3 + Element Plus + ECharts + Axios | Vue 3.5+ |
| 构建 | Vite | 6.x |
| 部署 | Docker Compose + Nginx | - |
