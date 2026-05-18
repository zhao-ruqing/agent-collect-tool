use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ActionEvent {
    pub id: i64,
    pub session_id: String,
    pub event_type: String,
    pub event_data: Option<Value>,
    pub created_at: Option<DateTime<Utc>>,
}
