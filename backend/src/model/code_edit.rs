use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct CodeEdit {
    pub id: i64,
    pub session_id: String,
    pub file_path_hash: Option<String>,
    pub edit_type: Option<String>,
    pub lines_added: Option<i32>,
    pub lines_removed: Option<i32>,
    pub diff_skeleton: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}
