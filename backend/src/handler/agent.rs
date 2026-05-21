use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::Deserialize;
use serde_json::json;
use crate::router::AppState;
use crate::store;
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

/// Agent 注册请求
#[derive(Deserialize)]
pub struct RegisterRequest {
    pub agent_id: String,
    #[serde(default)]
    pub hostname_hash: String,
    #[serde(default)]
    pub os_info: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

/// Agent 配置查询参数
#[derive(Deserialize)]
pub struct ConfigQuery {
    pub agent_id: String,
}

/// POST /api/v1/agent/register
///
/// 注册新 Agent，返回 api_key 和默认配置
pub async fn register_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    // 生成 api_key = HMAC-SHA256(agent_id, jwt_secret)
    let mut mac = HmacSha256::new_from_slice(state.agent_api_secret.as_bytes())
        .expect("HMAC key 创建失败");
    mac.update(req.agent_id.as_bytes());
    let api_key = hex::encode(mac.finalize().into_bytes());

    // 写入数据库
    match store::agent::register(
        &state.pool,
        &req.agent_id,
        &req.hostname_hash,
        &api_key,
        req.os_info.as_deref(),
        req.version.as_deref(),
    )
    .await
    {
        Ok(_) => {
            tracing::info!("Agent 注册成功: id={}", req.agent_id);
            (
                StatusCode::OK,
                Json(json!({
                    "status": "ok",
                    "agent_id": req.agent_id,
                    "api_key": api_key,
                    "config": {
                        "collect_interval_secs": 60,
                        "report_interval_secs": 60,
                    }
                })),
            )
                .into_response()
        }
        Err(e) => {
            tracing::error!("Agent 注册失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("注册失败: {}", e) })),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/agent/config?agent_id=xxx
///
/// 获取 Agent 配置
pub async fn get_agent_config(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ConfigQuery>,
) -> impl IntoResponse {
    match store::agent::find_by_id(&state.pool, &query.agent_id).await {
        Ok(Some(agent)) => {
            Json(json!({
                "status": "ok",
                "agent_id": agent.id,
                "config": {
                    "collect_interval_secs": 60,
                    "report_interval_secs": 60,
                }
            }))
            .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Agent 未注册" })),
        )
            .into_response(),
        Err(e) => {
            tracing::error!("查询 Agent 失败: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("查询失败: {}", e) })),
            )
                .into_response()
        }
    }
}
