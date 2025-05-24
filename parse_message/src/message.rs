use chrono::{DateTime, FixedOffset, Utc};
use serde::Serialize;

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
        Ok(dt) => {
            // JST (+09:00)
            let jst = FixedOffset::east_opt(9 * 3600).unwrap();
            let dt_jst = dt.with_timezone(&jst);
            dt_jst.format("%Y%m%d_%H%M%S").to_string()
        }
        Err(_) => timestamp.to_owned(),
    }
}
