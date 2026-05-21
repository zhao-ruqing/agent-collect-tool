use axum::{
    extract::DefaultBodyLimit,
    routing::{get, post},
    Router,
};
use crate::handler::{admin, agent, collect};
use sqlx::MySqlPool;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    decompression::RequestDecompressionLayer,
    trace::TraceLayer,
};

pub struct AppState {
    pub pool: MySqlPool,
    pub agent_api_secret: String,
}

pub fn create_router(pool: MySqlPool, agent_api_secret: String) -> Router {
    let state = Arc::new(AppState {
        pool,
        agent_api_secret,
    });

    // CORS：允许管理后台跨域访问
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // 采集接口
        .route("/api/v1/collect", post(collect::collect_data))
        // Agent 接口
        .route("/api/v1/agent/register", post(agent::register_agent))
        .route("/api/v1/agent/config", get(agent::get_agent_config))
        // 管理端接口
        .route("/api/v1/admin/dashboard", get(admin::get_dashboard_stats))
        .route("/api/v1/admin/conversations", get(admin::list_conversations))
        .route("/api/v1/admin/conversations/:session_id", get(admin::get_conversation_detail))
        .route("/api/v1/admin/edits", get(admin::list_code_edits))
        .route("/api/v1/admin/events", get(admin::list_action_events))
        .route("/api/v1/admin/daily-stats", get(admin::get_daily_stats))
        .route("/api/v1/admin/agents", get(admin::list_agents))
        .route("/api/v1/admin/agents/:agent_id", get(admin::get_agent_detail))
        .route("/api/v1/admin/export", get(admin::export_csv))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(RequestDecompressionLayer::new())
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB 上传限制
        .with_state(state)
}
