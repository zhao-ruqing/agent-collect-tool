use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde_json::json;
use crate::router::AppState;
use crate::service::collect::{self, CollectionPayload};
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

/// POST /api/v1/collect
///
/// 接收 Agent 上报的采集数据，HMAC 签名验证后写入 MySQL
pub async fn collect_data(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    // HMAC 签名验证（如果配置了密钥）
    if !state.agent_api_secret.is_empty() {
        let signature = headers
            .get("X-Agent-Signature")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let mut mac = HmacSha256::new_from_slice(state.agent_api_secret.as_bytes())
            .expect("HMAC key 创建失败");
        mac.update(&body);
        let expected = hex::encode(mac.finalize().into_bytes());

        if signature != expected {
            tracing::warn!(
                "HMAC 签名验证失败: agent={:?}",
                headers.get("X-Agent-Id").and_then(|v| v.to_str().ok())
            );
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "error": "签名验证失败" })),
            )
                .into_response();
        }
    }

    // 反序列化请求体
    let payload: CollectionPayload = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("请求体解析失败: {}", e);
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({ "error": format!("数据格式错误: {}", e) })),
            )
                .into_response();
        }
    };

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
