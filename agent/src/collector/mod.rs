pub mod claude;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================
// 采集器 trait
// ============================================================

/// 采集器接口：每种 AI 编程工具实现此 trait
pub trait Collector: Send + Sync {
    /// 采集器名称（如 "claude-code", "trae"）
    fn name(&self) -> &str;

    /// 检测该工具是否已安装
    fn is_installed(&self) -> bool;

    /// 检测该工具当前是否在运行
    fn is_running(&self) -> bool;

    /// 增量采集：从上次 cursor 位置读取新数据
    /// 返回 Vec<RawEvent>，可能为空
    fn collect_incremental(&mut self) -> Result<Vec<RawEvent>>;

    /// 重置采集游标（重新全量采集）
    fn reset_cursor(&mut self) -> Result<()>;
}

// ============================================================
// 工具类型
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolType {
    ClaudeCode,
    Trae,
}

impl ToolType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "claude-code" | "claude" => Some(ToolType::ClaudeCode),
            "trae" => Some(ToolType::Trae),
            _ => None,
        }
    }
}

impl std::fmt::Display for ToolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolType::ClaudeCode => write!(f, "claude-code"),
            ToolType::Trae => write!(f, "trae"),
        }
    }
}

// ============================================================
// 原始事件枚举
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum RawEvent {
    /// 对话记录
    Conversation(ConversationRecord),
    /// 代码编辑记录
    CodeEdit(CodeEditRecord),
    /// 用户行为事件（接受/拒绝/修改等）
    Action(ActionEvent),
    /// 会话元信息
    Session(SessionRecord),
}

// ============================================================
// 数据模型
// ============================================================

/// 对话记录：一次用户-AI 的完整对话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationRecord {
    /// 对话 session_id（来自 Claude Code 的 sessionId）
    pub session_id: String,
    /// 项目路径哈希
    pub project_path_hash: String,
    /// 项目路径（原始，会上报后由后端脱敏）
    pub project_path: String,
    /// Git 分支名
    pub git_branch: Option<String>,
    /// 对话开始时间
    pub started_at: DateTime<Utc>,
    /// 对话结束时间
    pub ended_at: Option<DateTime<Utc>>,
    /// 对话中的消息列表
    pub messages: Vec<MessageRecord>,
    /// 使用的模型
    pub model: Option<String>,
    /// 工具类型
    pub tool: ToolType,
}

/// 单条消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRecord {
    /// 消息序号（从 1 开始）
    pub seq: u32,
    /// 角色：user / assistant
    pub role: MessageRole,
    /// 消息内容（原始文本，会上报后由后端脱敏）
    pub content: String,
    /// 使用的模型
    pub model: Option<String>,
    /// 输入 token 数
    pub tokens_input: Option<i32>,
    /// 输出 token 数
    pub tokens_output: Option<i32>,
    /// 消息时间戳
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
}

/// 代码编辑记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeEditRecord {
    /// 所属 session_id
    pub session_id: String,
    /// 文件路径（原始，会上报后由后端脱敏）
    pub file_path: String,
    /// 编辑类型：create / modify / delete / rename
    pub edit_type: EditType,
    /// 新增行数
    pub lines_added: Option<i32>,
    /// 删除行数
    pub lines_removed: Option<i32>,
    /// 变更内容（diff 文本，会上报后由后端脱敏）
    pub diff_content: Option<String>,
    /// 编辑时间
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditType {
    Create,
    Modify,
    Delete,
    Rename,
}

/// 行为事件：用户对 AI 输出做出的操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionEvent {
    /// 所属 session_id
    pub session_id: String,
    /// 事件类型：accept / reject / modify / ignore / regenerate / copy / paste
    pub action: ActionType,
    /// 相关的消息序号
    pub message_seq: Option<u32>,
    /// 相关的文件路径（如有）
    pub file_path: Option<String>,
    /// 附加数据（JSON）
    pub extra: Option<serde_json::Value>,
    /// 事件时间
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionType {
    Accept,
    Reject,
    Modify,
    Ignore,
    Regenerate,
    Copy,
    Paste,
}

/// 会话元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    /// 会话 ID
    pub session_id: String,
    /// 进程 ID
    pub pid: Option<u32>,
    /// 工作目录
    pub cwd: Option<String>,
    /// 会话开始时间
    pub started_at: DateTime<Utc>,
    /// 会话结束时间
    pub ended_at: Option<DateTime<Utc>>,
    /// Claude Code 版本
    pub version: Option<String>,
    /// 会话状态：active / completed / abandoned
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Active,
    Completed,
    Abandoned,
}
