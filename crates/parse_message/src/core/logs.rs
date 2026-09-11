use regex_lite::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::LazyLock};
use tsify::Tsify;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum AggregatedStorageMode {
    Daily,
    Weekly,
    Monthly,
}
impl std::fmt::Display for AggregatedStorageMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct AggregatedLogEntry {
    pub message_id: String,
    pub date: String,
    pub time: String,
    pub author_name: String,
    pub markdown: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct AggregatedLogOptions {
    pub mode: AggregatedStorageMode,
    pub show_author_names: bool,
    pub show_message_time: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct AggregatedLogMergeResult {
    pub content: String,
    pub added_count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(transparent)]
pub struct LogEntries(pub Vec<AggregatedLogEntry>);

static MESSAGE_IDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?m)^<!-- discord-message-id: ([0-9]+) -->\r?$").unwrap());
static DATE_SECTIONS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^<!-- discord-message-date: ([0-9]{4}-[0-9]{2}-[0-9]{2}) -->\r?\n## ([0-9]{4}-[0-9]{2}-[0-9]{2})\r?$").unwrap()
});
static FRONTMATTER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^---\r?\n(?:[\s\S]*?\r?\n)?---\r?(?:\n|$)").unwrap());
static BLANK_LINES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:[ \t]*\r?\n)*").unwrap());

