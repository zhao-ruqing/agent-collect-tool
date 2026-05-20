use axum::{
    extract::{Query, State},
    Json,
    response::IntoResponse,
};
use serde_json::json;
use crate::router::AppState;
use crate::service::admin::{self, QueryParams};
use std::sync::Arc;

/// GET /api/v1/admin/dashboard
pub async fn get_dashboard_stats(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match admin::get_dashboard_stats(&state.pool).await {
        Ok(stats) => Json(json!({ "code": 0, "data": stats })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// GET /api/v1/admin/conversations?page=1&page_size=20&agent_id=xxx&date_from=2026-05-01&date_to=2026-05-20
pub async fn list_conversations(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    match admin::list_conversations(&state.pool, &params).await {
        Ok(result) => Json(json!({ "code": 0, "data": result })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// GET /api/v1/admin/edits?page=1&page_size=20&date_from=2026-05-01&date_to=2026-05-20
pub async fn list_code_edits(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    match admin::list_code_edits(&state.pool, &params).await {
        Ok(result) => Json(json!({ "code": 0, "data": result })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// GET /api/v1/admin/events?page=1&page_size=20&tool=accept&date_from=2026-05-01
pub async fn list_action_events(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    match admin::list_action_events(&state.pool, &params).await {
        Ok(result) => Json(json!({ "code": 0, "data": result })).into_response(),
        Err(e) => e.into_response(),
    }
}

/// GET /api/v1/admin/daily-stats?page=1&page_size=20&agent_id=xxx&date_from=2026-05-01
pub async fn get_daily_stats(
    State(state): State<Arc<AppState>>,
    Query(params): Query<QueryParams>,
) -> impl IntoResponse {
    match admin::get_daily_stats(&state.pool, &params).await {
        Ok(result) => Json(json!({ "code": 0, "data": result })).into_response(),
        Err(e) => e.into_response(),
    }
}
