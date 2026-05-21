use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use anyhow::Result;

pub async fn create_pool(database_url: &str) -> Result<MySqlPool> {
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    tracing::info!("数据库连接池已创建 (max_connections=5)");
    Ok(pool)
}