pub fn marker(mode: AggregatedStorageMode) -> String {
    format!("<!-- discord-message-sender: {mode}-log -->")
}
pub fn create(mode: AggregatedStorageMode, period: &str) -> String {
    format!("{}\n# {period}\n", marker(mode))
}
pub fn has_marker(content: &str, mode: AggregatedStorageMode) -> bool {
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let content = &content[FRONTMATTER.find(content).map_or(0, |m| m.end())..];
    let content = &content[BLANK_LINES.find(content).map_or(0, |m| m.end())..];
    content.starts_with(&marker(mode))
}
pub fn is_managed(content: &str) -> bool {
    [
        AggregatedStorageMode::Daily,
        AggregatedStorageMode::Weekly,
        AggregatedStorageMode::Monthly,
    ]
    .into_iter()
    .any(|mode| has_marker(content, mode))
}
pub fn message_ids(content: &str) -> Vec<String> {
    MESSAGE_IDS
        .captures_iter(content)
        .map(|c| c[1].to_string())
        .collect()
}
fn append(content: &str, block: &str) -> String {
    let separator = if content.ends_with("\n\n") {
        ""
    } else if content.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    format!("{content}{separator}{block}\n")
}
fn escape_markdown(value: &str) -> String {
    let mut out = String::new();
    for c in value.chars() {
        if "\\`*_[]{}()#+-.!|<>".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
fn format_entry(entry: &AggregatedLogEntry, options: &AggregatedLogOptions) -> String {
    let mut details = Vec::new();
    if options.show_author_names {
        details.push(format!("**{}**", escape_markdown(&entry.author_name)));
    }
    if options.show_message_time {
        details.push(entry.time.clone());
    }
    let marker = format!("<!-- discord-message-id: {} -->", entry.message_id);
    let markdown = entry
        .markdown
        .replace("<!-- discord-message-id:", "<!-- discord-message-id :")
        .replace("<!-- discord-message-date:", "<!-- discord-message-date :");
    if details.is_empty() {
        format!("{marker}\n{markdown}")
    } else {
        format!("{marker}\n{}\n\n{markdown}", details.join(" · "))
    }
}
fn append_to_date(content: &str, date: &str, block: &str) -> String {
    let sections: Vec<_> = DATE_SECTIONS
        .captures_iter(content)
        .filter(|c| c[1] == c[2])
        .collect();
    let Some(index) = sections.iter().position(|c| &c[1] == date) else {
        return append(
            content,
            &format!("<!-- discord-message-date: {date} -->\n## {date}\n\n{block}"),
        );
    };
    let Some(next) = sections.get(index + 1) else {
        return append(content, block);
    };
    let offset = next.get(0).unwrap().start();
    let before = append(&content[..offset], block);
    format!(
        "{before}{}{}",
        if before.ends_with("\n\n") { "" } else { "\n" },
        &content[offset..]
    )
}
pub fn merge(
    content: &str,
    entries: &[AggregatedLogEntry],
    options: &AggregatedLogOptions,
) -> AggregatedLogMergeResult {
    let mut ids: HashSet<_> = message_ids(content).into_iter().collect();
    let pending: Vec<_> = entries
        .iter()
        .filter(|e| ids.insert(e.message_id.clone()))
        .collect();
    let mut content = content.to_string();
    if !pending.is_empty() {
        if options.mode == AggregatedStorageMode::Daily {
            content = append(
                &content,
                &pending
                    .iter()
                    .map(|e| format_entry(e, options))
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            );
        } else {
            // Preserve first occurrence order, matching the historical JS Map.
            let mut dates: Vec<(&str, Vec<String>)> = Vec::new();
            for entry in &pending {
                let block = format_entry(entry, options);
                if let Some((_, blocks)) = dates.iter_mut().find(|(date, _)| *date == entry.date) {
                    blocks.push(block);
                } else {
                    dates.push((&entry.date, vec![block]));
                }
            }
            for (date, blocks) in dates {
                content = append_to_date(&content, date, &blocks.join("\n\n"));
            }
        }
    }
    AggregatedLogMergeResult {
        content,
        added_count: pending.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_ids_require_complete_ascii_markers_and_preserve_duplicate_ids() {
        let content = [
            "<!-- discord-message-id: 001 -->\r",
            "<!-- discord-message-id: 001 -->",
            " <!-- discord-message-id: 2 -->",
            "<!-- discord-message-id: 3 --> trailing",
            "<!-- discord-message-id: ４ -->",
            "<!-- discord-message-id:  -->",
            "<!-- discord-message-id: 5 -->\r\r",
            "text <!-- discord-message-id: 6 -->",
            "<!-- discord-message-id: 789 -->\r",
        ]
        .join("\n");
        assert_eq!(message_ids(&content), ["001", "001", "789"]);
    }

    #[test]
    fn managed_log_detection_only_skips_complete_frontmatter_and_blank_lines() {
        let marker = marker(AggregatedStorageMode::Daily);
        for prefix in [
            "",
            "\u{feff}",
            "---\n---\n",
            "\u{feff}---\r\nname: 日本語\r\n---\r\n \t\r\n",
            " \t\n\r\n",
        ] {
            assert!(
                is_managed(&format!("{prefix}{marker}")),
                "prefix: {prefix:?}"
            );
        }
        for prefix in [
            "---\nname: unclosed\n",
            "--- \n---\n",
            "\n---\n---\n",
            "---\n---\r\r\n",
            " ",
            "\u{a0}\n",
            "\u{feff}\u{feff}",
        ] {
            assert!(
                !is_managed(&format!("{prefix}{marker}")),
                "prefix: {prefix:?}"
            );
        }
    }

    #[test]
    fn log_insertion_preserves_utf8_offsets_and_ignores_malformed_date_sections() {
        for newline in ["\n", "\r\n"] {
            let prefix = [
                "<!-- discord-message-sender: weekly-log -->",
                "# 日本語 📒",
                "<!-- discord-message-date: 2026-07-01 -->",
                "## 2026-07-01",
                "<!-- discord-message-id: 1 -->",
                "old",
                "<!-- discord-message-date: 2026-07-02 -->",
                "## 2026-07-03",
                "<!-- discord-message-date: ２０２６-07-02 -->",
                "## ２０２６-07-02",
                "",
            ]
            .join(newline);
            let suffix =
                format!("<!-- discord-message-date: 2026-07-04 -->{newline}## 2026-07-04\r");
            let merged = merge(
                &format!("{prefix}{suffix}"),
                &[AggregatedLogEntry {
                    message_id: "2".into(),
                    date: "2026-07-01".into(),
                    time: "00:00".into(),
                    author_name: "".into(),
                    markdown: "new".into(),
                }],
                &AggregatedLogOptions {
                    mode: AggregatedStorageMode::Weekly,
                    show_author_names: false,
                    show_message_time: false,
                },
            );
            assert_eq!(merged.added_count, 1);
            assert_eq!(
                merged.content,
                format!("{prefix}\n<!-- discord-message-id: 2 -->\nnew\n\n{suffix}")
            );
        }
    }
}
