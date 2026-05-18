use axum::{extract::State, Json, response::IntoResponse};
use serde_json::json;
use crate::router::AppState;
use std::sync::Arc;

pub async fn register_agent(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({ "status": "ok", "agent_id": "agent-001" }))
}

pub async fn get_agent_config(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({ "collect_interval_secs": 60, "report_interval_secs": 300 }))
}
