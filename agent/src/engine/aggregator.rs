// 会话聚合器：将同一 session 的多个 CodeEdit/Action 事件合并到对应 Conversation 中
use crate::collector::RawEvent;
use std::collections::HashMap;

/// 按 session_id 聚合事件
/// 同一 session 内的所有事件归为一组，CodeEdit 和 Action 作为 Conversation 的子事件
#[derive(Debug, Default)]
pub struct EventAggregator {
    /// 内部缓冲区：按 session_id 暂存事件
    buffer: HashMap<String, SessionBucket>,
}

#[derive(Debug, Default)]
struct SessionBucket {
    pub conversation: Option<Box<RawEvent>>,
    pub edits: Vec<RawEvent>,
    pub actions: Vec<RawEvent>,
}

impl EventAggregator {
    pub fn new() -> Self {
        Self {
            buffer: HashMap::new(),
        }
    }

    /// 推入一个事件进行聚合
    pub fn push(&mut self, event: RawEvent) {
        let sid = event_session_id(&event);
        let bucket = self.buffer.entry(sid).or_default();

        match &event {
            RawEvent::Conversation(_) => {
                bucket.conversation = Some(Box::new(event));
            }
            RawEvent::CodeEdit(_) => {
                bucket.edits.push(event);
            }
            RawEvent::Action(_) => {
                bucket.actions.push(event);
            }
            RawEvent::Session(_) => {
                // Session 事件独立存在，不与其他事件合并
            }
        }
    }

    /// 对已完成的 session 进行合并，输出聚合后的事件列表
    /// completed_session_ids: 已知已结束的 session_id 列表
    pub fn flush_completed(&mut self, completed_session_ids: &[String]) -> Vec<RawEvent> {
        let mut result = Vec::new();

        for sid in completed_session_ids {
            if let Some(bucket) = self.buffer.remove(sid) {
                // 先输出 Conversation
                if let Some(conv) = bucket.conversation {
                    result.push(*conv);
                }
                // 再输出 CodeEdit 和 Action
                result.extend(bucket.edits);
                result.extend(bucket.actions);
            }
        }

        result
    }

    /// 强制刷新所有缓冲的事件（不等待 session 结束）
    pub fn flush_all(&mut self) -> Vec<RawEvent> {
        let mut result = Vec::new();

        let sids: Vec<String> = self.buffer.keys().cloned().collect();
        for sid in &sids {
            if let Some(bucket) = self.buffer.remove(sid) {
                if let Some(conv) = bucket.conversation {
                    result.push(*conv);
                }
                result.extend(bucket.edits);
                result.extend(bucket.actions);
            }
        }

        result
    }

    /// 当前缓冲区中的 session 数量
    pub fn pending_sessions(&self) -> usize {
        self.buffer.len()
    }
}

/// 从事件中提取 session_id
fn event_session_id(event: &RawEvent) -> String {
    match event {
        RawEvent::Conversation(c) => c.session_id.clone(),
        RawEvent::CodeEdit(e) => e.session_id.clone(),
        RawEvent::Action(a) => a.session_id.clone(),
        RawEvent::Session(s) => s.session_id.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::{
        ActionEvent, ActionType, CodeEditRecord, ConversationRecord, EditType,
        MessageRecord, MessageRole, ToolType,
    };
    use chrono::Utc;

    fn mk_conv(sid: &str) -> RawEvent {
        RawEvent::Conversation(ConversationRecord {
            session_id: sid.to_string(),
            project_path_hash: String::new(),
            project_path: "/proj".to_string(),
            git_branch: Some("main".to_string()),
            started_at: Utc::now(),
            ended_at: None,
            messages: vec![MessageRecord {
                seq: 1,
                role: MessageRole::User,
                content: "hi".to_string(),
                model: None,
                tokens_input: Some(5),
                tokens_output: None,
                timestamp: Utc::now(),
            }],
            model: Some("claude-4".to_string()),
            tool: ToolType::ClaudeCode,
        })
    }

    fn mk_edit(sid: &str, path: &str) -> RawEvent {
        RawEvent::CodeEdit(CodeEditRecord {
            session_id: sid.to_string(),
            file_path: path.to_string(),
            edit_type: EditType::Modify,
            lines_added: Some(3),
            lines_removed: Some(1),
            diff_content: None,
            timestamp: Utc::now(),
        })
    }

    fn mk_action(sid: &str, action: ActionType) -> RawEvent {
        RawEvent::Action(ActionEvent {
            session_id: sid.to_string(),
            action,
            message_seq: Some(1),
            file_path: None,
            extra: None,
            timestamp: Utc::now(),
        })
    }

    #[test]
    fn test_aggregate_single_session() {
        let mut agg = EventAggregator::new();
        agg.push(mk_conv("s1"));
        agg.push(mk_edit("s1", "src/main.rs"));
        agg.push(mk_action("s1", ActionType::Accept));

        let result = agg.flush_completed(&["s1".to_string()]);
        assert_eq!(result.len(), 3); // 1 Conv + 1 Edit + 1 Action
    }

    #[test]
    fn test_aggregate_multiple_sessions() {
        let mut agg = EventAggregator::new();
        agg.push(mk_conv("s1"));
        agg.push(mk_edit("s1", "a.rs"));
        agg.push(mk_conv("s2"));
        agg.push(mk_edit("s2", "b.rs"));

        let result = agg.flush_completed(&["s1".to_string()]);
        assert_eq!(result.len(), 2); // s1 的事件
        assert_eq!(agg.pending_sessions(), 1); // s2 还在缓冲区
    }

    #[test]
    fn test_flush_all() {
        let mut agg = EventAggregator::new();
        agg.push(mk_conv("s1"));
        agg.push(mk_conv("s2"));
        agg.push(mk_action("s1", ActionType::Reject));

        let result = agg.flush_all();
        assert_eq!(result.len(), 3);
        assert_eq!(agg.pending_sessions(), 0);
    }

    #[test]
    fn test_empty_aggregator() {
        let mut agg = EventAggregator::new();
        assert_eq!(agg.flush_all().len(), 0);
        assert_eq!(agg.flush_completed(&[]).len(), 0);
    }
}
