pub mod dedup;
pub mod aggregator;

use anyhow::{Context, Result};
use crate::collector::{Collector, RawEvent};
use crate::reporter::Reporter;
use crate::reporter::queue::LocalQueue;
use dedup::DedupFilter;
use aggregator::EventAggregator;
use std::path::PathBuf;

/// 采集引擎：管理所有 Collector 实例，编排采集→去重→聚合→缓冲→上报流程
pub struct Engine {
    collectors: Vec<Box<dyn Collector>>,
    dedup: DedupFilter,
    aggregator: EventAggregator,
    reporter: Box<dyn Reporter>,
    queue: LocalQueue,
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
    pub fn new(config: EngineConfig, reporter: Box<dyn Reporter>, queue_path: PathBuf) -> Result<Self> {
        let queue = LocalQueue::open(&queue_path, 10000)
            .with_context(|| format!("打开本地缓冲队列失败: {:?}", queue_path))?;
        log::info!("本地缓冲队列已打开: {:?}, 当前 {} 条待发送", queue_path, queue.len());

        Ok(Self {
            collectors: Vec::new(),
            dedup: DedupFilter::new(2000),
            aggregator: EventAggregator::new(),
            reporter,
            queue,
            config,
        })
    }

    /// 注册一个采集器
    pub fn register_collector(&mut self, collector: Box<dyn Collector>) {
        log::info!("注册采集器: {}", collector.name());
        self.collectors.push(collector);
    }

    /// 运行采集主循环
    pub async fn run(&mut self) -> Result<()> {
        log::info!(
            "引擎启动，已注册 {} 个采集器，采集间隔 {}s，上报间隔 {}s，队列积压 {} 条",
            self.collectors.len(),
            self.config.collect_interval_secs,
            self.config.report_interval_secs,
            self.queue.len(),
        );

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

                // 4. 刷新聚合结果并写入本地缓冲队列
                let batched = self.aggregator.flush_all();
                if !batched.is_empty() {
                    let batch_count = batched.len();
                    match self.push_to_queue(batched) {
                        Ok(_) => log::debug!("写入队列 {} 条事件", batch_count),
                        Err(e) => log::error!("写入队列失败: {}", e),
                    }
                }
            }

            // 5. 上报：到上报间隔时从队列取出并发送
            if last_report.elapsed().as_secs() >= self.config.report_interval_secs {
                self.flush_queue().await;
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

    /// 将事件序列化并推入本地缓冲队列
    fn push_to_queue(&mut self, events: Vec<RawEvent>) -> Result<()> {
        let payloads: Vec<Vec<u8>> = events
            .iter()
            .map(|e| serde_json::to_vec(e).unwrap_or_default())
            .filter(|p| !p.is_empty())
            .collect();

        if !payloads.is_empty() {
            self.queue.push_batch(payloads)?;
        }
        Ok(())
    }

    /// 从队列取出事件并上报
    async fn flush_queue(&mut self) {
        if self.queue.is_empty() {
            return;
        }

        let queue_len = self.queue.len();
        log::info!("队列中 {} 条事件待上报", queue_len);

        // 每次最多取 50 条
        match self.queue.pop_batch(50) {
            Ok(entries) => {
                if entries.is_empty() {
                    return;
                }

                // 反序列化回 RawEvent
                let events: Vec<RawEvent> = entries
                    .iter()
                    .filter_map(|e| serde_json::from_slice(&e.payload).ok())
                    .collect();

                let max_seq = entries.last().map(|e| e.seq).unwrap_or(0);
                let _event_count = events.len();

                match self.reporter.report(events).await {
                    Ok(count) if count > 0 => {
                        // 上报成功，清除已发送的条目
                        if let Err(e) = self.queue.clear_sent(max_seq) {
                            log::error!("清除队列已发送条目失败: {}", e);
                        } else {
                            log::info!("上报成功 {} 条，队列剩余 {}", count, self.queue.len());
                        }
                    }
                    Ok(_) => {
                        log::warn!("上报返回 0 条，事件保留在队列中");
                    }
                    Err(e) => {
                        log::error!("上报失败: {}，事件保留在队列中", e);
                    }
                }
            }
            Err(e) => {
                log::error!("从队列读取失败: {}", e);
            }
        }
    }

    /// 优雅关闭：刷新聚合缓冲区和队列
    pub async fn shutdown(&mut self) -> Result<()> {
        log::info!("引擎关闭中，刷新缓冲区...");

        // 刷新聚合器中的剩余事件
        let remaining = self.aggregator.flush_all();
        if !remaining.is_empty() {
            if let Err(e) = self.push_to_queue(remaining) {
                log::error!("关闭时写入队列失败: {}", e);
            }
        }

        // 尝试上报队列中的所有事件
        while !self.queue.is_empty() {
            self.flush_queue().await;
        }

        log::info!("引擎已关闭");
        Ok(())
    }
}
