// Cursor IDE 数据采集器子模块
//
// 数据源:
// - workspaceStorage/<hash>/state.vscdb (SQLite K-V 库, 会话元数据)
// - ~/.cursor/projects/<project>/agent-transcripts/<id>/<id>.jsonl (对话转录)

pub mod collector;
pub mod parser;
pub mod vscdb;

pub use collector::CursorCollector;
