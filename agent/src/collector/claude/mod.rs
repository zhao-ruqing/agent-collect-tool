pub mod conversation;
pub mod history;
pub mod session;

use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use conversation::{ConversationEvent, ConversationParser};
use history::HistoryParser;
use crate::collector::{
    ActionEvent, ActionType, CodeEditRecord, Collector, ConversationRecord, EditType,
    MessageRecord, MessageRole, RawEvent, SessionRecord, SessionStatus, ToolType,
};

/// Claude Code 采集器
///
/// 负责增量解析 ~/.claude/ 目录下的：
/// - history.jsonl → 对话列表
/// - sessions/<pid>.json → 会话元信息
/// - projects/<hash>/<session>.jsonl → 对话详情、编辑、行为事件
pub struct ClaudeCodeCollector {
    /// Claude 数据目录（默认 ~/.claude/）
    claude_home: PathBuf,
    /// 采集游标：记录各文件的已读位置
    cursor: ClaudeCursor,
}

#[derive(Debug, Default, Clone)]
struct ClaudeCursor {
    history_offset: u64,
    session_cursors: HashMap<String, u64>,
}

impl ClaudeCodeCollector {
    pub fn new(claude_home: PathBuf) -> Self {
        Self {
            claude_home,
            cursor: ClaudeCursor::default(),
        }
    }

    pub fn new_with_default_path() -> Result<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("无法获取 HOME 目录"))?
            .join(".claude");
        Ok(Self::new(home))
    }

    /// 根据配置创建采集器
    /// history_path_override: 可选的自定义历史文件路径
    pub fn from_config(history_path_override: Option<String>) -> Result<Self> {
        if let Some(ref path) = history_path_override {
            let p = PathBuf::from(path);
            if let Some(parent) = p.parent() {
                Ok(Self::new(parent.to_path_buf()))
            } else {
                Ok(Self::new(p.clone()))
            }
        } else {
            Self::new_with_default_path()
        }
    }

    fn history_path(&self) -> PathBuf {
        self.claude_home.join("history.jsonl")
    }

    fn sessions_dir(&self) -> PathBuf {
        self.claude_home.join("sessions")
    }

    fn projects_dir(&self) -> PathBuf {
        self.claude_home.join("projects")
    }
}

impl Collector for ClaudeCodeCollector {
    fn name(&self) -> &str {
        "claude-code"
    }

    fn is_installed(&self) -> bool {
        self.claude_home.exists()
    }

    fn is_running(&self) -> bool {
        use sysinfo::System;
        let sys = System::new_all();
        for proc in sys.processes().values() {
            if proc.name().to_lowercase().contains("claude") {
                return true;
            }
        }
        false
    }

    fn collect_incremental(&mut self) -> Result<Vec<RawEvent>> {
        let mut events = Vec::new();

        // 1. 解析 history.jsonl 增量
        let mut history_parser = HistoryParser::new(self.history_path());
        history_parser.set_offset(self.cursor.history_offset);
        if let Ok(history_entries) = history_parser.parse_incremental() {
            for entry in &history_entries {
                let session = SessionRecord {
                    session_id: entry.session_id.clone(),
                    pid: None,
                    cwd: Some(entry.project.clone()),
                    started_at: entry.timestamp,
                    ended_at: None,
                    version: None,
                    status: SessionStatus::Active,
                };
                events.push(RawEvent::Session(session));
            }
            self.cursor.history_offset = history_parser.offset();
        }

        // 2. 解析 sessions/*.json
        if let Ok(sessions) = session::parse_sessions_dir(&self.sessions_dir()) {
            for meta in &sessions {
                let session = SessionRecord {
                    session_id: meta.session_id.clone(),
                    pid: meta.pid,
                    cwd: meta.cwd.clone(),
                    started_at: meta.parsed_started_at().unwrap_or_default(),
                    ended_at: None,
                    version: meta.version.clone(),
                    status: match meta.status.as_deref() {
                        Some("completed") => SessionStatus::Completed,
                        Some("abandoned") => SessionStatus::Abandoned,
                        _ => SessionStatus::Active,
                    },
                };
                events.push(RawEvent::Session(session));
            }
        }

        // 3. 解析 projects/*/*.jsonl 增量
        if let Ok(session_files) = conversation::scan_project_sessions(&self.projects_dir()) {
            for session_path in &session_files {
                let path_key = session_path.to_string_lossy().to_string();
                let mut conv_parser = ConversationParser::new(session_path.clone());
                let prev_offset = self.cursor.session_cursors.get(&path_key).copied().unwrap_or(0);
                conv_parser.set_offset(prev_offset);

                if let Ok(conv_events) = conv_parser.parse_incremental() {
                    if !conv_events.is_empty() {
                        let sid = session_path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("unknown");
                        let aggregated = aggregate_conversation_events(sid, &conv_events);
                        events.extend(aggregated);
                    }
                    self.cursor.session_cursors.insert(path_key, conv_parser.offset());
                }
            }
        }

        Ok(events)
    }

