pub mod dedup;
pub mod aggregator;

use anyhow::Result;
use crate::collector::Collector;
use crate::reporter::Reporter;
use dedup::DedupFilter;
use aggregator::EventAggregator;

/// 采集引擎：管理所有 Collector 实例，编排采集→去重→聚合→缓冲→上报流程
pub struct Engine {
    collectors: Vec<Box<dyn Collector>>,
    dedup: DedupFilter,
    aggregator: EventAggregator,
    reporter: Box<dyn Reporter>,
    config: EngineConfig,
}

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub collect_interval_secs: u64,
    pub report_interval_secs: u64,
    pub data_dir: String,
    pub server_url: String,
    pub agent_id: String,
}

impl Engine {
    pub fn new(config: EngineConfig, reporter: Box<dyn Reporter>) -> Self {
        Self {
            collectors: Vec::new(),
            dedup: DedupFilter::new(2000),
            aggregator: EventAggregator::new(),
            reporter,
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
        log::info!(
            "引擎启动，已注册 {} 个采集器，采集间隔 {}s，上报间隔 {}s",
            self.collectors.len(),
            self.config.collect_interval_secs,
            self.config.report_interval_secs,
        );

        // 每隔 config.report_interval_secs 上报一次
        let mut last_report = tokio::time::Instant::now();

        loop {
            let tick = tokio::time::Instant::now();

            // 1. 遍历所有采集器，增量采集
            let mut raw_events = Vec::new();
            for collector in self.collectors.iter_mut() {
                if !collector.is_installed() {
                    log::debug!("采集器 {} 未安装，跳过", collector.name());
                    continue;
                }

                match collector.collect_incremental() {
                    Ok(events) => {
                        let count = events.len();
                        if count > 0 {
                            log::debug!("采集器 {} 返回 {} 条事件", collector.name(), count);
                            raw_events.extend(events);
                        }
                    }
                    Err(e) => {
                        log::error!("采集器 {} 出错: {}", collector.name(), e);
                    }
                }
            }

            let collected_count = raw_events.len();
            if collected_count > 0 {
                log::info!("本轮共采集 {} 条原始事件", collected_count);

                // 2. 去重
                let unique_events = self.dedup.filter(raw_events);
                let dup_count = collected_count - unique_events.len();
                if dup_count > 0 {
                    log::debug!("去重过滤 {} 条重复事件", dup_count);
                }

                // 3. 聚合到会话
                for event in unique_events {
                    self.aggregator.push(event);
                }
            }

            // 4. 上报：到了上报间隔或没有数据需要采集时
            if last_report.elapsed().as_secs() >= self.config.report_interval_secs {
                let batched_events = self.aggregator.flush_all();
                if !batched_events.is_empty() {
                    log::info!("准备上报 {} 条事件", batched_events.len());
                    match self.reporter.report(batched_events).await {
                        Ok(count) => {
                            log::info!("上报成功: {} 条", count);
                        }
                        Err(e) => {
                            log::error!("上报失败: {}", e);
                            // 上报失败的事件会由 Reporter 内部的队列保留
                        }
                    }
                }
                last_report = tokio::time::Instant::now();
            }

            // 等待下次采集
            let elapsed = tick.elapsed().as_secs();
            let sleep_duration = self.config.collect_interval_secs.saturating_sub(elapsed);
            if sleep_duration > 0 {
                tokio::time::sleep(tokio::time::Duration::from_secs(sleep_duration)).await;
            }
        }
    }

    /// 优雅关闭：刷新缓冲区并上报
    pub async fn shutdown(&mut self) -> Result<()> {
        log::info!("引擎关闭中，刷新缓冲区...");
        let remaining = self.aggregator.flush_all();
        if !remaining.is_empty() {
            log::info!("关闭前上报 {} 条剩余事件", remaining.len());
            if let Err(e) = self.reporter.report(remaining).await {
                log::error!("关闭前上报失败: {}", e);
            }
        }
        Ok(())
    }
}
