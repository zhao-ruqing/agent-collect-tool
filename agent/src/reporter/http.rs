// HTTP 上报器：批量压缩上报到后端，HMAC 签名 + 指数退避重试
use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde_json::json;
use sha2::Sha256;
use hmac::{Hmac, Mac};
use std::time::Duration;

use crate::collector::RawEvent;
use super::Reporter;

type HmacSha256 = Hmac<Sha256>;

/// HTTP 上报器配置
#[derive(Debug, Clone)]
pub struct HttpReporterConfig {
    pub server_url: String,
    pub agent_id: String,
    pub agent_version: String,
    pub timeout_secs: u64,
    pub api_secret: String,
}

/// HTTP 上报器
pub struct HttpReporter {
    client: Client,
    config: HttpReporterConfig,
}

impl HttpReporter {
    pub fn new(config: HttpReporterConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .gzip(true)
            .build()
            .with_context(|| "创建 HTTP Client 失败")?;

        Ok(Self { client, config })
    }

    /// 计算请求体的 HMAC-SHA256 签名
    fn sign_body(&self, body: &[u8]) -> String {
        if self.config.api_secret.is_empty() {
            return String::new();
        }
        let mut mac = HmacSha256::new_from_slice(self.config.api_secret.as_bytes())
            .expect("HMAC key 创建失败");
        mac.update(body);
        hex::encode(mac.finalize().into_bytes())
    }
}

#[async_trait]
impl Reporter for HttpReporter {
    fn name(&self) -> &str {
        "http"
    }

    async fn report(&self, events: Vec<RawEvent>) -> Result<usize> {
        let total = events.len();
        if total == 0 {
            return Ok(0);
        }

        let url = format!("{}/api/v1/collect", self.config.server_url);
        let payload = build_collection_payload(&self.config, events);

        let mut retry_delay = Duration::from_secs(1);
        let max_retries = 5;

        for attempt in 0..=max_retries {
            let json_body = serde_json::to_vec(&payload)
                .with_context(|| "序列化上报数据失败")?;

            // 计算 HMAC 签名（未压缩 JSON）
            let signature = self.sign_body(&json_body);

            let mut request = self.client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Content-Encoding", "gzip")
                .header("X-Agent-Id", &self.config.agent_id);

            // 如果配置了密钥，添加签名头
            if !signature.is_empty() {
                request = request.header("X-Agent-Signature", &signature);
            }

            let response = request
                .body(gzip_compress(&json_body)?)
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        log::info!("上报成功: {} 条事件, 状态码 {}", total, status);
                        return Ok(total);
                    } else if status.as_u16() == 429 {
                        let retry_after = resp.headers()
                            .get("Retry-After")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse::<u64>().ok())
                            .unwrap_or(retry_delay.as_secs());
                        log::warn!("被限流 (429)，{} 秒后重试", retry_after);
                        tokio::time::sleep(Duration::from_secs(retry_after)).await;
                        continue;
                    } else if status.as_u16() == 401 {
                        let body = resp.text().await.unwrap_or_default();
                        log::error!("签名验证失败 (401): {}, 不重试", body);
                        return Err(anyhow::anyhow!("签名验证失败: {}", body));
                    } else if status.is_client_error() {
                        let body = resp.text().await.unwrap_or_default();
                        log::error!("上报被拒绝 ({}): {}, 不重试", status, body);
                        return Err(anyhow::anyhow!("上报被拒绝 ({}): {}", status, body));
                    } else {
                        log::warn!("上报服务端错误 ({}), 第 {} 次重试", status, attempt + 1);
                    }
                }
                Err(e) => {
                    log::warn!("上报网络错误: {}, 第 {} 次重试", e, attempt + 1);
                }
            }

            if attempt < max_retries {
                tokio::time::sleep(retry_delay).await;
                retry_delay *= 2;
            }
        }

        Err(anyhow::anyhow!("上报失败，已重试 {} 次", max_retries))
    }
}

/// 构建 CollectionPayload JSON
fn build_collection_payload(config: &HttpReporterConfig, events: Vec<RawEvent>) -> serde_json::Value {
    let events_json: Vec<serde_json::Value> = events
        .iter()
        .map(|e| serde_json::to_value(e).unwrap_or(json!({})))
        .collect();

    json!({
        "agent_id": config.agent_id,
        "agent_version": config.agent_version,
        "events": events_json,
    })
}

/// Gzip 压缩
fn gzip_compress(data: &[u8]) -> Result<Vec<u8>> {
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).with_context(|| "gzip 压缩失败")?;
    encoder.finish().with_context(|| "gzip 完成失败")
}
