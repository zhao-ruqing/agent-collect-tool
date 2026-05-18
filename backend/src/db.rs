use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use anyhow::Result;

pub async fn create_pool(database_url: &str) -> Result<MySqlPool> {
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    
    Ok(pool)
}
