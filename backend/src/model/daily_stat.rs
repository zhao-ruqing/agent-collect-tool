use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::NaiveDate;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct DailyStat {
    pub id: i64,
    pub agent_id: String,
    pub stat_date: NaiveDate,
    pub total_sessions: i32,
    pub total_messages: i32,
    pub total_tokens: i32,
    pub total_edits: i32,
}
