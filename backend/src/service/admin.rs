use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

use crate::model::{action_event::ActionEvent, agent::Agent, code_edit::CodeEdit, daily_stat::DailyStat, message::Message, session::Session};

/// 分页 + 筛选参数
#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    /// 按 agent_id 筛选
    pub agent_id: Option<String>,
    /// 按 AI 工具筛选 (sessions.tool_type: claude-code / trae)
    pub tool_type: Option<String>,
    /// 按事件类型筛选 (用于 action_events 表 event_type)
    pub tool: Option<String>,
    /// 按模型筛选 (用于 messages 表)
    pub model: Option<String>,
    /// 关键词搜索 (匹配 session_id、git_branch)
    pub keyword: Option<String>,
    /// 开始日期 (YYYY-MM-DD)
    pub date_from: Option<String>,
    /// 结束日期 (YYYY-MM-DD)
    pub date_to: Option<String>,
}

impl QueryParams {
    pub fn offset(&self) -> u32 {
        let page = self.page.unwrap_or(1).max(1);
        let page_size = self.page_size.unwrap_or(20).min(100);
        (page - 1) * page_size
    }

    pub fn limit(&self) -> u32 {
        self.page_size.unwrap_or(20).min(100)
    }

    pub fn page(&self) -> u32 {
        self.page.unwrap_or(1)
    }
}

/// 仪表盘统计
#[derive(Debug, serde::Serialize)]
pub struct DashboardStats {
    pub total_agents: i64,
    pub total_sessions: i64,
    pub today_sessions: i64,
    pub total_messages: i64,
    pub total_edits: i64,
    pub recent_sessions: Vec<Session>,
    /// 近 7 天每日对话数
    pub daily_trend: Vec<DailyTrendItem>,
}

#[derive(Debug, serde::Serialize, sqlx::FromRow)]
pub struct DailyTrendItem {
    pub date: chrono::NaiveDate,
    pub count: i64,
}

