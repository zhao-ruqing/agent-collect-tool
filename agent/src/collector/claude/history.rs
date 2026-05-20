use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

/// history.jsonl 中的单条记录
#[derive(Debug, Clone, Deserialize)]
pub struct HistoryEntry {
    /// 对话标题/首条提示
    pub display: String,
    /// 对话时间戳
    pub timestamp: String,
    /// 项目路径
    pub project: String,
    /// 会话 ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

impl HistoryEntry {
    /// 解析时间戳
    pub fn parsed_timestamp(&self) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.timestamp)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    }
}
/// history.jsonl 增量解析器
///
/// 记录文件读取偏移，每次调用只解析新增的行
pub struct HistoryParser {
    file_path: PathBuf,
    /// 上次读取到的字节偏移
    last_offset: u64,
}

impl HistoryParser {
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

    /// 增量解析：从上次 offset 处开始读取新增的行
    pub fn parse_incremental(&mut self) -> Result<Vec<HistoryEntry>> {
        let file = std::fs::File::open(&self.file_path)
            .with_context(|| format!("无法打开 history 文件: {:?}", self.file_path))?;

        let file_size = file.metadata()?.len();

        // 文件未变化或变小（被轮转），跳过
        if file_size <= self.last_offset {
            if file_size < self.last_offset {
                // 文件被轮转，从头开始
                log::info!("history.jsonl 文件被轮转，重置偏移");
                self.last_offset = 0;
            }
            return Ok(vec![]);
        }

        let mut reader = BufReader::new(file);
        reader.seek(SeekFrom::Start(self.last_offset))?;

        let mut entries = Vec::new();
        for line in reader.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<HistoryEntry>(trimmed) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    log::warn!("解析 history 行失败: {} — 行内容: {}", e, &trimmed[..trimmed.len().min(200)]);
                }
            }
        }

        // 更新偏移到文件末尾
        self.last_offset = file_size;

        log::debug!(
            "history.jsonl 增量解析: {} 条新记录 (偏移 {} → {})",
            entries.len(),
            self.last_offset,
            file_size
        );

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_incremental() {
        let mut tmp = std::env::temp_dir();
        tmp.push("test_history.jsonl");

        // 写入测试数据
        let mut f = std::fs::File::create(&tmp).unwrap();
        writeln!(f, r#"{{"display":"测试1","timestamp":"2024-01-01T10:00:00.000Z","project":"/tmp/test","sessionId":"s1"}}"#).unwrap();
        writeln!(f, r#"{{"display":"测试2","timestamp":"2024-01-01T11:00:00.000Z","project":"/tmp/test","sessionId":"s2"}}"#).unwrap();

        let mut parser = HistoryParser::new(tmp.clone());
        let entries = parser.parse_incremental().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].session_id, "s1");
        assert_eq!(entries[1].session_id, "s2");

        // 增量：没有新行
        let entries = parser.parse_incremental().unwrap();
        assert_eq!(entries.len(), 0);

        // 增量：追加新行
        let mut f = std::fs::OpenOptions::new().append(true).open(&tmp).unwrap();
        writeln!(f, r#"{{"display":"测试3","timestamp":"2024-01-01T12:00:00.000Z","project":"/tmp/test","sessionId":"s3"}}"#).unwrap();
        drop(f);

        let entries = parser.parse_incremental().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_id, "s3");

        std::fs::remove_file(&tmp).ok();
    }
}