    fn reset_cursor(&mut self) -> Result<()> {
        self.cursor = ClaudeCursor::default();
        Ok(())
    }
}

/// 将 conversation JSONL 事件聚合为 RawEvent 列表
fn aggregate_conversation_events(session_id: &str, events: &[ConversationEvent]) -> Vec<RawEvent> {
    let mut result = Vec::new();
    let mut messages: Vec<MessageRecord> = Vec::new();
    let mut msg_seq: u32 = 0;

    // 从事件中提取上下文信息（cwd, git_branch）
    let first_event = events.first();
    let cwd = first_event.and_then(|e| e.cwd.clone()).unwrap_or_default();
    let git_branch = first_event.and_then(|e| e.git_branch.clone());

    for event in events {
        match event.event_type.as_str() {
            "user" | "assistant" => {
                let Some(ref msg_block) = event.message else { continue };

                // 跳过 meta/system 消息（如 local-command-caveat）
                let role = if msg_block.is_user() {
                    MessageRole::User
                } else if msg_block.is_assistant() {
                    MessageRole::Assistant
                } else {
                    continue;
                };

                // 跳过空内容（如纯 tool_use 的 assistant 消息）
                let content_text = event.content_text();
                if content_text.is_empty() && msg_block.is_assistant() {
                    // assistant 消息可能只有 tool_use 而没有 text，跳过
                    continue;
                }

                msg_seq += 1;
                let msg = MessageRecord {
                    seq: msg_seq,
                    role,
                    content: content_text,
                    model: msg_block.model.clone(),
                    tokens_input: msg_block.usage.as_ref().and_then(|u| u.input_tokens),
                    tokens_output: msg_block.usage.as_ref().and_then(|u| u.output_tokens),
                    timestamp: event.parsed_timestamp().unwrap_or_default(),
                };
                messages.push(msg);

                // 同时检查是否有文件操作（user 消息中嵌套 toolUseResult）
                if let Some(ref tool_result) = event.tool_use_result {
                    let edit_type = match tool_result.result_type.as_deref() {
                        Some("create") => EditType::Create,
                        Some("delete") => EditType::Delete,
                        Some("update") | Some("file_unchanged") => EditType::Modify,
                        _ => continue,
                    };

                    let (lines_added, lines_removed, diff_content) = if let Some(ref patches) = tool_result.structured_patch {
                        let added: i32 = patches.iter()
                            .filter_map(|p| p.lines.as_ref().map(|l| l.iter().filter(|line| line.starts_with('+')).count() as i32))
                            .sum();
                        let removed: i32 = patches.iter()
                            .filter_map(|p| p.lines.as_ref().map(|l| l.iter().filter(|line| line.starts_with('-')).count() as i32))
                            .sum();
                        let diff = patches.iter()
                            .filter_map(|p| p.lines.as_ref().map(|l| l.join("\n")))
                            .collect::<Vec<_>>()
                            .join("\n");
                        (Some(added), Some(removed), if diff.is_empty() { None } else { Some(diff) })
                    } else {
                        (None, None, None)
                    };

                    let code_edit = CodeEditRecord {
                        session_id: session_id.to_string(),
                        file_path: tool_result.file_path.clone().unwrap_or_default(),
                        edit_type,
                        lines_added,
                        lines_removed,
                        diff_content,
                        timestamp: event.parsed_timestamp().unwrap_or_default(),
                    };
                    result.push(RawEvent::CodeEdit(code_edit));
                }
            }

            _ => {
                // 忽略其他事件类型: file-history-snapshot, attachment, etc.
            }
        }
    }

    if !messages.is_empty() {
        messages.sort_by_key(|m| m.seq);
        let started_at = messages.first().map(|m| m.timestamp).unwrap_or_default();
        let ended_at = messages.last().map(|m| m.timestamp);
        let model = messages.iter().find(|m| m.model.is_some()).and_then(|m| m.model.clone());

        let conversation = ConversationRecord {
            session_id: session_id.to_string(),
            project_path_hash: String::new(),
            project_path: cwd,
            git_branch,
            started_at,
            ended_at,
            messages,
            model,
            tool: ToolType::ClaudeCode,
        };
        result.push(RawEvent::Conversation(conversation));
    }

    result
}
