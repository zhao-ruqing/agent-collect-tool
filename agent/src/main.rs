mod collector;
mod config;
mod engine;
mod reporter;
mod service;
mod util;

use std::env;
use anyhow::Result;
use crate::config::AgentConfig;
use crate::engine::{Engine, EngineConfig};
use crate::reporter::http::{HttpReporter, HttpReporterConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = args[1].as_str();
    match command {
        "install" => install_service()?,
        "uninstall" => uninstall_service()?,
        "start" => start_service()?,
        "stop" => stop_service()?,
        "status" => status_service()?,
        "run" => run_foreground().await?,
        _ => {
            eprintln!("未知命令: {}", command);
            print_usage();
        }
    }

    Ok(())
}

fn print_usage() {
    println!("用法: agent.exe <命令>");
    println!("命令:");
    println!("  install   - 注册为 Windows 服务");
    println!("  uninstall - 移除 Windows 服务");
    println!("  start     - 启动 Windows 服务");
    println!("  stop      - 停止 Windows 服务");
    println!("  status    - 查看服务状态");
    println!("  run       - 前台调试模式");
}

fn install_service() -> Result<()> {
    println!("安装服务...");
    // TODO: 使用 windows-service 实现实际服务安装
    println!("服务安装成功 (mock)");
    Ok(())
}

fn uninstall_service() -> Result<()> {
    println!("移除服务...");
    println!("服务移除成功 (mock)");
    Ok(())
}

fn start_service() -> Result<()> {
    println!("启动服务...");
    println!("服务已启动 (mock)");
    Ok(())
}

fn stop_service() -> Result<()> {
    println!("停止服务...");
    println!("服务已停止 (mock)");
    Ok(())
}

fn status_service() -> Result<()> {
    println!("检查服务状态...");
    println!("服务状态: 运行中 (mock)");
    Ok(())
}

async fn run_foreground() -> Result<()> {
    log::info!("Agent 前台模式启动...");

    let config = AgentConfig::load()?;
    log::info!("配置加载成功: {:?}", config);

    // 创建 HTTP 上报器
    let http_reporter = HttpReporter::new(HttpReporterConfig {
        server_url: config.server_url.clone(),
        agent_id: config.agent_id.clone(),
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        timeout_secs: 30,
    })?;

    // 创建引擎
    let engine_config = EngineConfig {
        collect_interval_secs: config.collect_interval_secs,
        report_interval_secs: config.report_interval_secs,
        data_dir: config.data_dir.clone(),
        server_url: config.server_url.clone(),
        agent_id: config.agent_id.clone(),
    };
    let mut engine = Engine::new(engine_config, Box::new(http_reporter));

    // 注册采集器
    if config.tools.iter().any(|t| t == "claude-code" || t == "claude") {
        use crate::collector::claude::ClaudeCodeCollector;
        let claude_collector = ClaudeCodeCollector::from_config(config.claude_history_path.clone())?;
        engine.register_collector(Box::new(claude_collector));
    }

    // 运行主循环
    log::info!("开始采集主循环...");
    engine.run().await?;

    Ok(())
}
