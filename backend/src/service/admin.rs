use serde::Deserialize;
use sqlx::MySqlPool;

use crate::model::{action_event::ActionEvent, code_edit::CodeEdit, daily_stat::DailyStat, session::Session};

/// 分页 + 筛选参数
#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    /// 按 agent_id 筛选
    pub agent_id: Option<String>,
    /// 按工具筛选
    pub tool: Option<String>,
    /// 按模型筛选
    pub model: Option<String>,
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
    pub date: String,
    pub count: i64,
}

/// 分页列表
#[derive(Debug, serde::Serialize)]
pub struct PaginatedList<T: serde::Serialize> {
    pub list: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
}

/// 仪表盘统计
pub async fn get_dashboard_stats(pool: &MySqlPool) -> Result<DashboardStats, crate::error::AppError> {
    let total_agents: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
        .fetch_one(pool)
        .await?;

    let total_sessions: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sessions")
        .fetch_one(pool)
        .await?;

    let today_sessions: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE DATE(started_at) = CURDATE()")
            .fetch_one(pool)
            .await?;

    let total_messages: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages")
        .fetch_one(pool)
        .await?;

    let total_edits: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM code_edits")
        .fetch_one(pool)
        .await?;

    let recent_sessions = sqlx::query_as::<_, Session>(
        "SELECT * FROM sessions ORDER BY started_at DESC LIMIT 10",
    )
    .fetch_all(pool)
    .await?;

    // 近 7 天每日会话趋势
    let daily_trend: Vec<DailyTrendItem> = sqlx::query_as::<_, DailyTrendItem>(
        r#"SELECT DATE(started_at) as date, COUNT(*) as count
           FROM sessions
           WHERE started_at >= DATE_SUB(CURDATE(), INTERVAL 6 DAY)
           GROUP BY DATE(started_at)
           ORDER BY date"#,
    )
    .fetch_all(pool)
    .await?;

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

/// 代码编辑列表，支持按时间范围 / agent_id 筛选
pub async fn list_code_edits(
    pool: &MySqlPool,
    params: &QueryParams,
) -> Result<PaginatedList<CodeEdit>, crate::error::AppError> {
    let (where_clause, bind_values) = build_edit_filter(params);

    let count_sql = format!("SELECT COUNT(*) FROM code_edits{}", where_clause);
    let list_sql = format!(
        "SELECT * FROM code_edits{} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        where_clause
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

/// 行为事件列表，支持按时间范围 / 事件类型筛选
pub async fn list_action_events(
    pool: &MySqlPool,
    params: &QueryParams,
) -> Result<PaginatedList<ActionEvent>, crate::error::AppError> {
    let (where_clause, bind_values) = build_action_filter(params);

    let count_sql = format!("SELECT COUNT(*) FROM action_events{}", where_clause);
    let list_sql = format!(
        "SELECT * FROM action_events{} ORDER BY created_at DESC LIMIT ? OFFSET ?",
        where_clause
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

fn build_edit_filter(params: &QueryParams) -> (String, Vec<String>) {
    let mut conditions = Vec::new();
    let mut values: Vec<String> = Vec::new();

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
    (clause, values)
}

fn build_action_filter(params: &QueryParams) -> (String, Vec<String>) {
    let mut conditions = Vec::new();
    let mut values: Vec<String> = Vec::new();

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
    (clause, values)
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
