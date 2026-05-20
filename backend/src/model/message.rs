use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Message {
    pub id: i64,
    pub session_id: String,
    pub role: String, // 'user' or 'assistant'
    pub content_hash: Option<String>,
    pub content: Option<String>,
    pub model: Option<String>,
    pub tokens_input: Option<i32>,
    pub tokens_output: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
}
