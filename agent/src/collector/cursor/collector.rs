// Cursor IDE 数据采集器
// 从 workspaceStorage (state.vscdb) 读取 composer 元数据，
// 从 ~/.cursor/projects/ 读取 agent-transcripts JSONL 获取消息内容

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::parser::{self, GenerationEntry, TranscriptLine};
use super::vscdb::VscDbReader;
use crate::collector::trae::workspace::{discover_workspaces, WorkspaceInfo};
use crate::collector::{
    Collector, ConversationRecord, MessageRecord, MessageRole, RawEvent,
    SessionRecord, SessionStatus, ToolType,
};
use chrono::{DateTime, TimeZone, Utc};

/// Cursor 采集器
pub struct CursorCollector {
    /// Cursor 用户数据目录 (%APPDATA%/Cursor/User)
    cursor_user_dir: PathBuf,
    /// Cursor projects 目录 (~/.cursor/projects)
    cursor_projects_dir: PathBuf,
    /// 采集游标
    cursor: CursorCursor,
    /// 游标持久化路径
    cursor_path: Option<PathBuf>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CursorCursor {
    /// 工作区游标: workspace_hash → vscdb 最后修改时间
    workspace_mtimes: HashMap<String, u64>,
    /// transcript 文件游标: (project_dir, composer_id) → 已读取行数
    transcript_offsets: HashMap<String, usize>,
    /// ~/.cursor/projects/ 下 agent-transcripts 目录的最新 mtime（秒级）
    projects_last_mtime: u64,
}

impl CursorCollector {
    /// 使用默认路径创建采集器
    pub fn new_with_default_path(cursor_path: Option<PathBuf>) -> Result<Self> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("无法获取 APPDATA 目录"))?;
        let cursor_user_dir = data_dir.join("Cursor").join("User");

        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("无法获取 HOME 目录"))?;
        let cursor_projects_dir = home.join(".cursor").join("projects");

        let cursor = cursor_path
            .as_ref()
            .and_then(|p| load_cursor(p))
            .unwrap_or_default();

        Ok(Self {
            cursor_user_dir,
            cursor_projects_dir,
            cursor,
            cursor_path,
        })
    }

    /// 检查 vscdb 是否有更新
    fn vscdb_modified(&self, ws: &WorkspaceInfo) -> bool {
        let current_mtime = std::fs::metadata(&ws.vscdb_path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        let prev_mtime = self.cursor.workspace_mtimes.get(&ws.hash).copied();

        match (prev_mtime, current_mtime) {
            (Some(prev), Some(curr)) => curr > prev,
            (None, Some(_)) => true,
            _ => false,
        }
    }

    /// 更新工作区游标
    fn update_cursor(&mut self, ws: &WorkspaceInfo) {
        let mtime = std::fs::metadata(&ws.vscdb_path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        if let Some(mt) = mtime {
            self.cursor.workspace_mtimes.insert(ws.hash.clone(), mt);
        }
    }

    /// 检查 ~/.cursor/projects/ 下 transcript 文件是否比游标更新
    fn projects_modified(&self) -> bool {
        match latest_transcript_mtime(&self.cursor_projects_dir) {
            Some(mt) if mt > self.cursor.projects_last_mtime => true,
            _ => false,
        }
    }

    /// 更新 projects 游标到最新 transcript mtime
    fn update_projects_cursor(&mut self) {
        if let Some(mt) = latest_transcript_mtime(&self.cursor_projects_dir) {
            self.cursor.projects_last_mtime = mt;
        }
    }

    /// 查找 composer 对应的 transcript 目录
    fn find_transcript_dir(&self, composer_id: &str) -> Option<PathBuf> {
        if !self.cursor_projects_dir.exists() {
            return None;
        }

        for entry in std::fs::read_dir(&self.cursor_projects_dir).ok()? {
            let entry = entry.ok()?;
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let transcript_dir = dir.join("agent-transcripts").join(composer_id);
            if transcript_dir.exists() {
                return Some(transcript_dir);
            }
        }
        None
    }

    /// 读取 transcript JSONL 文件（增量：从游标记录的偏移量开始）
    fn read_transcript_lines(
        &mut self,
        transcript_dir: &std::path::Path,
        composer_id: &str,
    ) -> Result<Vec<TranscriptLine>> {
        let transcript_file = transcript_dir.join(format!("{}.jsonl", composer_id));
        if !transcript_file.exists() {
            return Ok(vec![]);
        }

        let content = std::fs::read_to_string(&transcript_file)
            .with_context(|| format!("读取 transcript: {:?}", transcript_file))?;

        let offset_key = format!("{}:{}", transcript_dir.to_string_lossy(), composer_id);
        let prev_offset = self.cursor.transcript_offsets.get(&offset_key).copied().unwrap_or(0);

        let lines: Vec<&str> = content.lines().collect();

        // 从上次偏移量开始读取新行
        let new_lines: Vec<TranscriptLine> = lines
            .iter()
            .skip(prev_offset)
            .filter_map(|line| {
                serde_json::from_str::<TranscriptLine>(line)
                    .map_err(|e| {
                        log::debug!("解析 transcript 行失败: {} — {:?}", e, line.get(..100));
                    })
                    .ok()
            })
            .collect();

        // 更新偏移量
        let new_count = new_lines.len();
        if new_count > 0 {
            self.cursor
                .transcript_offsets
                .insert(offset_key, prev_offset + new_count);
        }

        log::debug!(
            "transcript {} 偏移 {} → {} (新 {} 行)",
            composer_id,
            prev_offset,
            prev_offset + new_count,
            new_count,
        );

        Ok(new_lines)
    }

    /// 扫描所有 transcript 目录，返回 (project_dir_name, composer_id, transcript_dir_path)
    fn scan_all_transcripts(&self) -> Vec<(String, String, PathBuf)> {
        let mut result = Vec::new();
        if !self.cursor_projects_dir.exists() {
            return result;
        }
        for entry in std::fs::read_dir(&self.cursor_projects_dir).ok().into_iter().flatten() {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let project_dir = entry.path();
            if !project_dir.is_dir() {
                continue;
            }
            let project_name = project_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            let transcripts_dir = project_dir.join("agent-transcripts");
            if !transcripts_dir.exists() {
                continue;
            }
            for sub_entry in std::fs::read_dir(&transcripts_dir).ok().into_iter().flatten() {
                let sub_entry = match sub_entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let composer_dir = sub_entry.path();
                if !composer_dir.is_dir() {
                    continue;
                }
                let composer_id = composer_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();
                if !composer_id.is_empty()
                    && composer_dir.join(format!("{}.jsonl", composer_id)).exists()
                {
                    result.push((project_name.clone(), composer_id, composer_dir));
                }
            }
        }
        result
    }

    /// 处理纯 transcript 会话（vscdb 中无记录的孤儿 composer）
    fn process_transcript_session(
        &mut self,
        composer_id: &str,
        transcript_dir: &std::path::Path,
        project_dir_name: &str,
    ) -> Result<Vec<RawEvent>> {
        let mut events = Vec::new();

        let lines = self.read_transcript_lines(transcript_dir, composer_id)?;
        if lines.is_empty() {
            return Ok(events);
        }

        let project_path = parser::decode_project_dir(project_dir_name)
            .unwrap_or_else(|| project_dir_name.to_string());

        let started_at = Utc::now();

        // Session 记录
        let session_record = SessionRecord {
            session_id: composer_id.to_string(),
            pid: None,
            cwd: Some(project_path.clone()),
            started_at,
            ended_at: None,
            version: None,
            status: SessionStatus::Active,
            tool: ToolType::Cursor,
        };
        events.push(RawEvent::Session(session_record));

        // 消息
        let messages = build_messages(&lines);
        let conv_start = messages
            .first()
            .map(|m| m.timestamp)
            .unwrap_or(started_at);
        let conv_end = messages
            .last()
            .map(|m| m.timestamp);

        let conversation = ConversationRecord {
            session_id: composer_id.to_string(),
            project_path_hash: String::new(),
            project_path,
            git_branch: None,
            started_at: conv_start,
            ended_at: conv_end,
            messages,
            model: None,
            tool: ToolType::Cursor,
        };
        events.push(RawEvent::Conversation(conversation));

        Ok(events)
    }

    /// 处理单个工作区 → Vec<RawEvent>
    fn process_workspace(&mut self, ws: &WorkspaceInfo) -> Result<Vec<RawEvent>> {
        let mut events = Vec::new();

        let reader = VscDbReader::new(&ws.vscdb_path);
        let data = reader.read_ai_data()?;

        // 解析 composerData
        let composer_bytes = data.composer_data.unwrap_or_default();
        if composer_bytes.is_empty() {
            return Ok(events);
        }

        let composer_data = parser::parse_composer_data(&composer_bytes)?;
        log::debug!(
            "Cursor 工作区 {} 解析到 {} 个 composer",
            ws.hash,
            composer_data.all_composers.len(),
        );

        // 解析 generations（如有）
        let generations: Vec<GenerationEntry> = if let Some(ref gen_bytes) = data.generations {
            parser::parse_generations(gen_bytes).unwrap_or_default()
        } else {
            vec![]
        };

        // 为每个 composer 生成 RawEvent
        for entry in &composer_data.all_composers {
            if entry.composer_id.is_empty() {
                continue;
            }

            let started_at = entry
                .created_at
                .map(ms_to_datetime)
                .unwrap_or_else(|| Utc::now());

            let ended_at = entry.last_updated_at.map(ms_to_datetime);

            let status = if entry.is_archived.unwrap_or(false) {
                SessionStatus::Completed
            } else if entry.is_draft.unwrap_or(false) {
                SessionStatus::Abandoned
            } else {
                SessionStatus::Active
            };

            // Session 记录
            let session_record = SessionRecord {
                session_id: entry.composer_id.clone(),
                pid: None,
                cwd: if ws.project_path.is_empty() {
                    None
                } else {
                    Some(ws.project_path.clone())
                },
                started_at,
                ended_at,
                version: None,
                status,
                tool: ToolType::Cursor,
            };
            events.push(RawEvent::Session(session_record));

            // 读取 transcript 获取消息
            let messages = if let Some(transcript_dir) =
                self.find_transcript_dir(&entry.composer_id)
            {
                match self.read_transcript_lines(&transcript_dir, &entry.composer_id) {
                    Ok(lines) => build_messages(&lines),
                    Err(e) => {
                        log::debug!(
                            "读取 transcript 失败 composer={}: {}",
                            &entry.composer_id[..8.min(entry.composer_id.len())],
                            e
                        );
                        vec![]
                    }
                }
            } else {
                vec![]
            };

            // Conversation 记录
            let model = parser::infer_model(&entry.unified_mode);
            let conv_start = messages
                .first()
                .map(|m| m.timestamp)
                .unwrap_or(started_at);
            let conv_end = messages
                .last()
                .map(|m| m.timestamp)
                .or(ended_at);

            let conversation = ConversationRecord {
                session_id: entry.composer_id.clone(),
                project_path_hash: String::new(),
                project_path: ws.project_path.clone(),
                git_branch: None,
                started_at: conv_start,
                ended_at: conv_end,
                messages,
                model,
                tool: ToolType::Cursor,
            };
            events.push(RawEvent::Conversation(conversation));

            // 代码编辑: 从 composerData 元数据生成摘要（无实际 diff）
            if let (Some(added), Some(removed)) =
                (entry.total_lines_added, entry.total_lines_removed)
            {
                if added > 0 || removed > 0 {
                    let code_edit = crate::collector::CodeEditRecord {
                        session_id: entry.composer_id.clone(),
                        file_path: String::new(),
                        edit_type: crate::collector::EditType::Modify,
                        lines_added: Some(added),
                        lines_removed: Some(removed),
                        diff_content: None,
                        timestamp: ended_at.unwrap_or(started_at),
                    };
                    events.push(RawEvent::CodeEdit(code_edit));
                }
            }

            // 将 generations 关联到此 composer
            for gen in &generations {
                let ts = gen.unix_ms.map(ms_to_datetime).unwrap_or(started_at);
                if ts >= started_at && ended_at.map_or(true, |end| ts <= end) {
                    let extra = gen.text_description.as_ref().map(|_desc| {
                        serde_json::json!({
                            "generation_uuid": gen.generation_uuid,
                            "generation_type": gen.r#type,
                        })
                    });
                    let action = crate::collector::ActionEvent {
                        session_id: entry.composer_id.clone(),
                        action: crate::collector::ActionType::Accept,
                        message_seq: None,
                        file_path: None,
                        extra,
                        timestamp: ts,
                    };
                    events.push(RawEvent::Action(action));
                }
            }
        }

        Ok(events)
    }
}

impl Collector for CursorCollector {
    fn name(&self) -> &str {
        "cursor"
    }

    fn is_installed(&self) -> bool {
        self.cursor_user_dir.exists() || self.cursor_projects_dir.exists()
    }

    fn is_running(&self) -> bool {
        use sysinfo::System;
        let sys = System::new_all();
        for proc in sys.processes().values() {
            let name = proc.name().to_lowercase();
            if name.contains("cursor") && !name.contains("cursor-") {
                return true;
            }
        }
        false
    }

    fn collect_incremental(&mut self) -> Result<Vec<RawEvent>> {
        let mut events = Vec::new();

        let workspaces = discover_workspaces(&self.cursor_user_dir)?;
        let projects_changed = self.projects_modified();

        // 记录已由 vscdb 工作区处理的 composer ID
        let mut processed_composers = std::collections::HashSet::new();

        for ws in &workspaces {
            if !self.vscdb_modified(ws) && !projects_changed {
                continue;
            }

            log::debug!("Cursor 工作区 {} 有新数据，开始采集", ws.hash);

            match self.process_workspace(ws) {
                Ok(ws_events) => {
                    // 记录已处理的 composer
                    for event in &ws_events {
                        if let RawEvent::Session(ref s) = event {
                            processed_composers.insert(s.session_id.clone());
                        }
                    }
                    let count = ws_events.len();
                    if count > 0 {
                        log::info!(
                            "Cursor 工作区 {} ({}) 采集 {} 条事件",
                            ws.hash,
                            if ws.project_path.is_empty() {
                                "未知项目"
                            } else {
                                &ws.project_path
                            },
                            count,
                        );
                        events.extend(ws_events);
                    }
                }
                Err(e) => {
                    log::error!("Cursor 工作区 {} 采集失败: {}", ws.hash, e);
                }
            }

            self.update_cursor(ws);
        }

        // 扫描 transcript 目录，发现 vscdb 中不存在的孤儿 composer
        if projects_changed {
            let all_transcripts = self.scan_all_transcripts();
            for (project_dir_name, composer_id, transcript_dir) in &all_transcripts {
                if processed_composers.contains(composer_id) {
                    continue;
                }
                log::info!(
                    "Cursor 发现孤儿 transcript: {} (project={})",
                    &composer_id[..8.min(composer_id.len())],
                    project_dir_name,
                );
                match self.process_transcript_session(composer_id, transcript_dir, project_dir_name) {
                    Ok(session_events) => {
                        if !session_events.is_empty() {
                            log::info!(
                                "Cursor 孤儿 transcript {} 采集 {} 条事件",
                                &composer_id[..8.min(composer_id.len())],
                                session_events.len(),
                            );
                            events.extend(session_events);
                        }
                    }
                    Err(e) => {
                        log::error!("处理孤儿 transcript {} 失败: {}", composer_id, e);
                    }
                }
            }
        }

        // 更新 projects 游标（即使没有 workspace 变更，transcript 目录也可能有更新）
        self.update_projects_cursor();

        // 持久化游标
        if let Some(ref path) = self.cursor_path {
            if let Err(e) = save_cursor(path, &self.cursor) {
                log::warn!("保存 Cursor 游标失败: {}", e);
            }
        }

        Ok(events)
    }

    fn reset_cursor(&mut self) -> Result<()> {
        self.cursor = CursorCursor::default();
        Ok(())
    }
}

// ============================================================
// 辅助函数
// ============================================================

/// 从 transcript 行构建 MessageRecord 序列
fn build_messages(lines: &[TranscriptLine]) -> Vec<MessageRecord> {
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let role = match line.role.as_str() {
                "assistant" => MessageRole::Assistant,
                _ => MessageRole::User,
            };
            let content = parser::extract_text(&line.message.content);
            let estimated_tokens = if content.is_empty() {
                0
            } else {
                (content.chars().count() as i32).max(1) / 2
            };

            MessageRecord {
                seq: (i + 1) as u32,
                role,
                content,
                model: None,
                tokens_input: if role == MessageRole::User {
                    Some(estimated_tokens)
                } else {
                    None
                },
                tokens_output: if role == MessageRole::Assistant {
                    Some(estimated_tokens)
                } else {
                    None
                },
                timestamp: Utc::now(),
            }
        })
        .collect()
}