/// 分页列表
#[derive(Debug, Serialize)]
pub struct PaginatedList<T: Serialize> {
    pub list: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

/// 会话详情（含分页消息列表）
#[derive(Debug, Serialize)]
pub struct ConversationDetail {
    pub session: Session,
    pub messages: Vec<Message>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

/// Agent 详情
#[derive(Debug, Serialize)]
pub struct AgentDetail {
    pub agent: Agent,
    pub recent_sessions: Vec<Session>,
    pub stats: AgentStats,
}

#[derive(Debug, Serialize)]
pub struct AgentStats {
    pub total_sessions: i64,
    pub total_messages: i64,
    pub total_edits: i64,
}

/// 仪表盘统计，支持按 tool_type 筛选
pub async fn get_dashboard_stats(pool: &MySqlPool, params: &QueryParams) -> Result<DashboardStats, crate::error::AppError> {
    let total_agents: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
        .fetch_one(pool)
        .await?;

    // 动态构建 tool_type 条件（sessions 表直接筛选，子表需 JOIN）
    let has_tt = params.tool_type.is_some();
    let tt_val = params.tool_type.clone().unwrap_or_default();

    let (sess_cond, sess_vals) = if has_tt {
        (" WHERE tool_type = ?".to_string(), vec![tt_val.clone()])
    } else {
        (String::new(), vec![])
    };

    let total_sessions_sql = format!("SELECT COUNT(*) FROM sessions{}", sess_cond);
    let mut q = sqlx::query_as(&total_sessions_sql);
    for val in &sess_vals { q = q.bind(val); }
    let total_sessions: (i64,) = q.fetch_one(pool).await?;

    let today_cond = if has_tt {
        "WHERE tool_type = ? AND DATE(started_at) = CURDATE()".to_string()
    } else {
        "WHERE DATE(started_at) = CURDATE()".to_string()
    };
    let today_sql = format!("SELECT COUNT(*) FROM sessions {}", today_cond);
    let mut q = sqlx::query_as(&today_sql);
    for val in &sess_vals { q = q.bind(val); }
    let today_sessions: (i64,) = q.fetch_one(pool).await?;

    let total_messages_sql = if has_tt {
        "SELECT COUNT(*) FROM messages JOIN sessions s ON messages.session_id = s.id WHERE s.tool_type = ?"
    } else {
        "SELECT COUNT(*) FROM messages"
    };
    let mut q = sqlx::query_as(total_messages_sql);
    for val in &sess_vals { q = q.bind(val); }
    let total_messages: (i64,) = q.fetch_one(pool).await?;

    let total_edits_sql = if has_tt {
        "SELECT COUNT(*) FROM code_edits JOIN sessions s ON code_edits.session_id = s.id WHERE s.tool_type = ?"
    } else {
        "SELECT COUNT(*) FROM code_edits"
    };
    let mut q = sqlx::query_as(total_edits_sql);
    for val in &sess_vals { q = q.bind(val); }
    let total_edits: (i64,) = q.fetch_one(pool).await?;

    let recent_sessions_sql = format!("SELECT * FROM sessions{} ORDER BY started_at DESC LIMIT 10", sess_cond);
    let mut q = sqlx::query_as::<_, Session>(&recent_sessions_sql);
    for val in &sess_vals { q = q.bind(val); }
    let recent_sessions = q.fetch_all(pool).await?;

    let trend_cond = if has_tt {
        "WHERE tool_type = ? AND started_at >= DATE_SUB(CURDATE(), INTERVAL 6 DAY)"
    } else {
        "WHERE started_at >= DATE_SUB(CURDATE(), INTERVAL 6 DAY)"
    };
    let daily_trend_sql = format!(
        "SELECT DATE(started_at) as date, COUNT(*) as count FROM sessions {} GROUP BY DATE(started_at) ORDER BY date",
        trend_cond
    );
    let mut q = sqlx::query_as::<_, DailyTrendItem>(&daily_trend_sql);
    for val in &sess_vals { q = q.bind(val); }
    let daily_trend = q.fetch_all(pool).await?;

    Ok(DashboardStats {
        total_agents: total_agents.0,
        total_sessions: total_sessions.0,
        today_sessions: today_sessions.0,
        total_messages: total_messages.0,
        total_edits: total_edits.0,
        recent_sessions,
        daily_trend,
    })
}

/// 对话列表（通过 sessions 表），支持按 agent_id / 时间范围筛选
pub async fn list_conversations(
    pool: &MySqlPool,
    params: &QueryParams,
) -> Result<PaginatedList<Session>, crate::error::AppError> {
    let (where_clause, bind_values) = build_session_filter(params);

    let count_sql = format!("SELECT COUNT(*) FROM sessions{}", where_clause);
    let list_sql = format!(
        "SELECT * FROM sessions{} ORDER BY started_at DESC LIMIT ? OFFSET ?",
        where_clause
    );

    let mut count_query = sqlx::query_as(&count_sql);
    for val in &bind_values {
        count_query = count_query.bind(val);
    }
    let (total,): (i64,) = count_query.fetch_one(pool).await?;

    let mut list_query = sqlx::query_as::<_, Session>(&list_sql);
    for val in &bind_values {
        list_query = list_query.bind(val);
    }
    let list = list_query
        .bind(params.limit())
        .bind(params.offset())
        .fetch_all(pool)
        .await?;

    Ok(PaginatedList {
        list,
        total,
        page: params.page(),
        page_size: params.limit(),
    })
}

/// 代码编辑列表，支持按时间范围 / agent_id / tool_type 筛选
pub async fn list_code_edits(
    pool: &MySqlPool,
    params: &QueryParams,
) -> Result<PaginatedList<CodeEdit>, crate::error::AppError> {
    let (where_clause, join_clause, bind_values) = build_edit_filter(params);

    let count_sql = format!("SELECT COUNT(*) FROM code_edits{}{}", join_clause, where_clause);
    let list_sql = format!(
        "SELECT code_edits.* FROM code_edits{}{} ORDER BY code_edits.created_at DESC LIMIT ? OFFSET ?",
        join_clause, where_clause
    );

    let mut count_query = sqlx::query_as(&count_sql);
    for val in &bind_values {
        count_query = count_query.bind(val);
    }
    let (total,): (i64,) = count_query.fetch_one(pool).await?;

    let mut list_query = sqlx::query_as::<_, CodeEdit>(&list_sql);
    for val in &bind_values {
        list_query = list_query.bind(val);
    }
    let list = list_query
        .bind(params.limit())
        .bind(params.offset())
        .fetch_all(pool)
        .await?;

    Ok(PaginatedList {
        list,
        total,
        page: params.page(),
        page_size: params.limit(),
    })
}

/// 行为事件列表，支持按时间范围 / 事件类型 / tool_type 筛选
pub async fn list_action_events(
    pool: &MySqlPool,
    params: &QueryParams,
) -> Result<PaginatedList<ActionEvent>, crate::error::AppError> {
    let (where_clause, join_clause, bind_values) = build_action_filter(params);

    let count_sql = format!("SELECT COUNT(*) FROM action_events{}{}", join_clause, where_clause);
    let list_sql = format!(
        "SELECT action_events.* FROM action_events{}{} ORDER BY action_events.created_at DESC LIMIT ? OFFSET ?",
        join_clause, where_clause
    );

    let mut count_query = sqlx::query_as(&count_sql);
    for val in &bind_values {
        count_query = count_query.bind(val);
    }
    let (total,): (i64,) = count_query.fetch_one(pool).await?;

    let mut list_query = sqlx::query_as::<_, ActionEvent>(&list_sql);
    for val in &bind_values {
        list_query = list_query.bind(val);
    }
    let list = list_query
        .bind(params.limit())
        .bind(params.offset())
        .fetch_all(pool)
        .await?;

    Ok(PaginatedList {
        list,
        total,
        page: params.page(),
        page_size: params.limit(),
    })
}

/// 每日统计，支持按 agent_id / 日期范围筛选
pub async fn get_daily_stats(
    pool: &MySqlPool,
    params: &QueryParams,
) -> Result<PaginatedList<DailyStat>, crate::error::AppError> {
    let (where_clause, bind_values) = build_daily_filter(params);

    let count_sql = format!("SELECT COUNT(*) FROM daily_stats{}", where_clause);
    let list_sql = format!(
        "SELECT * FROM daily_stats{} ORDER BY stat_date DESC LIMIT ? OFFSET ?",
        where_clause
    );

    let mut count_query = sqlx::query_as(&count_sql);
    for val in &bind_values {
        count_query = count_query.bind(val);
    }
    let (total,): (i64,) = count_query.fetch_one(pool).await?;

    let mut list_query = sqlx::query_as::<_, DailyStat>(&list_sql);
    for val in &bind_values {
        list_query = list_query.bind(val);
    }
    let list = list_query
        .bind(params.limit())
        .bind(params.offset())
        .fetch_all(pool)
        .await?;

    Ok(PaginatedList {
        list,
        total,
        page: params.page(),
        page_size: params.limit(),
    })
}

/// 单个会话详情（含分页消息列表，按时间倒序）
pub async fn get_conversation_detail(
    pool: &MySqlPool,
    session_id: &str,
    params: &QueryParams,
) -> Result<ConversationDetail, crate::error::AppError> {
    let session: Session = sqlx::query_as("SELECT * FROM sessions WHERE id = ?")
        .bind(session_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("会话不存在".into()))?;

    // 仅统计有效的用户-AI对话消息，排除工具调用/系统消息等噪音
    let (total,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM messages WHERE session_id = ? AND role IN ('user', 'assistant') AND content IS NOT NULL AND content != ''",
    )
    .bind(session_id)
    .fetch_one(pool)
    .await?;

    let messages: Vec<Message> = sqlx::query_as(
        "SELECT * FROM messages WHERE session_id = ? AND role IN ('user', 'assistant') AND content IS NOT NULL AND content != '' ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(session_id)
    .bind(params.limit())
    .bind(params.offset())
    .fetch_all(pool)
    .await?;

    Ok(ConversationDetail {
        session,
        messages,
        total,
        page: params.page(),
        page_size: params.limit(),
    })
}

/// Agent 列表（分页）
pub async fn list_agents(
    pool: &MySqlPool,
    params: &QueryParams,
) -> Result<PaginatedList<Agent>, crate::error::AppError> {
    let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
        .fetch_one(pool)
        .await?;

    let list: Vec<Agent> = sqlx::query_as(
        "SELECT * FROM agents ORDER BY last_seen_at DESC LIMIT ? OFFSET ?",
    )
    .bind(params.limit())
    .bind(params.offset())
    .fetch_all(pool)
    .await?;

    Ok(PaginatedList {
        list,
        total,
        page: params.page(),
        page_size: params.limit(),
    })
}

/// Agent 详情（含最近会话和统计）
pub async fn get_agent_detail(
    pool: &MySqlPool,
    agent_id: &str,
) -> Result<AgentDetail, crate::error::AppError> {
    let agent: Agent = sqlx::query_as("SELECT * FROM agents WHERE id = ?")
        .bind(agent_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| crate::error::AppError::NotFound("客户端不存在".into()))?;

    let total_sessions: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE agent_id = ?")
            .bind(agent_id)
            .fetch_one(pool)
            .await?;

    let total_messages: (i64,) =
        sqlx::query_as(
            "SELECT COUNT(*) FROM messages m JOIN sessions s ON m.session_id = s.id WHERE s.agent_id = ?",
        )
        .bind(agent_id)
        .fetch_one(pool)
        .await?;

    let total_edits: (i64,) =
        sqlx::query_as(
            "SELECT COUNT(*) FROM code_edits ce JOIN sessions s ON ce.session_id = s.id WHERE s.agent_id = ?",
        )
        .bind(agent_id)
        .fetch_one(pool)
        .await?;

    let recent_sessions: Vec<Session> = sqlx::query_as(
        "SELECT * FROM sessions WHERE agent_id = ? ORDER BY started_at DESC LIMIT 5",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await?;

    Ok(AgentDetail {
        agent,
        recent_sessions,
        stats: AgentStats {
            total_sessions: total_sessions.0,
            total_messages: total_messages.0,
            total_edits: total_edits.0,
        },
    })
}

/// CSV 导出：查询 sessions + messages 汇总数据，支持按 tool_type 筛选
pub async fn export_csv(
    pool: &MySqlPool,
    params: &QueryParams,
) -> Result<String, crate::error::AppError> {
    let mut conditions: Vec<&str> = Vec::new();
    let mut bind_values: Vec<String> = Vec::new();

    if let Some(ref agent_id) = params.agent_id {
        conditions.push("s.agent_id = ?");
        bind_values.push(agent_id.clone());
    }
    if let Some(ref tool_type) = params.tool_type {
        conditions.push("s.tool_type = ?");
        bind_values.push(tool_type.clone());
    }
    if let Some(ref date_from) = params.date_from {
        conditions.push("s.started_at >= ?");
        bind_values.push(date_from.clone());
    }
    if let Some(ref date_to) = params.date_to {
        conditions.push("s.started_at <= ?");
        bind_values.push(date_to.clone());
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        r#"SELECT s.id, s.agent_id, s.project_path_hash, s.git_branch, s.tool_type,
                  s.started_at, s.ended_at,
                  COUNT(m.id) as msg_count,
                  COALESCE(SUM(m.tokens_input), 0) as total_input,
                  COALESCE(SUM(m.tokens_output), 0) as total_output
           FROM sessions s
           LEFT JOIN messages m ON m.session_id = s.id
           {}
           GROUP BY s.id
           ORDER BY s.started_at DESC
           LIMIT 10000"#,
        where_clause
    );

    let mut query = sqlx::query_as(&sql);
    for val in &bind_values {
        query = query.bind(val);
    }

    #[derive(sqlx::FromRow)]
    struct ExportRow {
        id: String,
        agent_id: String,
        project_path_hash: Option<String>,
        git_branch: Option<String>,
        tool_type: Option<String>,
        started_at: chrono::DateTime<chrono::Utc>,
        ended_at: Option<chrono::DateTime<chrono::Utc>>,
        msg_count: i64,
        total_input: i64,
        total_output: i64,
    }

    let rows: Vec<ExportRow> = query.fetch_all(pool).await?;

    let mut csv = String::from("会话ID,客户端ID,项目哈希,分支,AI工具,开始时间,结束时间,消息数,输入Token,输出Token\n");
    for r in rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            r.id,
            r.agent_id,
            r.project_path_hash.as_deref().unwrap_or(""),
            r.git_branch.as_deref().unwrap_or(""),
            r.tool_type.as_deref().unwrap_or(""),
            r.started_at.format("%Y-%m-%d %H:%M:%S"),
            r.ended_at.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default(),
            r.msg_count,
            r.total_input,
            r.total_output,
        ));
    }

