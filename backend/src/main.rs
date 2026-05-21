mod config;
mod db;
mod desensitize;
mod error;
mod handler;
mod model;
mod router;
mod service;
mod store;

use crate::config::BackendConfig;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化 tracing：请求追踪 + 慢查询日志
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "backend=debug,tower_http=debug,sqlx=warn".into());

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(filter))
        .init();

    let config = BackendConfig::from_env()?;

    // 创建数据库连接池（含慢查询日志）
    let pool = db::create_pool(&config.database_url).await?;

    // 创建路由
    let app = router::create_router(pool, config.agent_api_secret.clone());

    // 启动服务
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
