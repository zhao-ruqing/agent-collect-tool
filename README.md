# Agent Collect Tool

> AI 编程工具元信息静默收集系统：Rust Agent（Windows Service）→ Rust Axum 后端 → MySQL → Vue3 管理后台
> 支持**员工自查个人AI使用数据**、**管理员全局管控全员使用数据**，双角色权限隔离，静默无感采集

---

## AI 操作提示

> **开始任何开发前，AI 必须先阅读 `docs/` 目录下的所有文档：**
>
> - [DEVELOPMENT-PLAN.md](docs/DEVELOPMENT-PLAN.md) — 渐进式开发计划（Phase 1~7）
> - [AI-DEVELOPMENT-STANDARDS.md](docs/AI-DEVELOPMENT-STANDARDS.md) — 操作规范、代码风格、Git 工作流
> - [AI-Development-Progress/](docs/AI-Development-Progress/) — 开发进度记录（每次提交后更新）
> - 代码中所有注释均为中文（包括 Rust 代码）
> - 日志仅供参考，具体开发进度必须查看代码,一切以代码为准

---

## 技术栈

| 层     | 技术                                                 |
| ------ | ---------------------------------------------------- |
| Agent  | Rust (tokio, reqwest, sled, notify, windows-service) |
| 后端   | Rust (Axum, sqlx, tower-http)                        |
| 数据库 | MySQL 8.0                                            |
| 前端   | Vue3 + Element Plus + ECharts                        |

---

## 核心架构

```
Rust Agent (Windows Service)
  ├─ 增量解析 Claude Code JSONL 日志
  ├─ 去重 → 聚合 → sled 本地缓冲
  └─ HTTP 批量上报 (gzip) 自动携带设备身份信息
        │
        ▼
Rust Axum 后端
  ├─ 数据校验 + 限流 + 身份绑定
  ├─ 脱敏（路径/diff/内容）
  ├─ 角色权限鉴权分发
  └─ MySQL 分层存储人员使用数据
        │
        ▼
Vue3 权限分离管理后台
  ├─ 员工端：仅查看个人AI使用统计、对话记录、代码操作数据
  └─ 管理员端：查看全员数据、部门统计、排行分析、设备管理、数据导出
```

---

## 用户角色与权限体系

### 1. 普通员工

- 自主登录后台，仅展示**本人**所有AI编程工具使用数据
- 支持查看个人使用趋势、Token消耗、代码修改行为、会话记录
- 可自主修改个人昵称、绑定所属部门，无任何他人数据查看权限

### 2. 系统管理员

- 拥有全平台最高查看与管理权限
- 查看全体员工AI使用明细、批量数据统计、部门维度报表
- 人员信息编辑、设备解绑、数据风控预警、使用行为管控
- 支持全局数据筛选、排序、导出归档

### 身份识别方案

1. Agent 客户端自动读取**Windows系统登录账号+设备MAC唯一标识**完成自动绑定
2. 管理员后台批量录入真实姓名、工号、部门，完成账号实名映射
3. 无需员工手动填写信息，静默安装无感绑定身份

---

## 采集数据

对 **Claude Code CLI** 和 **Trae IDE** 进行静默元信息采集：

| 数据类别 | Claude Code | Trae | 说明 |
|---------|:---:|:---:|------|
| 对话统计（次数/模型/tokens） | ✓ | △ | Trae 无精确 token 数，从模型+轮次估算 |
| 用户输入内容 | ✓ | ✓ | 从日志 / vscdb input-history 提取 |
| 助手回复内容 | ✓ | ✗ | Trae 仅云端存储，本地未留存 |
| 代码变更（diff skeleton） | ✓ | ✓ | 从 Git 快照 `before→after` tag diff 提取 |
| 代码接受/拒绝行为 | ✓ | △ | 从 toolcall tag 推断，无显式 accept/reject 标记 |
| 项目路径（脱敏） | ✓ | ✓ | 从 session / workspace.json 获取 |
| Git 分支 | ✓ | ✗ | 快照为独立 Git 仓库，无源仓库分支信息 |
| 会话元信息（agent/模型） | ✓ | ✓ | 从 vscdb session map 获取 |

**Claude Code 数据源：** `~/.claude/history.jsonl` + `sessions/*.json` + `projects/<hash>/*.jsonl`  
**Trae 数据源：** `%APPDATA%/Trae/User/workspaceStorage/<hash>/state.vscdb`（SQLite K-V 库）+ `ModularData/ai-agent/snapshot/<sessionId>/`（Git 快照）

> 详细数据源格式和逆向分析见 [DEVELOPMENT-PLAN.md](docs/DEVELOPMENT-PLAN.md) Phase 2.2（Claude Code）和 Phase 2.4（Trae）

---

## 快速启动

```bash
# 后端
cd backend && sqlx migrate run && cargo run

# 管理后台(员工端+管理端同系统，登录区分权限)
cd admin-ui && npm install && npm run dev

# Agent（编译 + 安装为 Windows 系统服务，静默后台运行）
cd agent && cargo build --release
agent.exe install && agent.exe start
```

---

## 项目结构

```
agent-collect-tool/
├── README.md
├── agent/                    # Rust Agent 客户端（Windows 静默服务）
├── backend/                  # Rust Axum 权限后端、数据接收、鉴权逻辑
├── admin-ui/                 # Vue3 管理后台（员工自查+管理员管控合一）
└── docs/
    ├── DEVELOPMENT-PLAN.md
    ├── AI-DEVELOPMENT-STANDARDS.md
    └── AI-Development-Progress/
```

---

## 核心特色

1. **无感部署**：客户端打包为Windows系统服务，开机自启，后台静默运行，无界面无打扰
2. **权限隔离**：严格区分员工/管理员视图，数据权限隔离，保障隐私与管理需求
3. **本地缓冲**：断网自动缓存采集数据，联网批量补发，不丢失使用记录
4. **数据脱敏**：自动脱敏项目绝对路径、核心代码内容，只保留行为统计信息
5. **低资源占用**：Rust 编写客户端与服务端，内存占用极低，不影响开发工作
