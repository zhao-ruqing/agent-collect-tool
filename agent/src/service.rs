// Windows 服务管理：注册/启动/停止/运行
use std::ffi::OsString;
use std::sync::mpsc;
use std::time::Duration;
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceExitCode, ServiceStatus, ServiceType,
        ServiceState, ServiceControlAccept,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

const SERVICE_NAME: &str = "AgentCollectTool";

define_windows_service!(ffi_service_main, system_service_main);

/// 启动服务分发器（由 main 中 start 命令调用）
pub fn run_service() -> Result<(), windows_service::Error> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

/// 服务入口点
fn system_service_main(_arguments: Vec<OsString>) {
    // 初始化日志
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));

    if let Err(e) = run_app() {
        log::error!("服务运行失败: {}", e);
    }
}

/// 服务主逻辑：加载配置、创建引擎、运行采集主循环
fn run_app() -> Result<(), anyhow::Error> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel();

    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                log::info!("收到服务停止请求");
                let _ = shutdown_tx.send(());
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    // 创建 tokio runtime 并运行引擎
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;

    rt.block_on(async {
        // 加载配置
        let config = crate::config::AgentConfig::load()?;
        log::info!("服务模式启动，配置: {:?}", config);

        // 创建 HTTP 上报器
        let http_reporter = crate::reporter::http::HttpReporter::new(
            crate::reporter::http::HttpReporterConfig {
                server_url: config.server_url.clone(),
                agent_id: config.agent_id.clone(),
                agent_version: env!("CARGO_PKG_VERSION").to_string(),
                timeout_secs: 30,
            },
        )?;

        // 创建引擎
        let engine_config = crate::engine::EngineConfig {
            collect_interval_secs: config.collect_interval_secs,
            report_interval_secs: config.report_interval_secs,
            data_dir: config.data_dir.clone(),
            server_url: config.server_url.clone(),
            agent_id: config.agent_id.clone(),
        };

        let queue_path = std::path::PathBuf::from(&config.data_dir).join("queue");
        std::fs::create_dir_all(&queue_path)?;

        let mut engine = crate::engine::Engine::new(
            engine_config,
            Box::new(http_reporter),
            queue_path,
        )?;

        // 注册采集器
        use crate::collector::claude::ClaudeCodeCollector;
        if config.tools.iter().any(|t| t == "claude-code" || t == "claude") {
            match ClaudeCodeCollector::from_config(config.claude_history_path.clone()) {
                Ok(claude_collector) => {
                    engine.register_collector(Box::new(claude_collector));
                }
                Err(e) => {
                    log::warn!("Claude Code 采集器初始化失败: {}", e);
                }
            }
        }

        log::info!("引擎启动，开始采集主循环...");

        // 主循环：采集 + 等待关闭信号
        tokio::select! {
            result = engine.run() => {
                if let Err(e) = result {
                    log::error!("引擎运行出错: {}", e);
                }
            }
            _ = async {
                shutdown_rx.recv().ok();
            } => {
                log::info!("正在关闭引擎...");
                if let Err(e) = engine.shutdown().await {
                    log::error!("引擎关闭出错: {}", e);
                }
            }
        }

        Ok::<_, anyhow::Error>(())
    })?;

    status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    Ok(())
}
