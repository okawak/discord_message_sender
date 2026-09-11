//! Regression fixtures captured from the previous TypeScript implementation.
//! Keep cross-module compatibility checks together; unit tests live beside their implementation.
use parse_message::core::{channels, dates, logs, settings};
use serde_json::Value;

fn fixtures() -> Value {
    serde_json::from_str(include_str!("fixtures/compatibility.json")).unwrap()
}
fn decode<T: serde::de::DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}

#[test]
fn settings_match_previous_typescript_including_malformed_and_legacy_data() {
    for case in fixtures()["settings"].as_array().unwrap() {
        assert_eq!(
            serde_json::to_value(settings::migrate(&case["input"])).unwrap(),
            case["expected"],
            "input: {}",
            case["input"]
        );
    }
}

#[test]
fn channel_paths_match_previous_typescript_unicode_and_punctuation() {
    let fixtures = fixtures();
    for case in fixtures["channels"].as_array().unwrap() {
        let channel = decode(&case["channel"]);
        assert_eq!(
            channels::path_segment(&channel),
            case["segment"].as_str().unwrap()
        );
        assert_eq!(
            channels::directory("Logs/", &channel),
            case["directory"].as_str().unwrap()
        );
    }
    for case in fixtures["duplicates"].as_array().unwrap() {
        assert_eq!(
            channels::duplicate_path(&decode::<Vec<_>>(&case["channels"])).as_deref(),
            case["expected"].as_str()
        );
    }
}

#[test]
fn logs_match_previous_typescript_byte_for_byte() {
    for case in fixtures()["logs"].as_array().unwrap() {
        let result = logs::merge(
            case["existing"].as_str().unwrap(),
            &decode::<Vec<_>>(&case["entries"]),
            &decode(&case["options"]),
        );
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            case["expected"],
            "options: {}",
            case["options"]
        );
    }
}

#[test]
fn dates_match_intl_at_dst_leap_day_and_iso_week_boundaries() {
    for case in fixtures()["dates"].as_array().unwrap() {
        let result = dates::local(
            case["timestamp"].as_str().unwrap(),
            case["zone"].as_str().unwrap(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            case["expected"],
            "case: {case}"
        );
    }
}
