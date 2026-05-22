//! Trae IDE 数据采集器
//!
//! 从 Trae 的 workspaceStorage (state.vscdb) 和 Git 快照中
//! 增量提取 AI 对话数据，标准化为 RawEvent。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use super::{
    parser::{self, AgentConfig},
    snapshot::{group_turn_pairs, ChangedFile, SnapshotReader},
    vscdb::VscDbReader,
    workspace::{discover_workspaces, WorkspaceInfo},
};
use crate::collector::{
    CodeEditRecord, Collector, ConversationRecord, EditType,
    MessageRecord, MessageRole, RawEvent, SessionRecord, SessionStatus, ToolType,
};
use chrono::{DateTime, TimeZone, Utc};

/// Trae 采集器
pub struct TraeCollector {
    /// Trae 用户数据目录（%APPDATA%/Trae/User）
    trae_user_dir: std::path::PathBuf,
    /// 快照根目录（%APPDATA%/Trae/ModularData/ai-agent/snapshot）
    snapshot_base: std::path::PathBuf,
    /// 采集游标
    cursor: TraeCursor,
    /// 游标持久化文件路径
    cursor_path: Option<std::path::PathBuf>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct TraeCursor {
    /// 工作区游标：workspace_hash → vscdb 最后修改时间（秒级时间戳）
    workspace_mtimes: HashMap<String, u64>,
    /// 已处理的快照 tag：session_id → tag 名集合
    processed_tags: HashMap<String, HashSet<String>>,
}

impl TraeCollector {
    /// 使用自定义 Trae 用户数据目录创建采集器
    pub fn new(trae_user_dir: std::path::PathBuf, cursor_path: Option<std::path::PathBuf>) -> Self {
        let snapshot_base = trae_user_dir
            .parent()
            .map(|p| {
                p.join("ModularData")
                    .join("ai-agent")
                    .join("snapshot")
            })
            .unwrap_or_default();

        let cursor = cursor_path
            .as_ref()
            .and_then(|p| load_cursor(p))
            .unwrap_or_default();

        Self {
            trae_user_dir,
            snapshot_base,
            cursor,
            cursor_path,
        }
    }

    /// 使用默认 Trae 数据目录创建采集器
    pub fn new_with_default_path(cursor_path: Option<std::path::PathBuf>) -> Result<Self> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("无法获取 APPDATA 目录"))?;

        let trae_user_dir = data_dir.join("Trae").join("User");
        let snapshot_base = data_dir
            .join("Trae")
            .join("ModularData")
            .join("ai-agent")
            .join("snapshot");

        let cursor = cursor_path
            .as_ref()
            .and_then(|p| load_cursor(p))
            .unwrap_or_default();

