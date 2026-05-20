use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use serde::Deserialize;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;

/// 自定义时间戳反序列化：支持字符串 RFC3339 或整数毫秒时间戳
fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Unexpected};
    struct TimestampVisitor;
    impl<'de> de::Visitor<'de> for TimestampVisitor {
        type Value = DateTime<Utc>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a RFC3339 string or a unix epoch millisecond integer")
        }

        fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|e| E::custom(format!("invalid RFC3339: {}", e)))
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Utc.timestamp_millis_opt(v)
                .single()
                .ok_or_else(|| E::invalid_value(Unexpected::Signed(v), &self))
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Utc.timestamp_millis_opt(v as i64)
                .single()
                .ok_or_else(|| E::invalid_value(Unexpected::Unsigned(v), &self))
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
            Utc.timestamp_millis_opt(v as i64)
                .single()
                .ok_or_else(|| E::invalid_value(Unexpected::Float(v), &self))
        }
    }
    deserializer.deserialize_any(TimestampVisitor)
}

/// history.jsonl 中的单条记录
#[derive(Debug, Clone, Deserialize)]
pub struct HistoryEntry {
    /// 对话标题/首条提示
    pub display: String,
    /// 对话时间戳（RFC3339 字符串 或 毫秒整数）
    #[serde(deserialize_with = "deserialize_timestamp")]
    pub timestamp: DateTime<Utc>,
    /// 项目路径
    pub project: String,
    /// 会话 ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

/// 安全截断字符串，确保不会在 UTF-8 字符中间切割
fn safe_truncate(s: &str, max_chars: usize) -> &str {
    if s.len() <= max_chars {
        return s;
    }
    let mut end = max_chars;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
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
                    log::warn!("解析 history 行失败: {} — 行内容: {}", e, safe_truncate(trimmed, 200));
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
    fn test_safe_truncate_ascii() {
        assert_eq!(safe_truncate("hello", 3), "hel");
        assert_eq!(safe_truncate("hello", 10), "hello");
    }

    #[test]
    fn test_safe_truncate_utf8() {
        let s = "你好世界🌍";
        // '你' = 3 bytes, '好' = 3 bytes, so truncating to 5 bytes would split '好'
        // safe_truncate should roll back to 3 (just "你")
        assert_eq!(safe_truncate(s, 5), "你");
    }

    #[test]
    fn test_parse_incremental() {
        let mut tmp = std::env::temp_dir();
        tmp.push("test_history_parse.jsonl");
        // 先清理可能存在的旧文件
        std::fs::remove_file(&tmp).ok();

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

    #[test]
    fn test_parse_integer_timestamp() {
        let mut tmp = std::env::temp_dir();
        tmp.push("test_history_int_ts.jsonl");
        std::fs::remove_file(&tmp).ok();

        let mut f = std::fs::File::create(&tmp).unwrap();
        // 毫秒时间戳: 1704067200000 = 2024-01-01T00:00:00Z
        writeln!(f, r#"{{"display":"整数时间戳","timestamp":1704067200000,"project":"/tmp","sessionId":"int1"}}"#).unwrap();

        let mut parser = HistoryParser::new(tmp.clone());
        let entries = parser.parse_incremental().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].session_id, "int1");

        std::fs::remove_file(&tmp).ok();
    }
}
