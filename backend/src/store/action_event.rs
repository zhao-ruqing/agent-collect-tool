use sqlx::MySqlPool;

/// 插入一条行为事件
pub async fn insert(
    pool: &MySqlPool,
    session_id: &str,
    event_type: &str,
    event_data: Option<&serde_json::Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO action_events (session_id, event_type, event_data) VALUES (?, ?, ?)",
    )
    .bind(session_id)
    .bind(event_type)
    .bind(event_data)
    .execute(pool)
    .await?;
    Ok(())
}
