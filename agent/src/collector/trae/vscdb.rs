//! state.vscdb SQLite K-V 增量读取器
//!
//! Trae 基于 VS Code，使用 SQLite 存储状态数据。
//! ItemTable 结构: key TEXT PRIMARY KEY, value BLOB
//!
//! 使用 rusqlite 只读模式打开，不干扰 Trae 正常运行。

use anyhow::Result;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// vscdb 读取器（只读模式）
pub struct VscDbReader {
    db_path: PathBuf,
}

/// 从 vscdb 中读取的 AI 相关数据
#[derive(Debug, Default)]
pub struct VscDbData {
    /// memento/icube-ai-agent-storage — 会话列表 JSON
    pub session_storage: Option<Vec<u8>>,
    /// icube_session_agent_map — sessionId→agent 类型 JSON
    pub session_agent_map: Option<Vec<u8>>,
    /// {userId}_ai-chat:sessionRelation:modelMap — sessionId→模型 JSON (key, value)
    pub model_map: Option<(String, Vec<u8>)>,
    /// icube-ai-agent-storage-input-history — 用户输入历史 JSON
    pub input_history: Option<Vec<u8>>,
    /// ChatStore — 对话 UI 状态 JSON
    pub chat_store: Option<Vec<u8>>,
    /// currentAgentData_{userId} — 当前 Agent 配置 JSON (key, value)
    pub agent_data: Option<(String, Vec<u8>)>,
}

impl VscDbReader {
    pub fn new(db_path: &Path) -> Self {
        Self {
            db_path: db_path.to_path_buf(),
        }
    }

    /// 读取所有 AI 相关的 key（只读，不修改数据库）
    pub fn read_ai_data(&self) -> Result<VscDbData> {
        if !self.db_path.exists() {
            return Ok(VscDbData::default());
        }

        let conn = match Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            Ok(c) => c,
            Err(e) => {
                // 数据库被 Trae 锁定时不崩溃，直接返回空
                log::warn!("无法打开 vscdb (可能被 Trae 锁定): {:?} — {}", self.db_path, e);
                return Ok(VscDbData::default());
            }
        };

        let mut data = VscDbData::default();

        // 固定 key 直接读取
        data.session_storage = read_key(&conn, "memento/icube-ai-agent-storage");
        data.session_agent_map = read_key(&conn, "icube_session_agent_map");
        data.input_history = read_key(&conn, "icube-ai-agent-storage-input-history");
        data.chat_store = read_key(&conn, "ChatStore");

        // 查找 {userId}_ai-chat:sessionRelation:modelMap（userId 动态）
        if let Ok(keys) = find_keys_like(&conn, "%\\_ai-chat:sessionRelation:modelMap") {
            if let Some(key) = keys.first() {
                if let Some(val) = read_key(&conn, key) {
                    data.model_map = Some((key.clone(), val));
                }
            }
        }

        // 查找 currentAgentData_{userId}
        if let Ok(keys) = find_keys_like(&conn, "currentAgentData\\_%") {
            if let Some(key) = keys.first() {
                if let Some(val) = read_key(&conn, key) {
                    data.agent_data = Some((key.clone(), val));
                }
            }
        }

        Ok(data)
    }
}

/// 读取指定 key 的值（返回 UTF-8 字节）
/// Trae 的 vscdb 中 value 列可能存为 TEXT 或 BLOB，统一转为 Vec<u8>
fn read_key(conn: &Connection, key: &str) -> Option<Vec<u8>> {
    let result: rusqlite::Result<String> = conn.query_row(
        "SELECT value FROM ItemTable WHERE key = ?1",
        [key],
        |row| row.get(0),
    );
    match result {
        Ok(s) => Some(s.into_bytes()),
        Err(e) => {
            log::debug!("读取 key '{}' 失败: {}", key, e);
            None
        }
    }
}

/// 按 LIKE 模式查找匹配的 key
fn find_keys_like(conn: &Connection, pattern: &str) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT key FROM ItemTable WHERE key LIKE ?1 ESCAPE '\\'")?;
    let keys: Vec<String> = stmt
        .query_map([pattern], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(keys)
}
