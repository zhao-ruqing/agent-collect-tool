use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

/// conversation JSONL 中的单条事件（匹配 Claude Code 实际格式）
#[derive(Debug, Clone, Deserialize)]
pub struct ConversationEvent {
    /// 事件类型: "user", "assistant", "file-history-snapshot", etc.
    #[serde(rename = "type")]
    pub event_type: String,

    /// 事件 UUID
    #[serde(default)]
    pub uuid: Option<String>,

    /// 父事件 UUID（对话树）
    #[serde(default)]
    pub parent_uuid: Option<String>,

    /// Session ID
    #[serde(default, rename = "sessionId")]
    pub session_id: Option<String>,

    /// 工作目录
    #[serde(default)]
    pub cwd: Option<String>,

    /// Git 分支名
    #[serde(default, rename = "gitBranch")]
    pub git_branch: Option<String>,

    /// 时间戳
    #[serde(default)]
    pub timestamp: Option<String>,

    /// 消息体（user / assistant 消息）
    #[serde(default)]
    pub message: Option<MessageBlock>,

    /// 文件操作结果（工具调用结果中的 create / update / delete）
    #[serde(default, rename = "toolUseResult")]
    pub tool_use_result: Option<ToolUseResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageBlock {
    /// 角色: "user", "assistant"
    #[serde(default)]
    pub role: Option<String>,

    /// 消息内容：user 消息中是字符串，assistant 消息中是内容块数组
    #[serde(default)]
    pub content: serde_json::Value,

    /// 模型名称（仅 assistant）
    #[serde(default)]
    pub model: Option<String>,

    /// Token 用量（仅 assistant）
    #[serde(default)]
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UsageInfo {
    #[serde(default, rename = "input_tokens")]
    pub input_tokens: Option<i32>,
    #[serde(default, rename = "output_tokens")]
    pub output_tokens: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolUseResult {
    /// 操作类型: "create", "update", "delete"
    #[serde(rename = "type")]
    pub result_type: Option<String>,

    /// 文件路径
    #[serde(default, rename = "filePath")]
    pub file_path: Option<String>,

    /// 新文件内容（create/update 时）
    #[serde(default)]
    pub content: Option<String>,

    /// 结构化补丁（update 时）
    #[serde(default, rename = "structuredPatch")]
    pub structured_patch: Option<Vec<StructuredPatch>>,

    /// 原文件内容（update 时）
    #[serde(default, rename = "originalFile")]
    pub original_file: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StructuredPatch {
    #[serde(default)]
    pub old_start: Option<i32>,
    #[serde(default)]
    pub old_lines: Option<i32>,
    #[serde(default)]
    pub new_start: Option<i32>,
    #[serde(default)]
    pub new_lines: Option<i32>,
    #[serde(default)]
    pub lines: Option<Vec<String>>,
}

impl ConversationEvent {
    /// 解析时间戳
    pub fn parsed_timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamp
            .as_deref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
    }

    /// 从嵌套的 content 中提取纯文本（用于哈希/摘要）
    pub fn content_text(&self) -> String {
        match &self.message {
            Some(msg) => extract_content_text(&msg.content),
            None => String::new(),
        }
    }
}

/// 判断内容块是否为工具交互噪音（tool_use / tool_result / thinking 等）
fn is_noise_block(block: &serde_json::Value) -> bool {
    matches!(
        block.get("type").and_then(|v| v.as_str()),
        Some("tool_use") | Some("tool_result") | Some("thinking")
    )
}

/// 从 serde_json::Value 提取文本内容（仅提取用户输入和 AI 实际输出）
fn extract_content_text(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => {
            // 字符串可能是序列化后的 JSON 工具结果数组，尝试检测并过滤
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
                if parsed.is_array() {
                    return extract_content_text(&parsed);
                }
            }
            // 如果是纯工具结果 JSON 字符串（以 [{" 开头），视为噪音
            let trimmed = s.trim();
            if trimmed.starts_with("[{") && (trimmed.contains("\"tool_use_id\"") || trimmed.contains("\"type\":\"tool_result\"") || trimmed.contains("\"type\":\"tool_use\"")) {
                return String::new();
            }
            s.clone()
        }
        serde_json::Value::Array(arr) => {
            // 如果全部是工具交互块，整条消息视为噪音
            if !arr.is_empty() && arr.iter().all(|b| is_noise_block(b)) {
                return String::new();
            }
            let parts: Vec<String> = arr
                .iter()
                .filter_map(|block| {
                    if is_noise_block(block) {
                        return None;
                    }
                    // 仅提取 text 类型内容块
                    block.get("text")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .collect();
            if parts.is_empty() {
                String::new()
            } else {
                parts.join("\n")
            }
        }
        _ => String::new(),
    }
}

impl MessageBlock {
    /// 是否为 user 类型
    pub fn is_user(&self) -> bool {
        self.role.as_deref() == Some("user")
    }

    /// 是否为 assistant 类型
    pub fn is_assistant(&self) -> bool {
        self.role.as_deref() == Some("assistant")
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
/// 支持两种结构:
/// 1. projects/<project_hash>/<session_id>.jsonl
/// 2. projects/<project_hash>/<session_id>/<session_id>.jsonl
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
            if session_path.is_file() && session_path.extension().map_or(false, |ext| ext == "jsonl") {
                // 结构 1: projects/<hash>/<session>.jsonl
                session_files.push(session_path);
            } else if session_path.is_dir() {
                // 结构 2: projects/<hash>/<session_id>/<session_id>.jsonl
                if let Ok(sub_entries) = std::fs::read_dir(&session_path) {
                    for sub_entry in sub_entries {
                        if let Ok(sub) = sub_entry {
                            let sub_path = sub.path();
                            if sub_path.extension().map_or(false, |ext| ext == "jsonl") {
                                session_files.push(sub_path);
                            }
                        }
                    }
                }
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
            r#"{{"type":"user","message":{{"role":"user","content":"Hello","model":"claude-sonnet-4-6","usage":{{"input_tokens":0,"output_tokens":0}}}}}}"#
        ).unwrap();
        writeln!(
            f,
            r#"{{"type":"assistant","message":{{"role":"assistant","content":"Hi!","model":"claude-sonnet-4-6","usage":{{"input_tokens":10,"output_tokens":5}}}}}}"#
        ).unwrap();

        let mut parser = ConversationParser::new(tmp.clone());
        let events = parser.parse_incremental().unwrap();
        assert_eq!(events.len(), 2);
        // 新格式：role 在 message 对象内
        assert_eq!(events[0].message.as_ref().and_then(|m| m.role.as_deref()), Some("user"));
        assert_eq!(events[1].message.as_ref().and_then(|m| m.role.as_deref()), Some("assistant"));

        // 增量无新数据
        let events = parser.parse_incremental().unwrap();
        assert_eq!(events.len(), 0);

        std::fs::remove_file(&tmp).ok();
    }

    #[test]
    fn test_parse_tool_use_result() {
        let tmp = std::env::temp_dir().join("test_tool_use.jsonl");
        let mut f = std::fs::File::create(&tmp).unwrap();
        writeln!(
            f,
            r#"{{"type":"user","message":{{"role":"user","content":"create a file"}},"toolUseResult":{{"type":"create","filePath":"src/main.rs","content":"fn main() {{}}"}},"timestamp":"2024-01-01T10:00:00.000Z"}}"#
        ).unwrap();

        let mut parser = ConversationParser::new(tmp.clone());
        let events = parser.parse_incremental().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(&events[0].event_type, "user");

        let tool = events[0].tool_use_result.as_ref().unwrap();
        assert_eq!(tool.result_type.as_deref(), Some("create"));
        assert_eq!(tool.file_path.as_deref(), Some("src/main.rs"));

        std::fs::remove_file(&tmp).ok();
    }
}
