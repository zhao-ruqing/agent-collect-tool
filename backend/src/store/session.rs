use sqlx::MySqlPool;
use chrono::{DateTime, Utc};

/// 创建或更新 session
pub async fn upsert(
    pool: &MySqlPool,
    id: &str,
    agent_id: &str,
    project_path_hash: &str,
    git_branch: Option<&str>,
    tool_type: Option<&str>,
    started_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO sessions (id, agent_id, project_path_hash, git_branch, tool_type, started_at, ended_at)
           VALUES (?, ?, ?, ?, ?, ?, ?)
           ON DUPLICATE KEY UPDATE
             ended_at = COALESCE(VALUES(ended_at), sessions.ended_at),
             git_branch = COALESCE(VALUES(git_branch), sessions.git_branch),
             tool_type = COALESCE(VALUES(tool_type), sessions.tool_type)"#,
    )
    .bind(id)
    .bind(agent_id)
    .bind(project_path_hash)
    .bind(git_branch)
    .bind(tool_type)
    .bind(started_at)
    .bind(ended_at)
    .execute(pool)
    .await?;
    Ok(())
}
