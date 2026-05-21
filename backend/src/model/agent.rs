use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Agent {
    pub id: String,
    pub hostname_hash: String,
    pub api_key: Option<String>,
    pub os_info: Option<String>,
    pub version: Option<String>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
}
