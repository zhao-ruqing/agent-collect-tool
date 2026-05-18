use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

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
}

fn default_tools() -> Vec<String> {
    vec!["claude-code".to_string()]
}

impl AgentConfig {
    pub fn load() -> Result<Self> {
        let config_path = "config.toml";
        let content = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read config file: {}", config_path))?;

        let mut config: AgentConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", config_path))?;

        // Override with environment variables if present
        if let Ok(server_url) = std::env::var("AGENT_SERVER_URL") {
            config.server_url = server_url;
        }
        if let Ok(agent_id) = std::env::var("AGENT_ID") {
            config.agent_id = agent_id;
        }

        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.server_url.is_empty() {
            return Err(anyhow::anyhow!("server_url cannot be empty"));
        }
        if self.agent_id.is_empty() {
            return Err(anyhow::anyhow!("agent_id cannot be empty"));
        }
        if self.collect_interval_secs == 0 {
            return Err(anyhow::anyhow!("collect_interval_secs must be greater than 0"));
        }
        if self.report_interval_secs == 0 {
            return Err(anyhow::anyhow!("report_interval_secs must be greater than 0"));
        }
        
        let data_path = Path::new(&self.data_dir);
        if !data_path.exists() {
            fs::create_dir_all(data_path)
                .with_context(|| format!("Failed to create data directory: {}", self.data_dir))?;
        }

        Ok(())
    }
}
