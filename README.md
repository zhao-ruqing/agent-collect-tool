# Agent Collect Tool

> AI 编程工具元信息静默收集系统：Rust Agent（Windows Service）→ Rust Axum 后端 → MySQL → Vue3 管理后台

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
  └─ HTTP 批量上报 (gzip)
        │
        ▼
Rust Axum 后端
  ├─ 数据校验 + 限流
  ├─ 脱敏（路径/diff/内容）
  └─ MySQL 存储
        │
        ▼
Vue3 管理后台
  ├─ 仪表盘（趋势图/占比图）
  ├─ 对话记录查询
  ├─ 代码编辑浏览
  └─ 行为事件分析
```

---

## 采集数据

对 **Claude Code CLI** 和 **Trae** 进行静默元信息采集：

- 对话统计：每日对话次数、模型、token 消耗
- 代码变更：修改文件、diff 骨架、编辑类型
- 代码接受：接受/拒绝/修改等行为事件
- 会话上下文：项目路径（脱敏）、Git 分支、工具版本

> 已验证的 Claude Code 数据源详见 [DEVELOPMENT-PLAN.md](docs/DEVELOPMENT-PLAN.md) Phase 2.2

---

## 快速启动

```bash
# 数据库
docker run -d --name mysql-collect -p 3306:3306 \
  -e MYSQL_ROOT_PASSWORD=root \
  -e MYSQL_DATABASE=agent_collect \
  mysql:8.0

# 后端
cd backend && sqlx migrate run && cargo run

# 管理后台
cd admin-ui && npm install && npm run dev

# Agent（编译 + 安装为 Windows 服务）
cd agent && cargo build --release
agent.exe install && agent.exe start
```

---

## 项目结构

```
agent-collect-tool/
├── README.md
├── agent/                    # Rust Agent（Windows 服务）
├── backend/                  # Rust Axum 后端
├── admin-ui/                 # Vue3 管理后台
└── docs/
    ├── DEVELOPMENT-PLAN.md
    ├── AI-DEVELOPMENT-STANDARDS.md
    └── AI-Development-Progress/
```
