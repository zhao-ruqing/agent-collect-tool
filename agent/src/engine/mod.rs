use anyhow::Result;
use crate::collector::Collector;

/// 采集引擎：管理所有 Collector 实例，编排采集→去重→聚合→上报流程
pub struct Engine {
    /// 已注册的采集器列表
    collectors: Vec<Box<dyn Collector>>,
    /// 配置
    config: EngineConfig,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// 采集间隔（秒）
    pub collect_interval_secs: u64,
    /// 上报间隔（秒）
    pub report_interval_secs: u64,
    /// 本地缓冲目录（sled）
    pub data_dir: String,
    /// 后端服务 URL
    pub server_url: String,
    /// Agent ID
    pub agent_id: String,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self {
        Self {
            collectors: Vec::new(),
            config,
        }
    }

    /// 注册一个采集器
    pub fn register_collector(&mut self, collector: Box<dyn Collector>) {
        log::info!("注册采集器: {}", collector.name());
        self.collectors.push(collector);
    }

    /// 运行采集主循环
    pub async fn run(&mut self) -> Result<()> {
        log::info!("引擎启动，已注册 {} 个采集器", self.collectors.len());

        loop {
            // 遍历所有采集器，增量采集
            for collector in self.collectors.iter_mut() {
                if !collector.is_installed() {
                    log::debug!("采集器 {} 未安装，跳过", collector.name());
                    continue;
                }

                match collector.collect_incremental() {
                    Ok(events) => {
                        if !events.is_empty() {
                            log::info!(
                                "采集器 {} 返回 {} 条事件",
                                collector.name(),
                                events.len()
                            );
                            // TODO: 去重 → 聚合 → 写入 sled 缓冲
                        }
                    }
                    Err(e) => {
                        log::error!("采集器 {} 出错: {}", collector.name(), e);
                    }
                }
            }

            // TODO: 上报 tick（独立线程/异步任务）

            tokio::time::sleep(tokio::time::Duration::from_secs(
                self.config.collect_interval_secs,
            ))
            .await;
        }
    }
}
