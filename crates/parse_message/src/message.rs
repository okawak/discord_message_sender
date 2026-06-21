use chrono::{DateTime, FixedOffset, Utc};
use serde::Serialize;

const JST_OFFSET_SECONDS: i32 = 9 * 3600;

#[derive(Serialize)]
pub struct ParseMessageResult {
    pub md: String,
    pub is_clip: bool,
    pub name: String, // no need .md extension
}

// ex) "2025-05-24T13:51:41.933000+00:00"
// → "20250524_225141" (JST +09:00)
pub fn format_name(timestamp: &str) -> String {
    let dt_utc = DateTime::parse_from_rfc3339(timestamp).map(|dt| dt.with_timezone(&Utc));

    match dt_utc {
        Ok(dt) => FixedOffset::east_opt(JST_OFFSET_SECONDS)
            .map(|jst| dt.with_timezone(&jst).format("%Y%m%d_%H%M%S").to_string())
            .unwrap_or_else(|| timestamp.to_owned()),
        Err(_) => timestamp.to_owned(),
    }
}
