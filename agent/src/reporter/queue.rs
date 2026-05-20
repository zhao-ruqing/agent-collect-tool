// 本地缓冲队列：基于 sled 的可靠本地缓冲，断网不丢数据
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::Path;

/// 本地队列：持久化存储待上报事件，按序号排序
pub struct LocalQueue {
    db: Db,
    /// 当前最大序号（用于生成新的 key）
    next_seq: u64,
    /// 队列容量上限
    max_items: usize,
}

/// 队列条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    /// 序号
    pub seq: u64,
    /// 序列化的事件 JSON
    pub payload: Vec<u8>,
}

impl LocalQueue {
    /// 打开或创建本地缓冲队列
    pub fn open(path: &Path, max_items: usize) -> Result<Self> {
        let db = sled::open(path)
            .with_context(|| format!("打开 sled 数据库失败: {:?}", path))?;

        // 恢复 next_seq：遍历现有 key 找最大值
        let mut max_key: u64 = 0;
        let mut found = false;
        for item in db.iter() {
            let (key, _) = item.with_context(|| "读取 sled 条目失败")?;
            if let Ok(seq) = key_to_seq(&key) {
                if seq > max_key {
                    max_key = seq;
                }
                found = true;
            }
        }

        let next_seq = if found { max_key + 1 } else { 0 };

        Ok(Self {
            db,
            next_seq,
            max_items,
        })
    }

    /// 推入一批事件到队列
    pub fn push_batch(&mut self, events: Vec<Vec<u8>>) -> Result<()> {
        for payload in events {
            let entry = QueueEntry {
                seq: self.next_seq,
                payload,
            };
            let value = serde_json::to_vec(&entry)
                .with_context(|| "序列化 QueueEntry 失败")?;
            let key = seq_to_key(self.next_seq);
            self.db.insert(key.as_bytes(), value)
                .with_context(|| "写入 sled 失败")?;
            self.next_seq += 1;
        }

        // 刷盘
        self.db.flush().with_context(|| "sled flush 失败")?;

        // 超出上限时丢弃最旧数据
        let current_len = self.len();
        if current_len > self.max_items {
            let to_drop = current_len - self.max_items;
            self.drop_oldest(to_drop)?;
            log::error!("队列超出上限 {}，丢弃最旧 {} 条数据", self.max_items, to_drop);
        }

        Ok(())
    }

    /// 弹出最多 max_size 条事件（FIFO）
    pub fn pop_batch(&mut self, max_size: usize) -> Result<Vec<QueueEntry>> {
        let mut entries = Vec::new();

        for item in self.db.iter() {
            let (_key, value) = item.with_context(|| "读取 sled 条目失败")?;
            let entry: QueueEntry = serde_json::from_slice(&value)
                .with_context(|| "反序列化 QueueEntry 失败")?;
            entries.push(entry);

            if entries.len() >= max_size {
                break;
            }
        }

        // 按 seq 排序（sled 遍历顺序不保证严格有序）
        entries.sort_by_key(|e| e.seq);

        Ok(entries)
    }

    /// 删除已成功上报的条目（删除所有 seq <= upto 的记录）
    pub fn clear_sent(&mut self, max_seq: u64) -> Result<()> {
        let keys_to_remove: Vec<Vec<u8>> = self.db.iter()
            .filter_map(|item| {
                let (key, value) = item.ok()?;
                let entry: QueueEntry = serde_json::from_slice(&value).ok()?;
                if entry.seq <= max_seq {
                    Some(key.to_vec())
                } else {
                    None
                }
            })
            .collect();

        for key in keys_to_remove {
            self.db.remove(key)?;
        }
        self.db.flush()?;

        Ok(())
    }

    /// 当前队列长度
    pub fn len(&self) -> usize {
        self.db.len()
    }

    /// 队列是否为空
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 删除最旧的 count 条记录
    fn drop_oldest(&mut self, count: usize) -> Result<()> {
        // 收集所有 key，排序后删除最旧的
        let mut all_keys: Vec<u64> = self.db.iter()
            .filter_map(|item| {
                let (key, _) = item.ok()?;
                key_to_seq(&key).ok()
            })
            .collect();
        all_keys.sort();

        let to_remove: Vec<Vec<u8>> = all_keys.iter()
            .take(count)
            .map(|seq| seq_to_key(*seq).into_bytes())
            .collect();

        for key in to_remove {
            let _ = self.db.remove(key);
        }
        self.db.flush()?;

        Ok(())
    }
}

/// 序号 -> sled key
fn seq_to_key(seq: u64) -> String {
    format!("{:020}", seq)
}

/// sled key -> 序号
fn key_to_seq(key: &[u8]) -> Result<u64> {
    let s = std::str::from_utf8(key)
        .with_context(|| "key 不是有效的 UTF-8")?;
    s.parse::<u64>()
        .with_context(|| format!("key 不是有效的序号: {}", s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_push_and_pop() {
        let dir = TempDir::new().unwrap();
        let mut queue = LocalQueue::open(dir.path(), 100).unwrap();

        let events: Vec<Vec<u8>> = (0..5)
            .map(|i| format!("event-{}", i).into_bytes())
            .collect();
        queue.push_batch(events).unwrap();
        assert_eq!(queue.len(), 5);

        let popped = queue.pop_batch(3).unwrap();
        assert_eq!(popped.len(), 3);
        assert_eq!(queue.len(), 5); // pop_batch 不删除数据
    }

    #[test]
    fn test_clear_sent() {
        let dir = TempDir::new().unwrap();
        let mut queue = LocalQueue::open(dir.path(), 100).unwrap();

        let events: Vec<Vec<u8>> = (0..3)
            .map(|i| format!("event-{}", i).into_bytes())
            .collect();
        queue.push_batch(events).unwrap();
        assert_eq!(queue.len(), 3);

        // 清除 seq <= 1 的数据（即前2条）
        queue.clear_sent(1).unwrap();
        assert_eq!(queue.len(), 1); // 只剩 seq=2
    }

    #[test]
    fn test_persistence() {
        let dir = TempDir::new().unwrap();

        {
            let mut queue = LocalQueue::open(dir.path(), 100).unwrap();
            let events = vec![b"hello".to_vec()];
            queue.push_batch(events).unwrap();
        }

        // 重新打开，数据还在
        {
            let queue = LocalQueue::open(dir.path(), 100).unwrap();
            assert_eq!(queue.len(), 1);
        }
    }

    #[test]
    fn test_capacity_limit() {
        let dir = TempDir::new().unwrap();
        let mut queue = LocalQueue::open(dir.path(), 5).unwrap();

        let events: Vec<Vec<u8>> = (0..10)
            .map(|i| format!("event-{}", i).into_bytes())
            .collect();
        queue.push_batch(events).unwrap();

        // 最多保留 5 条，且是最新的
        assert_eq!(queue.len(), 5);
        let popped = queue.pop_batch(10).unwrap();
        let seqs: Vec<u64> = popped.iter().map(|e| e.seq).collect();
        assert!(seqs.iter().all(|&s| s >= 5)); // 最旧的 0~4 被丢弃
    }

    #[test]
    fn test_empty_queue() {
        let dir = TempDir::new().unwrap();
        let mut queue = LocalQueue::open(dir.path(), 100).unwrap();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
        assert_eq!(queue.pop_batch(10).unwrap().len(), 0);
    }
}
