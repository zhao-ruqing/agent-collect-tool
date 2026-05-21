use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// 获取可执行文件所在目录，用于解析相对路径（解决 Windows 服务 CWD=System32 问题）
fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    pub server_url: String,
    pub agent_id: String,
    pub collect_interval_secs: u64,
    pub report_interval_secs: u64,
    pub log_level: String,
    pub data_dir: String,
    /// 要采集的工具列表: ["claude-code", "trae"]
    #[serde(default = "default_tools")]
    pub tools: Vec<String>,
    pub claude_history_path: Option<String>,
    /// API 签名密钥（与后端 AGENT_API_SECRET 一致）
    #[serde(default)]
    pub api_secret: String,
}

fn default_tools() -> Vec<String> {
    vec!["claude-code".to_string()]
}

impl AgentConfig {
    pub fn load() -> Result<Self> {
        // 优先从 exe 同级目录找配置，其次从 CWD 找
        let exe_config = exe_dir().join("config.toml");
        let config_path: PathBuf = if exe_config.exists() {
            exe_config
        } else {
            PathBuf::from("config.toml")
        };

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("无法读取配置文件: {:?}", config_path))?;

        let mut config: AgentConfig = toml::from_str(&content)
            .with_context(|| format!("无法解析配置文件: {:?}", config_path))?;

        // 环境变量覆盖
        if let Ok(server_url) = std::env::var("AGENT_SERVER_URL") {
            config.server_url = server_url;
        }
        if let Ok(agent_id) = std::env::var("AGENT_ID") {
            config.agent_id = agent_id;
        }
        if let Ok(api_secret) = std::env::var("AGENT_API_SECRET") {
            config.api_secret = api_secret;
        }

        // 将相对路径的 data_dir 解析为基于 exe 目录的规范绝对路径
        let data_path = Path::new(&config.data_dir);
        if data_path.is_relative() {
            let abs = exe_dir().join(data_path);
            // 规范化路径（消除 ./ 和 ..）
            if let Ok(canon) = abs.canonicalize() {
                config.data_dir = canon.to_string_lossy().to_string();
            } else {
                // 目录尚不存在时无法 canonicalize，使用 join 结果
                config.data_dir = abs.to_string_lossy().to_string();
            }
        }

        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.server_url.is_empty() {
            return Err(anyhow::anyhow!("server_url 不能为空"));
        }
        if self.agent_id.is_empty() {
            return Err(anyhow::anyhow!("agent_id 不能为空"));
        }
        if self.collect_interval_secs == 0 {
            return Err(anyhow::anyhow!("collect_interval_secs 必须大于 0"));
        }
        if self.report_interval_secs == 0 {
            return Err(anyhow::anyhow!("report_interval_secs 必须大于 0"));
        }

        let data_path = Path::new(&self.data_dir);
        if !data_path.exists() {
            fs::create_dir_all(data_path)
                .with_context(|| format!("无法创建数据目录: {}", self.data_dir))?;
        }

        Ok(())
    }
}