/// 毫秒时间戳 → DateTime<Utc>
fn ms_to_datetime(ts: i64) -> DateTime<Utc> {
    let secs = ts / 1000;
    let nsecs = ((ts % 1000) * 1_000_000) as u32;
    Utc.timestamp_opt(secs, nsecs)
        .single()
        .unwrap_or_else(|| Utc.timestamp_opt(0, 0).single().unwrap())
}

/// 扫描 ~/.cursor/projects/ 下所有 agent-transcripts 目录，返回最新文件 mtime
fn latest_transcript_mtime(projects_dir: &std::path::Path) -> Option<u64> {
    let mut latest: u64 = 0;
    let projects = std::fs::read_dir(projects_dir).ok()?;
    for entry in projects {
        let entry = entry.ok()?;
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let transcripts_dir = dir.join("agent-transcripts");
        if !transcripts_dir.exists() {
            continue;
        }
        for sub_entry in std::fs::read_dir(&transcripts_dir).ok()? {
            let sub_entry = sub_entry.ok()?;
            let composer_dir = sub_entry.path();
            if !composer_dir.is_dir() {
                continue;
            }
            for file_entry in std::fs::read_dir(&composer_dir).ok()? {
                let file_entry = file_entry.ok()?;
                if let Ok(meta) = file_entry.metadata() {
                    if let Ok(mtime) = meta.modified() {
                        if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                            let secs = dur.as_secs();
                            if secs > latest {
                                latest = secs;
                            }
                        }
                    }
                }
            }
        }
    }
    if latest > 0 { Some(latest) } else { None }
}

