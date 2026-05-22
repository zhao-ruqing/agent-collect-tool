use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::path::Path;

/// sessions/<pid>.json 中的会话元信息
#[derive(Debug, Clone, Deserialize)]
pub struct SessionMeta {
    /// 进程 ID
    pub pid: Option<u32>,
    /// 会话 ID
    #[serde(rename = "sessionId")]
    pub session_id: String,
    /// 工作目录
    #[serde(default)]
    pub cwd: Option<String>,
    /// 会话开始时间（字符串 ISO 8601 或数字毫秒时间戳）
    #[serde(rename = "startedAt", default)]
    pub started_at: Option<serde_json::Value>,
    /// Claude Code 版本
    #[serde(default)]
    pub version: Option<String>,
    /// 会话状态: active / completed / abandoned
    #[serde(default)]
    pub status: Option<String>,
}

impl SessionMeta {
    /// 解析开始时间，支持两种格式:
    /// - 字符串: ISO 8601 (如 "2024-01-01T10:00:00.000Z")
    /// - 数字:   毫秒时间戳 (如 1779414618547)
    pub fn parsed_started_at(&self) -> Option<DateTime<Utc>> {
        match &self.started_at {
            Some(serde_json::Value::String(s)) => {
                DateTime::parse_from_rfc3339(s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            }
            Some(serde_json::Value::Number(n)) => {
                n.as_i64().and_then(|ms| {
                    chrono::TimeZone::timestamp_millis_opt(&Utc, ms).single()
                })
            }
            _ => None,
        }
    }
}

/// 解析 sessions 目录下所有 *.json 文件
///
/// 遍历 ~/.claude/sessions/ 目录，解析每个会话元信息文件
pub fn parse_sessions_dir(sessions_dir: &Path) -> Result<Vec<SessionMeta>> {
    if !sessions_dir.exists() || !sessions_dir.is_dir() {
        return Ok(vec![]);
    }

    let mut sessions = Vec::new();

    for entry in std::fs::read_dir(sessions_dir)
        .with_context(|| format!("无法读取 sessions 目录: {:?}", sessions_dir))?
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();

        // 只处理 .json 文件
        if path.extension().map_or(true, |ext| ext != "json") {
            continue;
        }

        match parse_session_file(&path) {
            Ok(meta) => sessions.push(meta),
            Err(e) => {
                log::warn!("解析会话文件失败 {:?}: {}", path, e);
            }
        }
    }

    log::debug!("从 {:?} 解析到 {} 个会话", sessions_dir, sessions.len());
    Ok(sessions)
}

/// 解析单个会话文件
fn parse_session_file(path: &Path) -> Result<SessionMeta> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("无法读取会话文件: {:?}", path))?;

    let meta: SessionMeta = serde_json::from_str(&content)
        .with_context(|| format!("解析会话 JSON 失败: {:?}", path))?;

    Ok(meta)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_parse_sessions_dir() {
        let tmp = std::env::temp_dir().join("test_sessions");
        std::fs::create_dir_all(&tmp).unwrap();

        let f1 = tmp.join("12345.json");
        let mut f = std::fs::File::create(&f1).unwrap();
        write!(
            f,
            r#"{{"pid":12345,"sessionId":"s1","cwd":"/tmp/test","startedAt":"2024-01-01T10:00:00.000Z","version":"1.0.0","status":"active"}}"#
        )
        .unwrap();

        let sessions = parse_sessions_dir(&tmp).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "s1");
        assert_eq!(sessions[0].pid, Some(12345));

        std::fs::remove_dir_all(&tmp).ok();
    }
}
