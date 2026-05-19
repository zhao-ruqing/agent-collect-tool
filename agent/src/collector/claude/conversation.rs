use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

/// conversation JSONL 中的单条事件
#[derive(Debug, Clone, Deserialize)]
pub struct ConversationEvent {
    /// 事件类型: "message", "code_edit", "action"
    #[serde(rename = "type")]
    pub event_type: Option<String>,

    /// 角色: "user", "assistant"
    #[serde(default)]
    pub role: Option<String>,

    /// 消息/事件内容
    #[serde(default)]
    pub content: Option<String>,

    /// 使用的模型
    #[serde(default)]
    pub model: Option<String>,

    /// Token 统计
    #[serde(default)]
    pub tokens: Option<TokenInfo>,

    /// 时间戳
    #[serde(default)]
    pub timestamp: Option<String>,

    /// 序号（消息顺序）
    #[serde(default)]
    pub seq: Option<u32>,

    /// 消息 ID
    #[serde(default)]
    pub id: Option<String>,

    /// 父消息 ID（对话树）
    #[serde(default)]
    pub parent_id: Option<String>,

    /// 文件编辑信息（当 type = "code_edit" 时）
    #[serde(default)]
    pub file_edit: Option<FileEditInfo>,

    /// 行为事件信息（当 type = "action" 时）
    #[serde(default)]
    pub action: Option<ActionInfo>,

    /// 其他未知字段
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenInfo {
    #[serde(default)]
    pub input: Option<i32>,
    #[serde(default)]
    pub output: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileEditInfo {
    /// 文件路径
    #[serde(default)]
    pub path: Option<String>,
    /// 编辑类型
    #[serde(default)]
    pub edit_type: Option<String>,
    /// 新增行数
    #[serde(default)]
    pub lines_added: Option<i32>,
    /// 删除行数
    #[serde(default)]
    pub lines_removed: Option<i32>,
    /// diff 内容
    #[serde(default)]
    pub diff: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ActionInfo {
    /// 行为类型: accept, reject, modify, ignore, regenerate, copy, paste
    #[serde(default)]
    pub action_type: Option<String>,
    /// 相关消息序号
    #[serde(default)]
    pub message_seq: Option<u32>,
    /// 相关文件路径
    #[serde(default)]
    pub file_path: Option<String>,
}

impl ConversationEvent {
    /// 解析时间戳
    pub fn parsed_timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamp
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
    }
}

/// conversation JSONL 增量解析器
///
/// 解析 projects/<hash>/<session>.jsonl 文件
pub struct ConversationParser {
    file_path: PathBuf,
    /// 上次读取的字节偏移
    last_offset: u64,
}

impl ConversationParser {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            last_offset: 0,
        }
    }

    /// 获取当前偏移
    pub fn offset(&self) -> u64 {
        self.last_offset
    }

    /// 从指定偏移恢复
    pub fn set_offset(&mut self, offset: u64) {
        self.last_offset = offset;
    }

    /// 增量解析
    pub fn parse_incremental(&mut self) -> Result<Vec<ConversationEvent>> {
        if !self.file_path.exists() {
            return Ok(vec![]);
        }

        let file = match std::fs::File::open(&self.file_path) {
            Ok(f) => f,
            Err(_) => return Ok(vec![]),
        };

        let file_size = file.metadata()?.len();

        if file_size <= self.last_offset {
            if file_size < self.last_offset {
                self.last_offset = 0;
            }
            return Ok(vec![]);
        }

        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::Start(self.last_offset))?;

        let mut events = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<ConversationEvent>(trimmed) {
                Ok(event) => events.push(event),
                Err(e) => {
                    log::debug!(
                        "解析 conversation 行失败: {} — 行内容: {}",
                        e,
                        &trimmed[..trimmed.len().min(200)]
                    );
                }
            }
        }

        self.last_offset = file_size;

        log::debug!(
            "{:?}: 增量解析 {} 条事件",
            self.file_path.file_name(),
            events.len()
        );

        Ok(events)
    }
}

/// 扫描 projects 目录，找到所有 session JSONL 文件
///
/// 结构: projects/<project_hash>/<session_id>.jsonl
pub fn scan_project_sessions(projects_dir: &std::path::Path) -> Result<Vec<PathBuf>> {
    if !projects_dir.exists() || !projects_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut session_files = Vec::new();

    for project_entry in std::fs::read_dir(projects_dir)? {
        let project_entry = match project_entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let project_dir = project_entry.path();
        if !project_dir.is_dir() {
            continue;
        }

        for session_entry in std::fs::read_dir(&project_dir)? {
            let session_entry = match session_entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let session_path = session_entry.path();
            if session_path.extension().map_or(false, |ext| ext == "jsonl") {
                session_files.push(session_path);
            }
        }
    }

    Ok(session_files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_conversation_incremental() {
        let mut tmp = std::env::temp_dir();
        tmp.push("test_conversation.jsonl");

        let mut f = std::fs::File::create(&tmp).unwrap();
        writeln!(
            f,
            r#"{{"type":"message","role":"user","content":"Hello","model":"claude-sonnet-4-6","tokens":{{"input":0,"output":0}},"timestamp":"2024-01-01T10:00:00.000Z","seq":1}}"#
        ).unwrap();
        writeln!(
            f,
            r#"{{"type":"message","role":"assistant","content":"Hi!","model":"claude-sonnet-4-6","tokens":{{"input":10,"output":5}},"timestamp":"2024-01-01T10:00:01.000Z","seq":2}}"#
        ).unwrap();

        let mut parser = ConversationParser::new(tmp.clone());
        let events = parser.parse_incremental().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].role.as_deref(), Some("user"));
        assert_eq!(events[1].role.as_deref(), Some("assistant"));

        // 增量无新数据
        let events = parser.parse_incremental().unwrap();
        assert_eq!(events.len(), 0);

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_parse_file_edit() {
        let tmp = std::env::temp_dir().join("test_edit.jsonl");
        let mut f = std::fs::File::create(&tmp).unwrap();
        writeln!(
            f,
            r#"{{"type":"code_edit","file_edit":{{"path":"src/main.rs","edit_type":"modify","lines_added":5,"lines_removed":2,"diff":"- old\\n+ new"}},"timestamp":"2024-01-01T10:00:00.000Z"}}"#
        ).unwrap();

        let mut parser = ConversationParser::new(tmp.clone());
        let events = parser.parse_incremental().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type.as_deref(), Some("code_edit"));

        let edit = events[0].file_edit.as_ref().unwrap();
        assert_eq!(edit.path.as_deref(), Some("src/main.rs"));
        assert_eq!(edit.lines_added, Some(5));

        std::fs::remove_file(&tmp).ok();
    }
}
