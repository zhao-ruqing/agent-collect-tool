use axum::{extract::State, Json, response::IntoResponse};
use serde_json::json;
use crate::router::AppState;
use std::sync::Arc;

pub async fn collect_data(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(json!({ "status": "ok", "message": "Data collected (mock)" }))
}
