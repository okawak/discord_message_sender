use super::{discord::PAGE_SIZE, models::DiscordMessage};
use serde::{Deserialize, Serialize};
use tsify::Tsify;

#[derive(Debug, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct PageSelection {
    pub messages: Vec<DiscordMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
}
// Compare decimal snowflakes without passing them through JS numbers or imposing u64 limits.
fn decimal(id: &str) -> Result<&str, String> {
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("Invalid Discord message ID: {id}"));
    }
    Ok(id.trim_start_matches('0'))
}
pub fn select_page(
    messages: Vec<DiscordMessage>,
    cursor: Option<&str>,
) -> Result<PageSelection, String> {
    let selected = if let Some(cursor) = cursor.filter(|c| !c.is_empty()) {
        let cursor = decimal(cursor)?;
        messages
            .iter()
            .filter_map(|m| match decimal(&m.id) {
                Ok(id) => ((id.len(), id) > (cursor.len(), cursor)).then(|| Ok(m.clone())),
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        messages.clone()
    };
    let before = if cursor.is_some_and(|c| !c.is_empty())
        && messages.len() == PAGE_SIZE
        && selected.len() == messages.len()
    {
        messages.last().map(|m| m.id.clone())
    } else {
        None
    };
    Ok(PageSelection {
        messages: selected,
        before,
    })
}
#[derive(Debug, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct SyncBatch {
    pub messages: Vec<DiscordMessage>,
    pub cursor: String,
}
#[derive(Debug, Serialize, Deserialize, Tsify)]
#[serde(transparent)]
pub struct MessagePages(pub Vec<Vec<DiscordMessage>>);
#[derive(Debug, Serialize, Deserialize, Tsify)]
#[serde(transparent)]
pub struct SyncBatches(pub Vec<SyncBatch>);
pub fn batches(pages: Vec<Vec<DiscordMessage>>) -> Vec<SyncBatch> {
    pages
        .into_iter()
        .rev()
        .filter_map(|mut messages| {
            let cursor = messages.first()?.id.clone();
            messages.reverse();
            Some(SyncBatch { messages, cursor })
        })
        .collect()
}
pub fn should_process(message: &DiscordMessage) -> bool {
    !message.author.as_ref().is_some_and(|a| a.bot == Some(true))
}

#[derive(Debug, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct SyncPreparation {
    pub channel_indices: Vec<u32>,
    pub settings: super::models::MessageSyncSettingsSnapshot,
}
pub fn prepare(
    settings: super::models::DiscordPluginSettings,
    zone: String,
) -> Result<SyncPreparation, String> {
    let indices = super::settings::configured_indices(&settings.channels);
    if settings.bot_token.is_empty() || indices.is_empty() {
        return Err("Discord message sender: bot token or channel is not configured.".into());
    }
    let channels: Vec<_> = indices
        .iter()
        .map(|&i| settings.channels[i as usize].clone())
        .collect();
    for channel in &channels {
        if let Some(error) = super::channels::validation_error(&channel.name) {
            return Err(error.into());
        }
    }
    if let Some(path) = super::channels::duplicate_path(&channels) {
        return Err(format!(
            "Discord message sender: duplicate channel folder \"{path}\". Use unique channel names."
        ));
    }
    Ok(SyncPreparation {
        channel_indices: indices,
        settings: super::settings::snapshot(settings, zone),
    })
}
pub fn notification_text(
    settings: &super::models::NotificationTemplates,
    channel: &super::models::DiscordChannelSettings,
    count: usize,
) -> String {
    super::discord::notification(
        if count == 0 {
            &settings.no_new
        } else {
            &settings.saved
        },
        channel,
        count,
    )
}
pub fn failure_notice(channel: &super::models::DiscordChannelSettings, reason: &str) -> String {
    format!(
        "Discord sync skipped \"{}\": {reason}.",
        super::channels::display_name(channel)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{models::DiscordPluginSettings, settings};
    use serde_json::json;

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
    fn large_snowflakes_are_compared_without_number_precision_loss() {
        let result = select_page(
            vec![
                message("1520291078606028901"),
                message("1520291078606028900"),
            ],
            Some("1520291078606028900"),
        )
        .unwrap();
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.messages[0].id, "1520291078606028901");
        assert!(result.before.is_none());
        assert!(select_page(vec![message("invalid")], Some("1")).is_err());
    }

    #[test]
    fn pagination_stops_at_cursor_and_keeps_full_page_continuation() {
        let page: Vec<_> = (101..=200)
            .rev()
            .map(|id| message(&id.to_string()))
            .collect();
        assert_eq!(
            select_page(page.clone(), Some("100"))
                .unwrap()
                .before
                .as_deref(),
            Some("101")
        );
        assert!(
            select_page(page.clone(), Some("150"))
                .unwrap()
                .before
                .is_none()
        );
        assert!(select_page(page, None).unwrap().before.is_none());
        let batches = batches(vec![
            vec![message("4"), message("3")],
            vec![message("2"), message("1")],
        ]);
        assert_eq!(
            batches
                .iter()
                .map(|b| b.cursor.as_str())
                .collect::<Vec<_>>(),
            vec!["2", "4"]
        );
        assert_eq!(batches[0].messages[0].id, "1");
    }

    #[test]
    fn sync_preparation_rejects_missing_credentials_and_duplicate_folders() {
        assert!(prepare(DiscordPluginSettings::default(), "UTC".into()).is_err());
        let settings = settings::normalize(
            &json!({"botToken":"dummy", "channels":[{"id":"1", "name":"x"},{"id":"2", "name":"X"}]}),
        );
        assert!(
            prepare(settings, "UTC".into())
                .unwrap_err()
                .contains("duplicate channel folder")
        );
    }
}
