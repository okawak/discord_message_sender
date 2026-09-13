use super::{models::*, trim};
use serde_json::Value;
use std::collections::HashSet;

pub const SCHEMA_VERSION: u32 = 3;
impl Default for NotificationTemplates {
    fn default() -> Self {
        Self {
            saved: "✅ {count} messages saved.".into(),
            no_new: "⚠️ No new messages.".into(),
        }
    }
}
impl Default for DiscordPluginSettings {
    fn default() -> Self {
        Self {
            settings_version: SCHEMA_VERSION,
            message_directory_name: "DiscordLogs".into(),
            clipping_directory_name: "DiscordClippings".into(),
            bot_token: String::new(),
            channels: vec![],
            message_prefix: "!".into(),
            message_storage_mode: MessageStorageMode::Individual,
            show_author_names: false,
            show_message_time: false,
            enable_auto_sync_on_startup: true,
            send_sync_notifications: true,
            notification_templates: NotificationTemplates::default(),
        }
    }
}
fn read_string(value: &Value, key: &str) -> String {
    trim(value[key].as_str().unwrap_or("")).into()
}
fn string_or(value: &Value, key: &str, fallback: &str) -> String {
    let text = read_string(value, key);
    if text.is_empty() {
        fallback.into()
    } else {
        text
    }
}
pub fn normalize(raw: &Value) -> DiscordPluginSettings {
    let defaults = DiscordPluginSettings::default();
    let mut channels = if let Some(channels) = raw["channels"].as_array() {
        channels
            .iter()
            .filter(|c| c.is_object())
            .map(|c| {
                let cursor = read_string(c, "lastProcessedMessageId");
                DiscordChannelSettings {
                    id: read_string(c, "id"),
                    name: read_string(c, "name"),
                    last_processed_message_id: (!cursor.is_empty()).then_some(cursor),
                }
            })
            .collect::<Vec<_>>()
    } else {
        let cursor = read_string(raw, "lastProcessedMessageId");
        vec![DiscordChannelSettings {
            id: read_string(raw, "channelId"),
            name: String::new(),
            last_processed_message_id: (!cursor.is_empty()).then_some(cursor),
        }]
    };
    let mut ids = HashSet::new();
    channels.retain(|c| !c.id.is_empty() && ids.insert(c.id.clone()));
    DiscordPluginSettings {
        channels,
        message_directory_name: string_or(
            raw,
            "messageDirectoryName",
            &defaults.message_directory_name,
        ),
        clipping_directory_name: string_or(
            raw,
            "clippingDirectoryName",
            &defaults.clipping_directory_name,
        ),
        bot_token: read_string(raw, "botToken"),
        message_prefix: string_or(raw, "messagePrefix", &defaults.message_prefix),
        message_storage_mode: serde_json::from_value(raw["messageStorageMode"].clone())
            .unwrap_or(MessageStorageMode::Individual),
        show_author_names: raw["showAuthorNames"].as_bool().unwrap_or(false),
        show_message_time: raw["showMessageTime"].as_bool().unwrap_or(false),
        enable_auto_sync_on_startup: raw["enableAutoSyncOnStartup"].as_bool().unwrap_or(true),
        send_sync_notifications: raw["sendSyncNotifications"].as_bool().unwrap_or(true),
        notification_templates: NotificationTemplates {
            saved: string_or(
                &raw["notificationTemplates"],
                "saved",
                &defaults.notification_templates.saved,
            ),
            no_new: string_or(
                &raw["notificationTemplates"],
                "noNew",
                &defaults.notification_templates.no_new,
            ),
        },
        ..defaults
    }
}
pub fn migrate(raw: &Value) -> SettingsMigrationResult {
    SettingsMigrationResult {
        settings: normalize(raw),
        did_migrate: raw.as_object().is_some_and(|v| {
            raw["settingsVersion"] != SCHEMA_VERSION
                || v.contains_key("channelId")
                || v.contains_key("lastProcessedMessageId")
        }),
    }
}
pub fn snapshot(s: DiscordPluginSettings, time_zone: String) -> MessageSyncSettingsSnapshot {
    MessageSyncSettingsSnapshot {
        bot_token: s.bot_token,
        message_directory_name: s.message_directory_name,
        clipping_directory_name: s.clipping_directory_name,
        message_prefix: s.message_prefix,
        message_storage_mode: s.message_storage_mode,
        show_author_names: s.show_author_names,
        show_message_time: s.show_message_time,
        send_sync_notifications: s.send_sync_notifications,
        notification_templates: s.notification_templates,
        time_zone,
    }
}
pub fn configured_indices(channels: &[DiscordChannelSettings]) -> Vec<u32> {
    let mut ids = HashSet::new();
    channels
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.id.is_empty() && ids.insert(&c.id))
        .map(|(i, _)| i as u32)
        .collect()
}
pub fn change_channel_id(
    mut channel: DiscordChannelSettings,
    id: String,
) -> DiscordChannelSettings {
    if channel.id != id {
        channel.id = id;
        channel.last_processed_message_id = None;
    }
    channel
}
pub fn get_control(settings: &DiscordPluginSettings, key: &str) -> SettingControlValue {
    let value = match key {
        "savedNotificationTemplate" => Value::String(settings.notification_templates.saved.clone()),
        "noNewNotificationTemplate" => {
            Value::String(settings.notification_templates.no_new.clone())
        }
        _ => serde_json::to_value(settings).expect("settings serialize")[key].clone(),
    };
    serde_json::from_value(value).unwrap_or(SettingControlValue::Unset)
}
/// Returns only the changed field, so the host can preserve channel object identity.
pub fn control_patch(key: &str, value: &Value) -> Result<SettingControlValue, String> {
    let defaults = DiscordPluginSettings::default();
    let fallback = match key {
        "messageDirectoryName" => Some(defaults.message_directory_name),
        "clippingDirectoryName" => Some(defaults.clipping_directory_name),
        "messagePrefix" => Some(defaults.message_prefix),
        "savedNotificationTemplate" => Some(defaults.notification_templates.saved),
        "noNewNotificationTemplate" => Some(defaults.notification_templates.no_new),
        "botToken" => Some(String::new()),
        _ => None,
    };
    if let Some(fallback) = fallback {
        let text = trim(value.as_str().unwrap_or(""));
        return Ok(SettingControlValue::Text(if text.is_empty() {
            fallback
        } else {
            text.into()
        }));
    }
    match key {
        "messageStorageMode" => {
            let Value::String(mode) = value else {
                return Err("Invalid message storage mode.".into());
            };
            serde_json::from_value::<MessageStorageMode>(value.clone())
                .map_err(|_| "Invalid message storage mode.".to_string())?;
            Ok(SettingControlValue::Text(mode.clone()))
        }
        "showAuthorNames"
        | "showMessageTime"
        | "enableAutoSyncOnStartup"
        | "sendSyncNotifications" => {
            value
                .as_bool()
                .map(SettingControlValue::Toggle)
                .ok_or_else(|| format!("Setting \"{key}\" requires a boolean value."))
        }
        _ => Err(format!("Unknown setting key \"{key}\".")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn channel(id: &str, name: &str) -> DiscordChannelSettings {
        DiscordChannelSettings {
            id: id.into(),
            name: name.into(),
            last_processed_message_id: None,
        }
    }

    #[test]
    fn cursor_is_preserved_only_when_channel_id_is_unchanged() {
        let mut channel = channel("1", "notes");
        channel.last_processed_message_id = Some("99".into());
        assert_eq!(change_channel_id(channel.clone(), "1".into()), channel);
        assert!(
            change_channel_id(channel, "2".into())
                .last_processed_message_id
                .is_none()
        );
    }

    #[test]
    fn controls_validate_without_resetting_other_settings() {
        assert_eq!(
            control_patch("messageDirectoryName", &json!(" \u{feff} ")).unwrap(),
            SettingControlValue::Text("DiscordLogs".into())
        );
        assert_eq!(
            control_patch("sendSyncNotifications", &json!(false)).unwrap(),
            SettingControlValue::Toggle(false)
        );
        assert!(control_patch("sendSyncNotifications", &json!("false")).is_err());
        assert!(control_patch("messageStorageMode", &json!("yearly")).is_err());
        assert!(control_patch("channels", &json!([])).is_err());
    }

    #[test]
    fn control_values_match_their_declared_primitive_types() {
        let settings = DiscordPluginSettings::default();
        assert_eq!(
            get_control(&settings, "messageDirectoryName"),
            SettingControlValue::Text("DiscordLogs".into()),
        );
        assert_eq!(
            get_control(&settings, "showAuthorNames"),
            SettingControlValue::Toggle(false),
        );
        assert_eq!(get_control(&settings, "missing"), SettingControlValue::Unset);
        for mode in ["individual", "daily", "weekly", "monthly"] {
            assert_eq!(
                control_patch("messageStorageMode", &json!(mode)).unwrap(),
                SettingControlValue::Text(mode.into()),
            );
        }
    }
}
