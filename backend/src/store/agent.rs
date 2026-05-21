use sqlx::MySqlPool;
use crate::model::agent::Agent;

/// 根据 agent_id 查询 agent
pub async fn find_by_id(pool: &MySqlPool, agent_id: &str) -> Result<Option<Agent>, sqlx::Error> {
    sqlx::query_as::<_, Agent>("SELECT * FROM agents WHERE id = ?")
        .bind(agent_id)
        .fetch_optional(pool)
        .await
}

/// 注册新 agent（自动注册，无 api_key）
pub async fn insert(pool: &MySqlPool, agent_id: &str, hostname_hash: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO agents (id, hostname_hash) VALUES (?, ?) ON DUPLICATE KEY UPDATE hostname_hash = VALUES(hostname_hash)")
        .bind(agent_id)
        .bind(hostname_hash)
        .execute(pool)
        .await?;
    Ok(())
}

/// 正式注册 agent（含 api_key 和详细信息）
pub async fn register(
    pool: &MySqlPool,
    agent_id: &str,
    hostname_hash: &str,
    api_key: &str,
    os_info: Option<&str>,
    version: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO agents (id, hostname_hash, api_key, os_info, version) \
         VALUES (?, ?, ?, ?, ?) \
         ON DUPLICATE KEY UPDATE \
         hostname_hash = VALUES(hostname_hash), \
         api_key = VALUES(api_key), \
         os_info = COALESCE(VALUES(os_info), os_info), \
         version = COALESCE(VALUES(version), version)"
    )
        .bind(agent_id)
        .bind(hostname_hash)
        .bind(api_key)
        .bind(os_info)
        .bind(version)
        .execute(pool)
        .await?;
    Ok(())
}

/// 更新 agent 最后在线时间
pub async fn update_last_seen(pool: &MySqlPool, agent_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE agents SET last_seen_at = NOW() WHERE id = ?")
        .bind(agent_id)
        .execute(pool)
        .await?;
    Ok(())
}