    Ok(csv)
}

// ============================================================
// 辅助函数：动态构建 WHERE 条件
// ============================================================

fn build_session_filter(params: &QueryParams) -> (String, Vec<String>) {
    let mut conditions = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(ref agent_id) = params.agent_id {
        conditions.push("agent_id = ?".to_string());
        values.push(agent_id.clone());
    }
    if let Some(ref tool_type) = params.tool_type {
        conditions.push("tool_type = ?".to_string());
        values.push(tool_type.clone());
    }
    if let Some(ref keyword) = params.keyword {
        let kw = keyword.trim();
        if !kw.is_empty() {
            conditions.push("(id LIKE ? OR git_branch LIKE ?)".to_string());
            let pattern = format!("%{}%", kw);
            values.push(pattern.clone());
            values.push(pattern);
        }
    }
    if let Some(ref date_from) = params.date_from {
        conditions.push("started_at >= ?".to_string());
        values.push(date_from.clone());
    }
    if let Some(ref date_to) = params.date_to {
        conditions.push("started_at <= ?".to_string());
        values.push(date_to.clone());
    }

    let clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };
    (clause, values)
}

fn build_edit_filter(params: &QueryParams) -> (String, String, Vec<String>) {
    let mut conditions = Vec::new();
    let mut values: Vec<String> = Vec::new();
    let mut join = String::new();

    if let Some(ref tool_type) = params.tool_type {
        join = " JOIN sessions s ON code_edits.session_id = s.id".to_string();
        conditions.push("s.tool_type = ?".to_string());
        values.push(tool_type.clone());
    }
    if let Some(ref date_from) = params.date_from {
        conditions.push("created_at >= ?".to_string());
        values.push(date_from.clone());
    }
    if let Some(ref date_to) = params.date_to {
        conditions.push("created_at <= ?".to_string());
        values.push(date_to.clone());
    }

    let clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };
    (clause, join, values)
}

