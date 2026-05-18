use axum::{extract::State, Json, response::IntoResponse};
use serde_json::json;
use crate::router::AppState;
use std::sync::Arc;

pub async fn get_dashboard_stats(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({ "total_agents": 1, "total_sessions": 10 }))
}