        Ok(Self {
            trae_user_dir,
            snapshot_base,
            cursor,
            cursor_path,
        })
    }

    /// 检查 vscdb 是否有更新（比较文件修改时间）
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
            self.cursor
                .workspace_mtimes
                .insert(ws.hash.clone(), mt);
        }
    }

    /// 检查 tag 是否已处理
    fn is_tag_processed(&self, session_id: &str, tag_name: &str) -> bool {
        self.cursor
            .processed_tags
            .get(session_id)
            .map(|tags| tags.contains(tag_name))
            .unwrap_or(false)
    }

    /// 标记 tag 为已处理
    fn mark_tag_processed(&mut self, session_id: &str, tag_name: &str) {
        self.cursor
            .processed_tags
            .entry(session_id.to_string())
            .or_default()
            .insert(tag_name.to_string());
    }

    /// 处理单个工作区的 vscdb 数据 → Vec<RawEvent>
    fn process_workspace(&mut self, ws: &WorkspaceInfo) -> Result<Vec<RawEvent>> {
        let mut events = Vec::new();

        let reader = VscDbReader::new(&ws.vscdb_path);
        let data = reader.read_ai_data()?;

        // 解析会话列表
        let session_storage = data.session_storage.unwrap_or_default();
        log::debug!(
            "Trae 工作区 {} session_storage 大小: {} bytes",
            ws.hash,
            session_storage.len()
        );
        let session_entries =
            parser::parse_session_list(&session_storage)?;
        let session_ids: Vec<String> = parser::extract_session_ids(&session_entries);
        log::debug!(
            "Trae 工作区 {} 解析到 {} 个会话条目, {} 个有效 session_id: {:?}",
            ws.hash,
            session_entries.len(),
            session_ids.len(),
            session_ids,
        );

        if session_ids.is_empty() {
            return Ok(events);
        }

        // 解析辅助映射
        let agent_map =
            parser::parse_agent_map(&data.session_agent_map.unwrap_or_default())?;
        let model_map = if let Some((_, ref val)) = data.model_map {
            parser::parse_model_map(val)?
        } else {
            HashMap::new()
        };
        let agent_config = if let Some((_, ref val)) = data.agent_data {
            parser::parse_agent_config(val).ok()
        } else {
            None
        };

        let input_data = data.input_history.unwrap_or_default();
        log::debug!("Trae 工作区 {} input_history 大小: {} bytes", ws.hash, input_data.len());
        let input_entries =
            parser::parse_input_history(&input_data)?;
        log::debug!("Trae 工作区 {} 解析到 {} 条用户输入", ws.hash, input_entries.len());
        // Trae 的 input_history 条目通常不含 sessionId，统一关联到第一个/当前会话
        let mut session_inputs: HashMap<String, Vec<&parser::UserInputEntry>> =
            HashMap::new();
        for entry in &input_entries {
            if let Some(ref sid) = entry.session_id {
                session_inputs.entry(sid.clone()).or_default().push(entry);
            } else if let Some(first_sid) = session_ids.first() {
                // 无 sessionId 的条目归入第一个会话
                session_inputs.entry(first_sid.clone()).or_default().push(entry);
            }
        }

        // 为每个会话生成 RawEvent
        for session_id in &session_ids {
            let entry = session_entries
                .iter()
                .find(|e| e.id.as_deref() == Some(session_id.as_str()));
            let started_at = entry
                .and_then(|e| e.created_at)
                .map(ms_to_datetime)
                .unwrap_or_else(|| Utc::now());

            let _agent_type = agent_map.get(session_id).cloned();
            let model = model_map.get(session_id).cloned();

            // Session 记录
            let session_record = SessionRecord {
                session_id: session_id.clone(),
                pid: None,
                cwd: if ws.project_path.is_empty() {
                    None
                } else {
                    Some(ws.project_path.clone())
                },
                started_at,
                ended_at: entry.and_then(|e| e.updated_at).map(ms_to_datetime),
                version: None,
                status: SessionStatus::Active,
                tool: ToolType::Trae,
            };
            events.push(RawEvent::Session(session_record));

            // 对话记录：从 input-history 构建 messages（仅用户侧，无 assistant 回复）
            let inputs = session_inputs.get(session_id.as_str());
            let messages = build_messages(inputs, &agent_config, &model);

            if !messages.is_empty() {
                let conv_start = messages.first().map(|m| m.timestamp).unwrap_or_default();
                let conv_end = messages.last().map(|m| m.timestamp);

                let conversation = ConversationRecord {
                    session_id: session_id.clone(),
                    project_path_hash: String::new(),
                    project_path: ws.project_path.clone(),
                    git_branch: None,
                    started_at: conv_start,
                    ended_at: conv_end,
                    messages,
                    model,
                    tool: ToolType::Trae,
                };
                events.push(RawEvent::Conversation(conversation));
            }

            // 代码变更：从 Git 快照提取
            if let Ok(snapshot_events) =
                self.process_snapshots(session_id)
            {
                events.extend(snapshot_events);
            }
        }

        Ok(events)
    }

    /// 处理 Git 快照，提取代码变更
    fn process_snapshots(&mut self, session_id: &str) -> Result<Vec<RawEvent>> {
        let mut events = Vec::new();

        // 尝试多种快照路径格式（v2 是版本号，兼容未来 v3 等）
        let candidates = [
            self.snapshot_base.join(session_id).join("v2"),
            self.snapshot_base.join(session_id),
        ];

        let repo_path = match candidates.iter().find(|p| is_git_repo(p)) {
            Some(p) => p.clone(),
            None => return Ok(events),
        };

        let reader = SnapshotReader::open(&repo_path)?;
        let tags = reader.list_tags()?;
        let turn_pairs = group_turn_pairs(&tags);

        for (before, after) in &turn_pairs {
            if self.is_tag_processed(session_id, &before.name)
                && self.is_tag_processed(session_id, &after.name)
            {
                continue;
            }

            match reader.diff_between_tags(&before.name, &after.name) {
                Ok(diff_result) => {
                    for file in &diff_result.files {
                        let code_edit = CodeEditRecord {
                            session_id: session_id.to_string(),
                            file_path: file.path.clone(),
                            edit_type: classify_edit_type(file),
                            lines_added: Some(file.lines_added),
                            lines_removed: Some(file.lines_removed),
                            diff_content: if file.diff_text.is_empty() {
                                None
                            } else {
                                Some(file.diff_text.clone())
                            },
                            timestamp: Utc::now(),
                        };
                        events.push(RawEvent::CodeEdit(code_edit));
                    }
                }
                Err(e) => {
                    log::debug!(
                        "快照 diff 失败 session={} {}..{}: {}",
                        session_id,
                        before.name,
                        after.name,
                        e
                    );
                }
            }

            self.mark_tag_processed(session_id, &before.name);
            self.mark_tag_processed(session_id, &after.name);
        }

        Ok(events)
    }
}

impl Collector for TraeCollector {
    fn name(&self) -> &str {
        "trae"
    }

    fn is_installed(&self) -> bool {
        self.trae_user_dir.exists()
    }