fn build_action_filter(params: &QueryParams) -> (String, String, Vec<String>) {
    let mut conditions = Vec::new();
    let mut values: Vec<String> = Vec::new();
    let mut join = String::new();

    if let Some(ref tool_type) = params.tool_type {
        join = " JOIN sessions s ON action_events.session_id = s.id".to_string();
        conditions.push("s.tool_type = ?".to_string());
        values.push(tool_type.clone());
    }
    if let Some(ref tool) = params.tool {
        conditions.push("event_type = ?".to_string());
        values.push(tool.clone());
    }
    if let Some(ref date_from) = params.date_from {
        conditions.push("created_at >= ?".to_string());
        values.push(date_from.clone());
    }
    if let Some(ref date_to) = params.date_to {
        conditions.push("created_at <= ?".to_string());
        values.push(date_to.clone());
    }

    let clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };
    (clause, join, values)
}

fn build_daily_filter(params: &QueryParams) -> (String, Vec<String>) {
    let mut conditions = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(ref agent_id) = params.agent_id {
        conditions.push("agent_id = ?".to_string());
        values.push(agent_id.clone());
    }
    if let Some(ref date_from) = params.date_from {
        conditions.push("stat_date >= ?".to_string());
        values.push(date_from.clone());
    }
    if let Some(ref date_to) = params.date_to {
        conditions.push("stat_date <= ?".to_string());
        values.push(date_to.clone());
    }

    let clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };
    (clause, values)
}
