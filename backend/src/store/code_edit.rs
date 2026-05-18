use sqlx::MySqlPool;

/// 插入一条代码编辑记录
pub async fn insert(
    pool: &MySqlPool,
    session_id: &str,
    file_path_hash: &str,
    edit_type: &str,
    lines_added: Option<i32>,
    lines_removed: Option<i32>,
    diff_skeleton: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO code_edits (session_id, file_path_hash, edit_type, lines_added, lines_removed, diff_skeleton) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(session_id)
    .bind(file_path_hash)
    .bind(edit_type)
    .bind(lines_added)
    .bind(lines_removed)
    .bind(diff_skeleton)
    .execute(pool)
    .await?;
    Ok(())
}
