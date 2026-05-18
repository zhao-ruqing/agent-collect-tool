use sqlx::MySqlPool;

/// 插入单条消息
pub async fn insert(
    pool: &MySqlPool,
    session_id: &str,
    role: &str,
    content_hash: &str,
    model: Option<&str>,
    tokens_input: Option<i32>,
    tokens_output: Option<i32>,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO messages (session_id, role, content_hash, model, tokens_input, tokens_output) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(session_id)
    .bind(role)
    .bind(content_hash)
    .bind(model)
    .bind(tokens_input)
    .bind(tokens_output)
    .execute(pool)
    .await?;

    Ok(result.last_insert_id())
}

/// 批量插入消息（逐条插入，使用参数化查询确保安全）
pub async fn batch_insert(
    pool: &MySqlPool,
    messages: &[(String, String, String, Option<String>, Option<i32>, Option<i32>)],
) -> Result<(), sqlx::Error> {
    for (session_id, role, content_hash, model, tokens_input, tokens_output) in messages {
        insert(
            pool,
            session_id,
            role,
            content_hash,
            model.as_deref(),
            *tokens_input,
            *tokens_output,
        )
        .await?;
    }
    Ok(())
}
