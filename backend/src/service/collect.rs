use sha2::{Digest, Sha256};
use sqlx::MySqlPool;
use tracing;

use crate::store;

/// 上报事件（来自 Agent 上报）
#[derive(Debug, serde::Deserialize)]
pub struct CollectionPayload {
    pub agent_id: String,
    pub events: Vec<CollectionEvent>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum CollectionEvent {
    Session(SessionPayload),
    Conversation(ConversationPayload),
    CodeEdit(CodeEditPayload),
    Action(ActionPayload),
}

#[derive(Debug, serde::Deserialize)]
pub struct SessionPayload {
    pub session_id: String,
    pub pid: Option<u32>,
    pub cwd: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub version: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ConversationPayload {
    pub session_id: String,
    pub project_path: Option<String>,
    pub project_path_hash: Option<String>,
    pub git_branch: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
    pub messages: Vec<MessagePayload>,
    pub model: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MessagePayload {
    pub seq: u32,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub tokens_input: Option<i32>,
    pub tokens_output: Option<i32>,
    pub timestamp: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct CodeEditPayload {
    pub session_id: String,
    pub file_path: String,
    pub edit_type: String,
    pub lines_added: Option<i32>,
    pub lines_removed: Option<i32>,
    pub diff_content: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct ActionPayload {
    pub session_id: String,
    pub action: String,
    pub message_seq: Option<u32>,
    pub file_path: Option<String>,
    pub extra: Option<serde_json::Value>,
    pub timestamp: String,
}

/// 处理结果
pub struct ProcessResult {
    pub accepted: usize,
    pub rejected: usize,
}

/// 处理采集数据
pub async fn process_batch(pool: &MySqlPool, payload: &CollectionPayload) -> Result<ProcessResult, crate::error::AppError> {
    let mut accepted = 0;
    let mut rejected = 0;

    for event in &payload.events {
        match event {
            CollectionEvent::Session(sess) => {
                match process_session(pool, &payload.agent_id, sess).await {
                    Ok(_) => accepted += 1,
                    Err(e) => {
                        tracing::warn!("处理 session 事件失败: {}", e);
                        rejected += 1;
                    }
                }
            }
            CollectionEvent::Conversation(conv) => {
                match process_conversation(pool, &payload.agent_id, conv).await {
                    Ok(_) => accepted += 1,
                    Err(e) => {
                        tracing::warn!("处理 conversation 事件失败: {}", e);
                        rejected += 1;
                    }
                }
            }
            CollectionEvent::CodeEdit(edit) => {
                match process_code_edit(pool, edit).await {
                    Ok(_) => accepted += 1,
                    Err(e) => {
                        tracing::warn!("处理 code_edit 事件失败: {}", e);
                        rejected += 1;
                    }
                }
            }
            CollectionEvent::Action(action) => {
                match process_action(pool, action).await {
                    Ok(_) => accepted += 1,
                    Err(e) => {
                        tracing::warn!("处理 action 事件失败: {}", e);
                        rejected += 1;
                    }
                }
            }
        }
    }

    // 更新 agent last_seen
    if let Err(e) = store::agent::update_last_seen(pool, &payload.agent_id).await {
        tracing::warn!("更新 agent last_seen 失败: {}", e);
    }

    Ok(ProcessResult { accepted, rejected })
}

async fn process_session(pool: &MySqlPool, agent_id: &str, sess: &SessionPayload) -> Result<(), crate::error::AppError> {
    let started_at = chrono::DateTime::parse_from_rfc3339(&sess.started_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|_| crate::error::AppError::BadRequest("无效的 started_at 格式".to_string())))?;

    let ended_at = sess
        .ended_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let project_path_hash = sess
        .cwd
        .as_deref()
        .map(|p| hash_str(p))
        .unwrap_or_default();

    store::session::upsert(
        pool,
        &sess.session_id,
        agent_id,
        &project_path_hash,
        sess.cwd.as_deref(), // git_branch 从 cwd 获取（后续可改进）
        started_at,
        ended_at,
    )
    .await?;

    Ok(())
}

async fn process_conversation(pool: &MySqlPool, agent_id: &str, conv: &ConversationPayload) -> Result<(), crate::error::AppError> {
    let started_at = chrono::DateTime::parse_from_rfc3339(&conv.started_at)
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .map_err(|_| crate::error::AppError::BadRequest("无效的 started_at 格式".to_string())))?;

    let ended_at = conv
        .ended_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&chrono::Utc));

    let project_path_hash = conv
        .project_path_hash
        .clone()
        .unwrap_or_else(|| {
            conv.project_path
                .as_deref()
                .map(|p| hash_str(p))
                .unwrap_or_default()
        });

    // 先创建 session
    store::session::upsert(
        pool,
        &conv.session_id,
        agent_id,
        &project_path_hash,
        conv.git_branch.as_deref(),
        started_at,
        ended_at,
    )
    .await?;

    // 再插入 messages（content 脱敏后存储）
    let messages: Vec<_> = conv
        .messages
        .iter()
        .map(|m| {
            let content_hash = hash_str(&m.content);
            (
                conv.session_id.clone(),
                m.role.clone(),
                content_hash,
                m.model.clone(),
                m.tokens_input,
                m.tokens_output,
            )
        })
        .collect();

    store::message::batch_insert(pool, &messages).await?;

    Ok(())
}

async fn process_code_edit(pool: &MySqlPool, edit: &CodeEditPayload) -> Result<(), crate::error::AppError> {
    let file_path_hash = hash_str(&edit.file_path);

    // diff 脱敏：截断为骨架
    let diff_skeleton = edit.diff_content.as_deref().map(truncate_diff);

    store::code_edit::insert(
        pool,
        &edit.session_id,
        &file_path_hash,
        &edit.edit_type,
        edit.lines_added,
        edit.lines_removed,
        diff_skeleton.as_deref(),
    )
    .await?;

    Ok(())
}

async fn process_action(pool: &MySqlPool, action: &ActionPayload) -> Result<(), crate::error::AppError> {
    let event_data = action.extra.clone();

    store::action_event::insert(
        pool,
        &action.session_id,
        &action.action,
        event_data.as_ref(),
    )
    .await?;

    Ok(())
}

/// SHA256 哈希
fn hash_str(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Diff 骨架化：只保留结构，去掉具体值
fn truncate_diff(diff: &str) -> String {
    // 提取每行的 +/- 操作符和缩进，但去掉具体内容
    diff.lines()
        .map(|line| {
            if line.starts_with('+') || line.starts_with('-') {
                // 保留操作符和缩进级别
                let trimmed = line.trim_start_matches(['+', '-', ' ']);
                let indent = line.len() - trimmed.len();
                let op = if line.starts_with('+') { '+' } else { '-' };
                format!("{}{}", op, " ".repeat(indent.saturating_sub(1)))
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
