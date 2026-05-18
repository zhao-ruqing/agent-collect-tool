use axum::{extract::State, http::StatusCode, Json, response::IntoResponse};
use serde_json::json;
use crate::router::AppState;
use crate::service::collect::{self, CollectionPayload};
use std::sync::Arc;

/// POST /api/v1/collect
///
/// 接收 Agent 上报的采集数据，校验后写入 MySQL
pub async fn collect_data(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CollectionPayload>,
) -> impl IntoResponse {
    tracing::info!(
        "收到采集数据: agent_id={}, events={}",
        payload.agent_id,
        payload.events.len()
    );

    match collect::process_batch(&state.pool, &payload).await {
        Ok(result) => {
            tracing::info!(
                "处理完成: accepted={}, rejected={}",
                result.accepted,
                result.rejected
            );
            (
                StatusCode::OK,
                Json(json!({
                    "status": "ok",
                    "accepted": result.accepted,
                    "rejected": result.rejected
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("处理采集数据失败: {}", e);
            e.into_response()
        }
    }
}
