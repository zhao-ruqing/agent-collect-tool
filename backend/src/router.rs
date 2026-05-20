use axum::{
    routing::{get, post},
    Router,
};
use crate::handler::{admin, agent, collect};
use sqlx::MySqlPool;
use std::sync::Arc;

pub struct AppState {
    pub pool: MySqlPool,
}

pub fn create_router(pool: MySqlPool) -> Router {
    let state = Arc::new(AppState { pool });

    Router::new()
        // 采集接口
        .route("/api/v1/collect", post(collect::collect_data))
        // Agent 接口
        .route("/api/v1/agent/register", post(agent::register_agent))
        .route("/api/v1/agent/config", get(agent::get_agent_config))
        // 管理端接口
        .route("/api/v1/admin/dashboard", get(admin::get_dashboard_stats))
        .route("/api/v1/admin/conversations", get(admin::list_conversations))
        .route("/api/v1/admin/conversations/{session_id}", get(admin::get_conversation_detail))
        .route("/api/v1/admin/edits", get(admin::list_code_edits))
        .route("/api/v1/admin/events", get(admin::list_action_events))
        .route("/api/v1/admin/daily-stats", get(admin::get_daily_stats))
        .route("/api/v1/admin/agents", get(admin::list_agents))
        .route("/api/v1/admin/agents/{agent_id}", get(admin::get_agent_detail))
        .route("/api/v1/admin/export", get(admin::export_csv))
        .with_state(state)
}
