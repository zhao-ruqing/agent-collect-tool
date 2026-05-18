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
        .route("/api/v1/collect", post(collect::collect_data))
        .route("/api/v1/agent/register", post(agent::register_agent))
        .route("/api/v1/agent/config", get(agent::get_agent_config))
        .route("/api/v1/admin/dashboard", get(admin::get_dashboard_stats))
        .with_state(state)
}
