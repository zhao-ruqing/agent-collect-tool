use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};

pub fn ms_to_datetime(ts: i64) -> DateTime<Utc> {
    match Utc.timestamp_millis_opt(ts) {
        chrono::LocalResult::Single(dt) => dt,
        _ => Utc::now(),
    }
}

pub fn iso_to_datetime(s: &str) -> Result<DateTime<Utc>> {
    let dt = DateTime::parse_from_rfc3339(s)?;
    Ok(dt.with_timezone(&Utc))
}
