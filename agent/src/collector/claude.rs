use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::collector::claude::conversation::{self, ConversationEvent, ConversationParser};
use crate::collector::claude::history::{self, HistoryParser};
use crate::collector::claude::session::{self, SessionMeta};
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
    /// history.jsonl 已读到的字节偏移
    history_offset: u64,
    /// 每个 session jsonl 文件的已读偏移 (文件路径字符串 → offset)
    session_cursors: HashMap<String, u64>,
}

impl ClaudeCodeCollector {
    pub fn new(claude_home: PathBuf) -> Self {
        Self {
            claude_home,
            cursor: ClaudeCursor::default(),
        }
    }

    /// 获取 history.jsonl 路径
    fn history_path(&self) -> PathBuf {
        self.claude_home.join("history.jsonl")
    }

    /// 获取 sessions 目录路径
    fn sessions_dir(&self) -> PathBuf {
        self.claude_home.join("sessions")
    }

    /// 获取 projects 目录路径
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
        // 通过 sysinfo 检测进程名包含 "claude" 的进程
        use sysinfo::System;
        let sys = System::new_all();
        for proc in sys.processes_by_exact_name("claude") {
            return true;
        }
        // 也检查包含 "Claude" 的进程名
        for proc in sys.processes().values() {
            if proc
                .name()
                .to_lowercase()
                .contains("claude")
            {
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
                // 每个 history entry 产生一个 SessionRecord
                let session = SessionRecord {
                    session_id: entry.session_id.clone(),
                    pid: None,
                    cwd: Some(entry.project.clone()),
                    started_at: entry.parsed_timestamp().unwrap_or_default(),
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
                let prev_offset = self
                    .cursor
                    .session_cursors
                    .get(&path_key)
                    .copied()
                    .unwrap_or(0);
                conv_parser.set_offset(prev_offset);

                if let Ok(conv_events) = conv_parser.parse_incremental() {
                    if !conv_events.is_empty() {
                        // 将 conversation JSONL 事件聚合为结构化记录
                        let aggregated = aggregate_conversation_events(&conv_events);
                        events.extend(aggregated);
                    }
                    self.cursor
                        .session_cursors
                        .insert(path_key, conv_parser.offset());
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
///
/// 按事件类型分类：
/// - type="message" → 聚合为 ConversationRecord
/// - type="code_edit" → CodeEditRecord
/// - type="action" → ActionEvent
fn aggregate_conversation_events(events: &[ConversationEvent]) -> Vec<RawEvent> {
    let mut result = Vec::new();

    // 按 session 分组的 messages（从事件流中提取）
    let mut messages: Vec<MessageRecord> = Vec::new();

    for event in events {
        let event_type = event.event_type.as_deref().unwrap_or("unknown");

        match event_type {
            "message" => {
                let seq = event.seq.unwrap_or(0);
                let role = match event.role.as_deref() {
                    Some("assistant") => MessageRole::Assistant,
                    _ => MessageRole::User,
                };
                let msg = MessageRecord {
                    seq,
                    role,
                    content: event.content.clone().unwrap_or_default(),
                    model: event.model.clone(),
                    tokens_input: event.tokens.as_ref().and_then(|t| t.input),
                    tokens_output: event.tokens.as_ref().and_then(|t| t.output),
                    timestamp: event.parsed_timestamp().unwrap_or_default(),
                };
                messages.push(msg);
            }

            "code_edit" => {
                if let Some(ref edit) = event.file_edit {
                    let code_edit = CodeEditRecord {
                        session_id: String::new(), // 由调用方填充
                        file_path: edit.path.clone().unwrap_or_default(),
                        edit_type: match edit.edit_type.as_deref() {
                            Some("create") => EditType::Create,
                            Some("delete") => EditType::Delete,
                            Some("rename") => EditType::Rename,
                            _ => EditType::Modify,
                        },
                        lines_added: edit.lines_added,
                        lines_removed: edit.lines_removed,
                        diff_content: edit.diff.clone(),
                        timestamp: event.parsed_timestamp().unwrap_or_default(),
                    };
                    result.push(RawEvent::CodeEdit(code_edit));
                }
            }

            "action" => {
                if let Some(ref action) = event.action {
                    let action_event = ActionEvent {
                        session_id: String::new(), // 由调用方填充
                        action: match action.action_type.as_deref() {
                            Some("accept") => ActionType::Accept,
                            Some("reject") => ActionType::Reject,
                            Some("modify") => ActionType::Modify,
                            Some("ignore") => ActionType::Ignore,
                            Some("regenerate") => ActionType::Regenerate,
                            Some("copy") => ActionType::Copy,
                            Some("paste") => ActionType::Paste,
                            _ => return, // 未知 action 类型，跳过
                        },
                        message_seq: action.message_seq,
                        file_path: action.file_path.clone(),
                        extra: None,
                        timestamp: event.parsed_timestamp().unwrap_or_default(),
                    };
                    result.push(RawEvent::Action(action_event));
                }
            }

            _ => {
                log::trace!("忽略未知事件类型: {}", event_type);
            }
        }
    }

    // 如果有 messages，构造一个 ConversationRecord
    // 注意：实际 session_id 和 project_path 需要从其他来源填充
    if !messages.is_empty() {
        messages.sort_by_key(|m| m.seq);
        let started_at = messages.first().map(|m| m.timestamp).unwrap_or_default();
        let ended_at = messages.last().map(|m| m.timestamp);
        let model = messages
            .iter()
            .find(|m| m.model.is_some())
            .and_then(|m| m.model.clone());

        let conversation = ConversationRecord {
            session_id: String::new(),
            project_path_hash: String::new(),
            project_path: String::new(),
            git_branch: None,
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
