mod collector;
mod config;
mod engine;
mod reporter;
mod service;
mod util;

use std::env;
use std::panic;
use std::process::Command;
use anyhow::{Context, Result};
use crate::config::AgentConfig;
use crate::engine::{Engine, EngineConfig};
use crate::reporter::http::{HttpReporter, HttpReporterConfig};

const SERVICE_NAME: &str = "AgentCollectTool";

fn main() -> Result<()> {
    // 全局 panic hook：记录崩溃信息后自动重启（仅服务模式）
    panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "未知 panic".to_string()
        };
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "未知位置".to_string());
        log::error!("!!! PANIC: {} (位置: {})", msg, loc);
        // 将 panic 信息写入 stderr 作为最后手段
        eprintln!("!!! PANIC: {} (位置: {})", msg, loc);
    }));

    let args: Vec<String> = env::args().collect();

    // 无参数或由 SCM 启动时，作为 Windows 服务运行
    if args.len() < 2 {
        return service::run_service().map_err(|e| anyhow::anyhow!("服务运行失败: {}", e));
    }

    let command = args[1].as_str();
    match command {
        "install" => install_service()?,
        "uninstall" => uninstall_service()?,
        "start" => start_service()?,
        "stop" => stop_service()?,
        "status" => status_service()?,
        "run" => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(run_foreground())?;
        }
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

/// 获取当前可执行文件的完整路径
fn exe_path() -> Result<String> {
    let path = env::current_exe()
        .with_context(|| "无法获取当前可执行文件路径")?;
    Ok(path.to_string_lossy().to_string())
}

/// 安装 Windows 服务
fn install_service() -> Result<()> {
    println!("正在注册 Windows 服务: {}...", SERVICE_NAME);
    let exe = exe_path()?;

    let output = Command::new("sc.exe")
        .args([
            "create",
            SERVICE_NAME,
            "binPath=",
            &exe,
            "start=",
            "auto",
            "DisplayName=",
            "Agent Collect Tool",
        ])
        .output()
        .with_context(|| "执行 sc.exe create 失败，请确保以管理员权限运行")?;

    if output.status.success() {
        println!("服务注册成功!");
        println!("  sc start {}   # 启动服务", SERVICE_NAME);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already exists") || stderr.contains("已存在") {
            println!("服务已存在，无需重复注册");
        } else {
            eprintln!("服务注册失败: {}", stderr.trim());
        }
    }
    Ok(())
}

/// 卸载 Windows 服务
fn uninstall_service() -> Result<()> {
    println!("正在移除 Windows 服务: {}...", SERVICE_NAME);

    // 先停止服务
    let _ = Command::new("sc.exe")
        .args(["stop", SERVICE_NAME])
        .output();

    let output = Command::new("sc.exe")
        .args(["delete", SERVICE_NAME])
        .output()
        .with_context(|| "执行 sc.exe delete 失败，请确保以管理员权限运行")?;

    if output.status.success() {
        println!("服务移除成功!");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not exist") || stderr.contains("不存在") {
            println!("服务不存在，无需移除");
        } else {
            eprintln!("服务移除失败: {}", stderr.trim());
        }
    }
    Ok(())
}

/// 启动 Windows 服务
fn start_service() -> Result<()> {
    println!("正在启动服务: {}...", SERVICE_NAME);

    let output = Command::new("sc.exe")
        .args(["start", SERVICE_NAME])
        .output()
        .with_context(|| "执行 sc.exe start 失败，请确保以管理员权限运行")?;

    if output.status.success() {
        println!("服务已启动!");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already running") || stderr.contains("已运行") || stderr.contains("1056") {
            println!("服务已在运行中");
        } else {
            eprintln!("服务启动失败: {}", stderr.trim());
        }
    }
    Ok(())
}

/// 停止 Windows 服务
fn stop_service() -> Result<()> {
    println!("正在停止服务: {}...", SERVICE_NAME);

    let output = Command::new("sc.exe")
        .args(["stop", SERVICE_NAME])
        .output()
        .with_context(|| "执行 sc.exe stop 失败，请确保以管理员权限运行")?;

    if output.status.success() {
        println!("服务已停止!");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not running") || stderr.contains("未运行") || stderr.contains("1062") {
            println!("服务未在运行");
        } else {
            eprintln!("服务停止失败: {}", stderr.trim());
        }
    }
    Ok(())
}

/// 查询服务状态
fn status_service() -> Result<()> {
    let output = Command::new("sc.exe")
        .args(["query", SERVICE_NAME])
        .output()
        .with_context(|| "执行 sc.exe query 失败")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", stdout);
    Ok(())
}

/// 前台调试模式：直接运行引擎，不注册为服务
async fn run_foreground() -> Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    log::info!("Agent 前台模式启动...");

    let config = AgentConfig::load()?;
    log::info!("配置加载成功，exe 目录: {:?}", std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())));

    // 创建 HTTP 上报器（带 HMAC 签名密钥）
    let http_reporter = HttpReporter::new(HttpReporterConfig {
        server_url: config.server_url.clone(),
        agent_id: config.agent_id.clone(),
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        timeout_secs: 30,
        api_secret: config.api_secret.clone(),
    })?;

    // 创建引擎
    let engine_config = EngineConfig {
        collect_interval_secs: config.collect_interval_secs,
        report_interval_secs: config.report_interval_secs,
        data_dir: config.data_dir.clone(),
        server_url: config.server_url.clone(),
        agent_id: config.agent_id.clone(),
    };

    // 本地缓冲队列路径
    let queue_path = std::path::PathBuf::from(&config.data_dir).join("queue");
    std::fs::create_dir_all(&queue_path)?;

    let mut engine = Engine::new(engine_config, Box::new(http_reporter), queue_path)?;

    // 注册 Claude Code 采集器
    if config.tools.iter().any(|t| t == "claude-code" || t == "claude") {
        use crate::collector::claude::ClaudeCodeCollector;
        let claude_collector = ClaudeCodeCollector::from_config(config.claude_history_path.clone())?;
        engine.register_collector(Box::new(claude_collector));
    }

    // 注册 Trae 采集器
    if config.tools.iter().any(|t| t == "trae") {
        use crate::collector::trae::TraeCollector;
        use crate::collector::Collector;
        use std::path::PathBuf;
        let trae_collector = if let Some(ref dir) = config.trae_data_dir {
            TraeCollector::new(PathBuf::from(dir))
        } else {
            TraeCollector::new_with_default_path()?
        };
        if trae_collector.is_installed() {
            engine.register_collector(Box::new(trae_collector));
        } else {
            log::info!("Trae 未安装，跳过 Trae 采集器注册");
        }
    }

    log::info!("开始采集主循环...");

    // Ctrl+C 优雅关闭
    tokio::select! {
        result = engine.run() => {
            if let Err(e) = result {
                log::error!("引擎运行出错: {}", e);
            }
        }
        _ = tokio::signal::ctrl_c() => {
            log::info!("收到 Ctrl+C，正在优雅关闭...");
            if let Err(e) = engine.shutdown().await {
                log::error!("引擎关闭出错: {}", e);
            }
        }
    }

    Ok(())
}
