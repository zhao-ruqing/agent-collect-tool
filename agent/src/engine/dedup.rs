// 去重逻辑：基于内容哈希 + LRU 缓存，避免重复事件
use lru::LruCache;
use sha2::{Digest, Sha256};
use std::num::NonZeroUsize;
use crate::collector::RawEvent;

/// 去重过滤器：基于 LRU 缓存最近 N 条事件的内容哈希
pub struct DedupFilter {
    /// LRU 缓存，key=内容哈希, value=时间戳（保留，备用）
    cache: LruCache<String, u64>,
}

impl DedupFilter {
    /// 创建去重过滤器
    /// capacity: 最大缓存条目数（最近 N 条）
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: LruCache::new(NonZeroUsize::new(capacity).unwrap_or(NonZeroUsize::new(1000).unwrap())),
        }
    }

    /// 检查事件是否重复
    /// 返回 true 表示是重复事件（应过滤），false 表示是新事件
    pub fn is_duplicate(&mut self, event: &RawEvent) -> bool {
        let hash = content_hash(event);
        if self.cache.contains(&hash) {
            true
        } else {
            let now = current_timestamp_ms();
            self.cache.put(hash, now);
            false
        }
    }

    /// 批量过滤，返回非重复事件
    pub fn filter(&mut self, events: Vec<RawEvent>) -> Vec<RawEvent> {
        events
            .into_iter()
            .filter(|e| !self.is_duplicate(e))
            .collect()
    }

    /// 当前缓存大小
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// 清空缓存
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

/// 计算事件的 SHA256 内容哈希（用于去重）
fn content_hash(event: &RawEvent) -> String {
    let mut hasher = Sha256::new();
    match event {
        RawEvent::Conversation(conv) => {
            hasher.update(conv.session_id.as_bytes());
            hasher.update(conv.started_at.timestamp_millis().to_string().as_bytes());
            for msg in &conv.messages {
                hasher.update(msg.seq.to_string().as_bytes());
                hasher.update(msg.timestamp.timestamp_millis().to_string().as_bytes());
            }
        }
        RawEvent::CodeEdit(edit) => {
            hasher.update(edit.session_id.as_bytes());
            hasher.update(edit.file_path.as_bytes());
            hasher.update(edit.timestamp.timestamp_millis().to_string().as_bytes());
        }
        RawEvent::Action(action) => {
            hasher.update(action.session_id.as_bytes());
            hasher.update(format!("{:?}", action.action).as_bytes());
            hasher.update(action.timestamp.timestamp_millis().to_string().as_bytes());
        }
        RawEvent::Session(sess) => {
            hasher.update(sess.session_id.as_bytes());
            hasher.update(sess.started_at.timestamp_millis().to_string().as_bytes());
            if let Some(ref v) = sess.version {
                hasher.update(v.as_bytes());
            }
        }
    }
    format!("{:x}", hasher.finalize())
}

fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collector::{ConversationRecord, MessageRecord};
    use chrono::Utc;

    #[test]
    fn test_dedup_filter_new_events() {
        let mut filter = DedupFilter::new(100);
        let event = create_test_event("session-1");
        assert!(!filter.is_duplicate(&event));
        assert_eq!(filter.len(), 1);
    }

    #[test]
    fn test_dedup_filter_duplicate() {
        let mut filter = DedupFilter::new(100);
        let event = create_test_event("session-1");
        assert!(!filter.is_duplicate(&event));
        // 相同事件再次检测
        assert!(filter.is_duplicate(&event));
        assert_eq!(filter.len(), 1);
    }

    #[test]
    fn test_dedup_filter_different_events() {
        let mut filter = DedupFilter::new(100);
        let event1 = create_test_event("session-1");
        let event2 = create_test_event("session-2");
        assert!(!filter.is_duplicate(&event1));
        assert!(!filter.is_duplicate(&event2));
        assert_eq!(filter.len(), 2);
    }

    #[test]
    fn test_batch_filter() {
        let mut filter = DedupFilter::new(100);
        let events = vec![
            create_test_event("s1"),
            create_test_event("s2"),
            create_test_event("s1"), // 重复
            create_test_event("s3"),
        ];
        let filtered = filter.filter(events);
        assert_eq!(filtered.len(), 3);
    }

    fn create_test_event(session_id: &str) -> RawEvent {
        RawEvent::Conversation(ConversationRecord {
            session_id: session_id.to_string(),
            project_path_hash: String::new(),
            project_path: "/test".to_string(),
            git_branch: None,
            started_at: Utc::now(),
            ended_at: None,
            messages: vec![MessageRecord {
                seq: 1,
                role: crate::collector::MessageRole::User,
                content: "hello".to_string(),
                model: Some("claude-4".to_string()),
                tokens_input: Some(10),
                tokens_output: None,
                timestamp: Utc::now(),
            }],
            model: Some("claude-4".to_string()),
            tool: crate::collector::ToolType::ClaudeCode,
        })
    }
}
