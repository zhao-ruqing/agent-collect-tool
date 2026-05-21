// Trae IDE 数据采集器子模块
//
// 数据源：
// - workspaceStorage/<hash>/state.vscdb（SQLite K-V 库）
// - ModularData/ai-agent/snapshot/<sessionId>/（Git 快照仓库）

pub mod collector;
pub mod parser;
pub mod snapshot;
pub mod vscdb;
pub mod workspace;

pub use collector::TraeCollector;
