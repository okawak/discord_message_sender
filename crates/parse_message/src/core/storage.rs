use super::{dates, logs::*, models::*};
use regex_lite::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::LazyLock};
use tsify::Tsify;

#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct MessageStorageOptions {
    pub message_storage_mode: MessageStorageMode,
    pub show_author_names: bool,
    pub show_message_time: bool,
    pub time_zone: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct StorageInput {
    pub message_directory: String,
    pub clipping_directory: String,
    pub messages: Vec<ProcessedMessage>,
    pub options: MessageStorageOptions,
    pub existing_ids: Vec<String>,
    pub existing_clipping_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
pub struct IndividualWrite {
    pub directory: String,
    pub path: String,
    pub content: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
pub struct AggregatedLogGroup {
    pub mode: AggregatedStorageMode,
    pub period: String,
    pub path: String,
    pub entries: Vec<AggregatedLogEntry>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
pub struct StoragePlan {
    pub individual: Vec<IndividualWrite>,
    pub groups: Vec<AggregatedLogGroup>,
}
static INDIVIDUAL_ID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[0-9]{8}_[0-9]{6}_([0-9]+)\.md$").unwrap());
pub fn individual_id(name: &str) -> Option<String> {
    INDIVIDUAL_ID.captures(name).map(|c| c[1].to_string())
}
fn target(
    directory: &str,
    mode: AggregatedStorageMode,
    date: &LocalDateTime,
) -> AggregatedLogGroup {
    let period = match mode {
        AggregatedStorageMode::Daily => &date.date,
        AggregatedStorageMode::Weekly => &date.week,
        AggregatedStorageMode::Monthly => &date.month,
    };
    AggregatedLogGroup {
        mode,
        period: period.clone(),
        path: format!("{directory}/{period}.md"),
        entries: vec![],
    }
}
pub fn candidate_paths(input: &StorageInput) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for message in &input.messages {
        // Validate every timestamp before the host can perform any write.
        dates::local(&message.timestamp, &input.options.time_zone)?;
        if message.is_clipping {
            continue;
        }
        for date in dates::possible_dates(&message.timestamp)? {
            for mode in [
                AggregatedStorageMode::Daily,
                AggregatedStorageMode::Weekly,
                AggregatedStorageMode::Monthly,
            ] {
                let path = target(&input.message_directory, mode, &date).path;
                if seen.insert(path.clone()) {
                    paths.push(path);
                }
            }
        }
    }
    Ok(paths)
}
pub fn plan(input: StorageInput) -> Result<StoragePlan, String> {
    let mut ids: HashSet<_> = input.existing_ids.into_iter().collect();
    let mut clipping_ids: HashSet<_> = input.existing_clipping_ids.into_iter().collect();
    let mut plan = StoragePlan {
        individual: vec![],
        groups: vec![],
    };
    for message in input.messages {
        let date = dates::local(&message.timestamp, &input.options.time_zone)?;
        let seen = if message.is_clipping {
            &mut clipping_ids
        } else {
            &mut ids
        };
        if !seen.insert(message.message_id.clone()) {
            continue;
        }
        if message.is_clipping
            || input.options.message_storage_mode == MessageStorageMode::Individual
        {
            let directory = if message.is_clipping {
                &input.clipping_directory
            } else {
                &input.message_directory
            };
            plan.individual.push(IndividualWrite {
                directory: directory.clone(),
                path: format!("{directory}/{}.md", message.file_name),
                content: message.markdown,
            });
            continue;
        }
        let mode = match input.options.message_storage_mode {
            MessageStorageMode::Daily => AggregatedStorageMode::Daily,
            MessageStorageMode::Weekly => AggregatedStorageMode::Weekly,
            MessageStorageMode::Monthly => AggregatedStorageMode::Monthly,
            MessageStorageMode::Individual => unreachable!(),
        };
        let mut group = target(&input.message_directory, mode, &date);
        let entry = AggregatedLogEntry {
            message_id: message.message_id,
            date: date.date,
            time: date.time,
            author_name: message.author_name,
            markdown: message.markdown,
        };
        if let Some(existing) = plan.groups.iter_mut().find(|g| g.path == group.path) {
            existing.entries.push(entry);
        } else {
            group.entries.push(entry);
            plan.groups.push(group);
        }
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::messages;

    fn message(id: &str) -> DiscordMessage {
        DiscordMessage {
            id: id.into(),
            content: "hello".into(),
            timestamp: "2026-06-30T15:30:00Z".into(),
            author: None,
            member: None,
        }
    }

    #[test]
    fn storage_plan_deduplicates_and_keeps_clippings_individual() {
        let regular =
            messages::processed("regular".into(), false, message("1"), "Asia/Tokyo").unwrap();
        let clipping =
            messages::processed("clip".into(), true, message("2"), "Asia/Tokyo").unwrap();
        let input = StorageInput {
            message_directory: "Logs".into(),
            clipping_directory: "Clips".into(),
            messages: vec![regular.clone(), regular.clone(), clipping],
            options: MessageStorageOptions {
                message_storage_mode: MessageStorageMode::Monthly,
                show_author_names: true,
                show_message_time: false,
                time_zone: "Asia/Tokyo".into(),
            },
            existing_ids: vec![],
            existing_clipping_ids: vec![],
        };
        let candidates = candidate_paths(&input).unwrap();
        assert!(candidates.contains(&"Logs/2026-06.md".into()));
        assert!(candidates.contains(&"Logs/2026-07.md".into()));
        let plan = super::plan(input.clone()).unwrap();
        assert_eq!(plan.individual[0].path, "Clips/20260701_003000_2.md");
        assert_eq!(plan.groups[0].path, "Logs/2026-07.md");
        assert_eq!(plan.groups[0].entries.len(), 1);
        let plan = super::plan(StorageInput {
            existing_ids: vec!["1".into()],
            existing_clipping_ids: vec!["2".into()],
            ..input
        })
        .unwrap();
        assert!(plan.groups.is_empty());
        assert!(plan.individual.is_empty());
    }

    #[test]
    fn storage_rejects_invalid_timestamps_before_returning_a_write_plan() {
        let mut message = messages::processed("test".into(), false, message("1"), "UTC").unwrap();
        message.timestamp = "invalid".into();
        let input = StorageInput {
            message_directory: "Logs".into(),
            clipping_directory: "Clips".into(),
            messages: vec![message],
            options: MessageStorageOptions {
                message_storage_mode: MessageStorageMode::Individual,
                show_author_names: false,
                show_message_time: false,
                time_zone: "UTC".into(),
            },
            existing_ids: vec![],
            existing_clipping_ids: vec![],
        };
        assert!(candidate_paths(&input).is_err());
        assert!(plan(input).is_err());
    }

    #[test]
    fn individual_ids_reject_lookalike_names_without_parsing_calendar_dates() {
        for (name, expected) in [
            ("20260701_123456_0001.md", "0001"),
            ("99999999_999999_123.md", "123"),
        ] {
            assert_eq!(individual_id(name).as_deref(), Some(expected));
        }
        for name in [
            "2026071_123456_1.md",
            "20260701_12345_1.md",
            "20260701_123456_.md",
            "20260701_123456_1_2.md",
            "20260701_123456_１２.md",
            "２０２６0701_123456_1.md",
            "20260701_123456_1.md\n",
            "20260701_123456_1.MD",
            "notes/20260701_123456_1.md",
        ] {
            assert!(individual_id(name).is_none(), "name: {name:?}");
        }
    }
}