    fn is_running(&self) -> bool {
        use sysinfo::System;
        let sys = System::new_all();
        for proc in sys.processes().values() {
            if proc.name().to_lowercase().contains("trae") {
                return true;
            }
        }
        false
    }

    fn collect_incremental(&mut self) -> Result<Vec<RawEvent>> {
        let mut events = Vec::new();

        let workspaces = discover_workspaces(&self.trae_user_dir)?;

        for ws in &workspaces {
            if !self.vscdb_modified(ws) {
                continue;
            }

            log::debug!("Trae 工作区 {} 有新数据，开始采集", ws.hash);

            match self.process_workspace(ws) {
                Ok(ws_events) => {
                    let count = ws_events.len();
                    if count > 0 {
                        log::info!("Trae 工作区 {} 采集 {} 条事件", ws.hash, count);
                        events.extend(ws_events);
                    }
                }
                Err(e) => {
                    log::error!("Trae 工作区 {} 采集失败: {}", ws.hash, e);
                }
            }

            self.update_cursor(ws);
        }

        // 持久化游标，避免重启后全量重采
        if let Some(ref path) = self.cursor_path {
            if let Err(e) = save_cursor(path, &self.cursor) {
                log::warn!("保存 Trae 游标失败: {}", e);
            }
        }

        Ok(events)
    }

    fn reset_cursor(&mut self) -> Result<()> {
        self.cursor = TraeCursor::default();
        Ok(())
    }
}

// ============================================================
// 辅助函数
// ============================================================

/// 从用户输入列表构建 MessageRecord 序列
fn build_messages(
    inputs: Option<&Vec<&parser::UserInputEntry>>,
    agent_config: &Option<AgentConfig>,
    model: &Option<String>,
) -> Vec<MessageRecord> {
    let entries = match inputs {
        Some(v) if !v.is_empty() => v,
        _ => return vec![],
    };

    let model_name = model
        .clone()
        .or_else(|| agent_config.as_ref().and_then(|c| c.model.clone()));

    entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let text = entry.text.clone().unwrap_or_default();
            // Trae 无精确 token 统计，按字符数/2 粗略估算
            let estimated_tokens = if text.is_empty() {
                0
            } else {
                (text.chars().count() as i32).max(1) / 2
            };

            MessageRecord {
                seq: (i + 1) as u32,
                role: MessageRole::User,
                content: text,
                model: model_name.clone(),
                tokens_input: Some(estimated_tokens),
                tokens_output: None,
                timestamp: entry
                    .timestamp
                    .map(ms_to_datetime)
                    .unwrap_or_else(|| Utc::now()),
            }
        })
        .collect()
}

/// 根据文件变更行数判断编辑类型
fn classify_edit_type(file: &ChangedFile) -> EditType {
    if file.lines_added > 0 && file.lines_removed == 0 {
        EditType::Create
    } else if file.lines_removed > 0 && file.lines_added == 0 {
        EditType::Delete
    } else {
        EditType::Modify
    }
}

/// 毫秒时间戳 → DateTime<Utc>
fn ms_to_datetime(ts: i64) -> DateTime<Utc> {
    let secs = ts / 1000;
    let nsecs = ((ts % 1000) * 1_000_000) as u32;
    Utc.timestamp_opt(secs, nsecs)
        .single()
        .unwrap_or_else(|| Utc.timestamp_opt(0, 0).single().unwrap())
}

/// 检查目录是否为 Git 仓库
fn is_git_repo(path: &std::path::Path) -> bool {
    path.exists() && (path.join(".git").exists() || path.join("HEAD").exists())
}

/// 从磁盘加载游标
fn load_cursor(path: &std::path::Path) -> Option<TraeCursor> {
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

/// 持久化游标到磁盘
fn save_cursor(path: &std::path::Path, cursor: &TraeCursor) -> Result<()> {
    let data = serde_json::to_vec(cursor).with_context(|| "序列化 Trae 游标失败")?;
    std::fs::write(path, &data).with_context(|| "写入 Trae 游标文件失败")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ms_to_datetime() {
        let dt = ms_to_datetime(1700000000000);
        assert_eq!(dt.timestamp(), 1700000000);
    }

    #[test]
    fn test_build_messages_empty() {
        let msgs = build_messages(None, &None, &None);
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_classify_edit() {
        let create = ChangedFile {
            path: "a.rs".into(),
            lines_added: 10,
            lines_removed: 0,
            diff_text: String::new(),
        };
        assert_eq!(classify_edit_type(&create), EditType::Create);

        let modify = ChangedFile {
            path: "b.rs".into(),
            lines_added: 5,
            lines_removed: 3,
            diff_text: String::new(),
        };
        assert_eq!(classify_edit_type(&modify), EditType::Modify);

        let delete = ChangedFile {
            path: "c.rs".into(),
            lines_added: 0,
            lines_removed: 8,
            diff_text: String::new(),
        };
        assert_eq!(classify_edit_type(&delete), EditType::Delete);
    }
}
