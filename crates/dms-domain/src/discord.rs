use super::{channels::display_name, models::DiscordChannelSettings, trim};
use regex_lite::{Captures, Regex};
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeMap, sync::LazyLock};

pub const API_VERSION: u32 = 10;
pub const PAGE_SIZE: usize = 100;
pub const MAX_RETRIES: u32 = 3;

#[derive(Serialize)]
struct CreateMessageBody<'a> {
    content: &'a str,
}

pub fn create_message_body(content: &str) -> String {
    serde_json::to_string(&CreateMessageBody { content })
        .expect("serializing a string-only Discord request cannot fail")
}

pub fn encode_component(value: &str) -> String {
    let mut out = String::new();
    for b in value.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}
pub fn messages_path(channel: &str, before: Option<&str>) -> String {
    let mut path = format!(
        "/channels/{}/messages?limit={PAGE_SIZE}",
        encode_component(channel)
    );
    if let Some(before) = before.filter(|s| !s.is_empty()) {
        path.push_str(&format!("&before={}", encode_component(before)));
    }
    path
}
fn header<'a>(headers: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}
fn js_number(value: &str) -> Option<f64> {
    let text = trim(value);
    if text.is_empty() {
        return Some(0.0);
    }
    for (prefix, radix) in [
        ("0x", 16),
        ("0X", 16),
        ("0b", 2),
        ("0B", 2),
        ("0o", 8),
        ("0O", 8),
    ] {
        if let Some(digits) = text.strip_prefix(prefix) {
            return u64::from_str_radix(digits, radix).ok().map(|n| n as f64);
        }
    }
    text.parse().ok()
}
fn milliseconds(value: &Value) -> Option<f64> {
    let seconds = match value {
        Value::Number(v) => v.as_f64(),
        Value::String(v) => js_number(v),
        _ => None,
    }?;
    (seconds.is_finite() && seconds >= 0.0).then(|| (seconds * 1000.0).ceil())
}
pub fn rate_limit_delay(headers: &BTreeMap<String, String>, text: &str) -> f64 {
    if let Some(delay) =
        header(headers, "Retry-After").and_then(|s| milliseconds(&Value::String(s.into())))
    {
        return delay;
    }
    let body: Value = serde_json::from_str(text).unwrap_or(Value::Null);
    milliseconds(&body["retry_after"]).unwrap_or(1000.0)
}
pub fn reset_delay(headers: &BTreeMap<String, String>) -> f64 {
    if header(headers, "X-RateLimit-Remaining").and_then(js_number) != Some(0.0) {
        return 0.0;
    }
    header(headers, "X-RateLimit-Reset-After")
        .and_then(|s| milliseconds(&Value::String(s.into())))
        .unwrap_or(0.0)
}
pub fn failure_notice(status: u32, method: &str) -> String {
    match status {
        401 => "invalid Discord bot token".into(),
        403 => format!(
            "missing Discord permission ({})",
            if method == "GET" {
                "View Channel / Read Message History"
            } else {
                "Send Messages"
            }
        ),
        404 => "Discord channel was not found".into(),
        _ => format!("Discord API returned {status}"),
    }
}
pub fn network_error_message(method: &str, path: &str) -> String {
    format!("Discord API {method} {path} request failed.")
}
pub fn rate_limit_notice(delay: f64) -> String {
    format!("Rate-limited. Retry after {}s", (delay / 1000.0).ceil())
}
pub fn error_message(status: u32, method: &str, path: &str, text: &str) -> String {
    let hint = match status {
        401 => " Unauthorized. Check the bot token.",
        403 => {
            " Forbidden. Check that the bot is invited to the server and has the required channel permissions."
        }
        404 => " Not Found. Check the channel ID.",
        _ => ".",
    };
    let detail = if text.is_empty() {
        String::new()
    } else if let Some(message) = serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|v| v["message"].as_str().map(str::to_string))
    {
        format!(" Discord says: {message}")
    } else {
        format!(" Discord response: {text}")
    };
    format!("Discord API {method} {path} failed with {status}{hint}{detail}")
}
static VARIABLES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{(count|channelId|channelName)\}").unwrap());
pub fn notification(template: &str, channel: &DiscordChannelSettings, count: usize) -> String {
    VARIABLES
        .replace_all(template, |c: &Captures<'_>| match &c[1] {
            "count" => count.to_string(),
            "channelId" => channel.id.clone(),
            _ => display_name(channel).into(),
        })
        .into_owned()
}
pub fn completion(count: usize, failures: usize) -> String {
    let saved = if count == 0 {
        "No new messages".into()
    } else {
        format!("{count} messages saved")
    };
    if failures == 0 {
        format!("Discord sync finished. {saved}.")
    } else {
        format!(
            "Discord sync finished. {saved}; {failures} channel{} failed.",
            if failures == 1 { "" } else { "s" }
        )
    }
}
#[derive(serde::Serialize, serde::Deserialize, tsify::Tsify)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RetryDecision {
    Success,
    Fail,
    Retry {
        delay: f64,
        #[serde(rename = "rateLimited")]
        rate_limited: bool,
    },
}
pub fn retry(
    status: Option<u32>,
    attempt: u32,
    headers: &BTreeMap<String, String>,
    text: &str,
) -> RetryDecision {
    match status {
        Some(200..=299) => RetryDecision::Success,
        Some(429) if attempt < MAX_RETRIES => RetryDecision::Retry {
            delay: rate_limit_delay(headers, text),
            rate_limited: true,
        },
        None | Some(500..) if attempt < MAX_RETRIES => RetryDecision::Retry {
            delay: 1000.0 * f64::from(attempt + 1),
            rate_limited: false,
        },
        _ => RetryDecision::Fail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn channel(id: &str, name: &str) -> DiscordChannelSettings {
        DiscordChannelSettings {
            id: id.into(),
            name: name.into(),
            last_processed_message_id: None,
        }
    }

    #[test]
    fn serializes_discord_request_bodies() {
        assert_eq!(
            create_message_body("line 1\n\"quoted\""),
            r#"{"content":"line 1\n\"quoted\""}"#
        );
    }

    #[test]
    fn retries_distinguish_network_transient_and_permanent_failures() {
        let headers = BTreeMap::from([("Retry-After".into(), "1.25".into())]);
        assert!(matches!(
            retry(Some(200), 0, &headers, ""),
            RetryDecision::Success
        ));
        assert!(matches!(
            retry(Some(403), 0, &headers, ""),
            RetryDecision::Fail
        ));
        assert!(matches!(
            retry(Some(429), 3, &headers, ""),
            RetryDecision::Fail
        ));
        assert!(matches!(
            retry(Some(429), 0, &headers, ""),
            RetryDecision::Retry {
                delay: 1250.0,
                rate_limited: true
            }
        ));
        assert!(matches!(
            retry(None, 2, &headers, ""),
            RetryDecision::Retry {
                delay: 3000.0,
                rate_limited: false
            }
        ));
        assert!(matches!(
            retry(Some(503), 0, &headers, ""),
            RetryDecision::Retry { .. }
        ));
    }

    #[test]
    fn notification_substitution_does_not_expand_placeholders_inside_channel_names() {
        assert_eq!(
            notification("{channelName} {count}", &channel("1", "{count}"), 5),
            "{count} 5"
        );
    }

    #[test]
    fn notification_keeps_unknown_and_nested_braces_and_does_not_rescan_values() {
        assert_eq!(
            notification(
                "📒 {{count}} {channelName}{channelId} {unknown} {count {count} {",
                &channel("{count}", "{channelId}"),
                7
            ),
            "📒 {7} {channelId}{count} {unknown} {count 7 {"
        );
    }
}
