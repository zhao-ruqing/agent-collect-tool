pub mod queue;
pub mod http;

use anyhow::Result;
use async_trait::async_trait;
use crate::collector::RawEvent;

/// 上报器接口：负责将采集的事件发送到后端服务
#[async_trait]
pub trait Reporter: Send + Sync {
    /// 上报一批事件，返回成功上报的数量
    async fn report(&self, events: Vec<RawEvent>) -> Result<usize>;

    /// 上报器名称
    fn name(&self) -> &str;
}
