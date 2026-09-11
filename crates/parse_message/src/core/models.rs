use serde::{Deserialize, Serialize};
use tsify::Tsify;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct DiscordChannelSettings {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_processed_message_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct NotificationTemplates {
    pub saved: String,
    pub no_new: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "lowercase")]
pub enum MessageStorageMode {
    Individual,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct DiscordPluginSettings {
    pub settings_version: u32,
    pub message_directory_name: String,
    pub clipping_directory_name: String,
    pub bot_token: String,
    pub channels: Vec<DiscordChannelSettings>,
    pub message_prefix: String,
    pub message_storage_mode: MessageStorageMode,
    pub show_author_names: bool,
    pub show_message_time: bool,
    pub enable_auto_sync_on_startup: bool,
    pub send_sync_notifications: bool,
    pub notification_templates: NotificationTemplates,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct SettingsMigrationResult {
    pub settings: DiscordPluginSettings,
    pub did_migrate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct MessageSyncSettingsSnapshot {
    pub bot_token: String,
    pub message_directory_name: String,
    pub clipping_directory_name: String,
    pub message_prefix: String,
    pub message_storage_mode: MessageStorageMode,
    pub show_author_names: bool,
    pub show_message_time: bool,
    pub send_sync_notifications: bool,
    pub notification_templates: NotificationTemplates,
    pub time_zone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
pub struct DiscordAuthor {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(type = "string | null")]
    pub global_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
pub struct DiscordMember {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(type = "string | null")]
    pub nick: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
pub struct DiscordMessage {
    pub id: String,
    pub content: String,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<DiscordAuthor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<DiscordMember>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct ProcessedMessage {
    pub message_id: String,
    pub timestamp: String,
    pub author_id: String,
    pub author_name: String,
    pub markdown: String,
    pub is_clipping: bool,
    pub file_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum MessageInstruction {
    Message { markdown: String },
    Url { url: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct LocalDateTime {
    pub date: String,
    pub month: String,
    pub week: String,
    pub time: String,
    pub file_timestamp: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(transparent)]
pub struct ChannelList(pub Vec<DiscordChannelSettings>);
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(transparent)]
pub struct MessageList(pub Vec<DiscordMessage>);
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(transparent)]
pub struct ProcessedMessageList(pub Vec<ProcessedMessage>);
#[derive(Debug, Clone, Serialize, Deserialize, Tsify)]
#[serde(transparent)]
pub struct LocalDateTimeList(pub Vec<LocalDateTime>);
