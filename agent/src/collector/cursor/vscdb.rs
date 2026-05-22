// Cursor state.vscdb SQLite K-V 读取器
// 读取 workspaceStorage 和 globalStorage 中的 AI 相关 key

use anyhow::Result;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub struct VscDbReader {
    db_path: PathBuf,
}

/// 从 vscdb 中读取的 Cursor AI 数据
#[derive(Debug, Default)]
pub struct CursorVscDbData {
    /// composer.composerData — 所有 Composer 会话元数据
    pub composer_data: Option<Vec<u8>>,
    /// aiService.generations — AI 代码生成事件
    pub generations: Option<Vec<u8>>,
    /// aiService.prompts — 用户提示词
    pub prompts: Option<Vec<u8>>,
}

impl VscDbReader {
    pub fn new(db_path: &Path) -> Self {
        Self {
            db_path: db_path.to_path_buf(),
        }
    }

    /// 读取 Cursor AI 相关 key（只读模式）
    pub fn read_ai_data(&self) -> Result<CursorVscDbData> {
        if !self.db_path.exists() {
            return Ok(CursorVscDbData::default());
        }

        let conn = match Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("无法打开 Cursor vscdb (可能被锁定): {:?} — {}", self.db_path, e);
                return Ok(CursorVscDbData::default());
            }
        };

        let mut data = CursorVscDbData::default();

        data.composer_data = read_key(&conn, "composer.composerData");
        data.generations = read_key(&conn, "aiService.generations");
        data.prompts = read_key(&conn, "aiService.prompts");

        Ok(data)
    }
}

/// 全局存储的 Cursor AI 数据
#[derive(Debug, Default)]
pub struct GlobalVscDbData {
    /// composer.composerHeaders — 跨项目 composer 头部
    pub composer_headers: Option<Vec<u8>>,
    /// agentLayout.shared.v6 — Agent 布局配置（含模型信息）
    pub agent_layout: Option<Vec<u8>>,
}

impl VscDbReader {
    /// 读取 globalStorage 中的 AI 相关 key
    pub fn read_global_data(&self) -> Result<GlobalVscDbData> {
        if !self.db_path.exists() {
            return Ok(GlobalVscDbData::default());
        }

        let conn = match Connection::open_with_flags(
            &self.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        ) {
            Ok(c) => c,
            Err(e) => {
                log::warn!("无法打开 Cursor globalStorage vscdb: {:?} — {}", self.db_path, e);
                return Ok(GlobalVscDbData::default());
            }
        };

        let mut data = GlobalVscDbData::default();

        data.composer_headers = read_key(&conn, "composer.composerHeaders");
        data.agent_layout = read_key(&conn, "agentLayout.shared.v6");

        Ok(data)
    }
}

/// 读取指定 key 的值（TEXT 类型，转为 Vec<u8>）
fn read_key(conn: &Connection, key: &str) -> Option<Vec<u8>> {
    let result: rusqlite::Result<String> = conn.query_row(
        "SELECT value FROM ItemTable WHERE key = ?1",
        [key],
        |row| row.get(0),
    );
    match result {
        Ok(s) => Some(s.into_bytes()),
        Err(e) => {
            log::debug!("Cursor vscdb 读取 key '{}' 失败: {}", key, e);
            None
        }
    }
}