/// 从磁盘加载游标
fn load_cursor(path: &std::path::Path) -> Option<CursorCursor> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// 持久化游标到磁盘
fn save_cursor(path: &std::path::Path, cursor: &CursorCursor) -> Result<()> {
    let data = serde_json::to_vec(cursor).with_context(|| "序列化 Cursor 游标失败")?;
    std::fs::write(path, &data).with_context(|| "写入 Cursor 游标文件失败")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_all_transcripts_finds_entries() {
        let home = dirs::home_dir().unwrap();
        let cursor_projects_dir = home.join(".cursor").join("projects");
        if !cursor_projects_dir.exists() {
            return; // skip if no Cursor data
        }

        let data_dir = dirs::data_dir().unwrap();
        let collector = CursorCollector {
            cursor_user_dir: data_dir.join("Cursor").join("User"),
            cursor_projects_dir,
            cursor: CursorCursor::default(),
            cursor_path: None,
        };

        let all = collector.scan_all_transcripts();
        // 验证返回的条目都有有效的 composer_id 和路径
        for (proj, cid, dir) in &all {
            assert!(!cid.is_empty(), "composer_id should not be empty");
            assert!(dir.exists(), "transcript dir should exist");
            assert!(!proj.is_empty(), "project name should not be empty");
        }
    }
}
