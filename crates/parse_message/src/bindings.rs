//! Typed JS boundary only. All decisions and transformations live in the domain crate.
use domain::{channels, dates, discord, logs, messages, models::*, settings, storage, sync};
use serde_json::Value;
use std::collections::BTreeMap;
use tsify::{Ts, Tsify};
use wasm_bindgen::{JsCast, prelude::*};

fn error(message: impl std::fmt::Display) -> JsError {
    JsError::new(&message.to_string())
}
fn raw(value: JsValue) -> Result<Value, JsError> {
    serde_wasm_bindgen::from_value(value).map_err(error)
}

// JsValue is unvalidated host data. Its generated TS type is unknown; runtime
// validation still happens below, before values enter the typed domain.

#[wasm_bindgen]
pub fn default_settings() -> Result<Ts<DiscordPluginSettings>, JsError> {
    Ok(DiscordPluginSettings::default().into_ts()?)
}
#[wasm_bindgen]
pub fn normalize_settings(
    #[wasm_bindgen(unchecked_param_type = "unknown")] value: JsValue,
) -> Result<Ts<DiscordPluginSettings>, JsError> {
    Ok(settings::normalize(&raw(value)?).into_ts()?)
}
#[wasm_bindgen]
pub fn migrate_settings(
    #[wasm_bindgen(unchecked_param_type = "unknown")] value: JsValue,
) -> Result<Ts<SettingsMigrationResult>, JsError> {
    Ok(settings::migrate(&raw(value)?).into_ts()?)
}
#[wasm_bindgen]
pub fn settings_snapshot(
    value: Ts<DiscordPluginSettings>,
    zone: String,
) -> Result<Ts<MessageSyncSettingsSnapshot>, JsError> {
    Ok(settings::snapshot(value.to_rust()?, zone).into_ts()?)
}
#[wasm_bindgen]
pub fn configured_channel_indices(value: Ts<ChannelList>) -> Result<Vec<u32>, JsError> {
    Ok(settings::configured_indices(&value.to_rust()?.0))
}
#[wasm_bindgen]
pub fn change_channel_id(
    value: Ts<DiscordChannelSettings>,
    id: String,
) -> Result<Ts<DiscordChannelSettings>, JsError> {
    Ok(settings::change_channel_id(value.to_rust()?, id).into_ts()?)
}
#[wasm_bindgen]
pub fn read_setting_control(
    value: Ts<DiscordPluginSettings>,
    key: &str,
) -> Result<Ts<SettingControlValue>, JsError> {
    Ok(settings::get_control(&value.to_rust()?, key).into_ts()?)
}
#[wasm_bindgen]
pub fn normalize_setting_control(
    key: &str,
    #[wasm_bindgen(unchecked_param_type = "unknown")] value: JsValue,
) -> Result<Ts<SettingControlValue>, JsValue> {
    let value = serde_wasm_bindgen::from_value(value)
        .map_err(|e| js_sys::TypeError::new(&e.to_string()))?;
    let result = settings::control_patch(key, &value).map_err(|e| js_sys::TypeError::new(&e))?;
    result.into_ts().map_err(|e| js_sys::TypeError::new(&e.to_string()).into())
}
#[wasm_bindgen]
pub fn trim_setting(value: &str) -> String {
    domain::trim(value).into()
}
#[wasm_bindgen]
pub fn channel_display_name(value: Ts<DiscordChannelSettings>) -> Result<String, JsError> {
    Ok(channels::display_name(&value.to_rust()?).into())
}
#[wasm_bindgen]
pub fn channel_path_segment(value: Ts<DiscordChannelSettings>) -> Result<String, JsError> {
    Ok(channels::path_segment(&value.to_rust()?))
}
#[wasm_bindgen]
pub fn channel_name_error(name: &str) -> Option<String> {
    channels::validation_error(name).map(str::to_string)
}
#[wasm_bindgen]
pub fn invalid_channel_name_message() -> String {
    channels::INVALID_NAME.into()
}
#[wasm_bindgen]
pub fn duplicate_channel_path(value: Ts<ChannelList>) -> Result<Option<String>, JsError> {
    Ok(channels::duplicate_path(&value.to_rust()?.0))
}
#[wasm_bindgen]
pub fn channel_directory(
    base: &str,
    channel: Ts<DiscordChannelSettings>,
) -> Result<String, JsError> {
    Ok(channels::directory(base, &channel.to_rust()?))
}
#[wasm_bindgen]
pub fn rename_channel(
    channels: Ts<ChannelList>,
    index: usize,
    name: &str,
) -> Result<String, JsError> {
    channels::rename(&channels.to_rust()?.0, index, name).map_err(error)
}
#[wasm_bindgen]
pub fn local_date_time(timestamp: &str, zone: &str) -> Result<Ts<LocalDateTime>, JsError> {
    Ok(dates::local(timestamp, zone).map_err(error)?.into_ts()?)
}
#[wasm_bindgen]
pub fn possible_local_dates(timestamp: &str) -> Result<Ts<LocalDateTimeList>, JsError> {
    Ok(LocalDateTimeList(dates::possible_dates(timestamp).map_err(error)?).into_ts()?)
}
#[wasm_bindgen]
pub fn message_instruction(
    input: &str,
    prefix: &str,
    clipping_already_saved: bool,
) -> Result<Ts<MessageInstruction>, JsError> {
    Ok(messages::instruction(input, prefix, clipping_already_saved)
        .map_err(error)?
        .into_ts()?)
}
#[wasm_bindgen]
pub fn complete_message_instruction(
    instruction: Ts<MessageInstruction>,
    message: Ts<DiscordMessage>,
    clipping_markdown: Option<String>,
    zone: &str,
) -> Result<Ts<ProcessedMessageList>, JsError> {
    Ok(messages::complete_instruction(
        instruction.to_rust()?,
        message.to_rust()?,
        clipping_markdown,
        zone,
    )
    .map_err(error)?
    .into_ts()?)
}
#[wasm_bindgen]
pub fn processed_message(
    markdown: String,
    clipping: bool,
    message: Ts<DiscordMessage>,
    zone: &str,
) -> Result<Ts<ProcessedMessage>, JsError> {
    Ok(
        messages::processed(markdown, clipping, message.to_rust()?, zone)
            .map_err(error)?
            .into_ts()?,
    )
}
#[wasm_bindgen]
pub fn should_process_message(message: Ts<DiscordMessage>) -> Result<bool, JsError> {
    Ok(sync::should_process(&message.to_rust()?))
}
#[wasm_bindgen]
pub fn aggregated_log_marker(mode: Ts<logs::AggregatedStorageMode>) -> Result<String, JsError> {
    Ok(logs::marker(mode.to_rust()?))
}
#[wasm_bindgen]
pub fn create_aggregated_log(
    mode: Ts<logs::AggregatedStorageMode>,
    period: &str,
) -> Result<String, JsError> {
    Ok(logs::create(mode.to_rust()?, period))
}
#[wasm_bindgen]
pub fn has_aggregated_log_marker(
    content: &str,
    mode: Ts<logs::AggregatedStorageMode>,
) -> Result<bool, JsError> {
    Ok(logs::has_marker(content, mode.to_rust()?))
}
#[wasm_bindgen]
pub fn is_managed_log(content: &str) -> bool {
    logs::is_managed(content)
}
#[wasm_bindgen]
pub fn aggregated_message_ids(content: &str) -> Vec<String> {
    logs::message_ids(content)
}
#[wasm_bindgen]
pub fn merge_aggregated_log(
    content: &str,
    entries: Ts<logs::LogEntries>,
    options: Ts<logs::AggregatedLogOptions>,
) -> Result<Ts<logs::AggregatedLogMergeResult>, JsError> {
    Ok(logs::merge(content, &entries.to_rust()?.0, &options.to_rust()?).into_ts()?)
}
#[wasm_bindgen]
pub fn individual_message_id(name: &str) -> Option<String> {
    storage::individual_id(name)
}
#[wasm_bindgen]
pub fn storage_candidate_paths(input: Ts<storage::StorageInput>) -> Result<Vec<String>, JsError> {
    storage::candidate_paths(&input.to_rust()?).map_err(error)
}
#[wasm_bindgen]
pub fn plan_message_storage(
    input: Ts<storage::StorageInput>,
) -> Result<Ts<storage::StoragePlan>, JsError> {
    Ok(storage::plan(input.to_rust()?).map_err(error)?.into_ts()?)
}
#[wasm_bindgen]
pub fn select_message_page(
    messages: Ts<MessageList>,
    cursor: Option<String>,
) -> Result<Ts<sync::PageSelection>, JsError> {
    Ok(sync::select_page(messages.to_rust()?.0, cursor.as_deref())
        .map_err(error)?
        .into_ts()?)
}
#[wasm_bindgen]
pub fn sync_batches(pages: Ts<sync::MessagePages>) -> Result<Ts<sync::SyncBatches>, JsError> {
    Ok(sync::SyncBatches(sync::batches(pages.to_rust()?.0)).into_ts()?)
}
#[wasm_bindgen]
pub fn discord_messages_path(channel: &str, before: Option<String>) -> String {
    discord::messages_path(channel, before.as_deref())
}
#[wasm_bindgen]
pub fn decode_discord_messages(json: &str) -> Result<Ts<MessageList>, JsError> {
    let messages: MessageList = serde_json::from_str(json)
        .map_err(|_| error("Discord API returned an invalid message list."))?;
    Ok(messages.into_ts()?)
}
#[wasm_bindgen]
pub fn decode_discord_message(json: &str) -> Result<Ts<DiscordMessage>, JsError> {
    let message: DiscordMessage = serde_json::from_str(json)
        .map_err(|_| error("Discord API returned an invalid message."))?;
    Ok(message.into_ts()?)
}
#[wasm_bindgen]
pub fn discord_create_message_body(content: &str) -> String {
    discord::create_message_body(content)
}
#[wasm_bindgen]
pub fn discord_api_version() -> u32 {
    discord::API_VERSION
}
#[wasm_bindgen]
pub fn discord_page_size() -> usize {
    discord::PAGE_SIZE
}
// Obsidian can return array-valued headers such as Set-Cookie at runtime.
// Read only the scalar headers used by the core; never deserialize cookie values.
fn rate_limit_headers(value: &JsValue) -> BTreeMap<String, String> {
    let mut headers = BTreeMap::new();
    let Some(object) = value.dyn_ref::<js_sys::Object>() else {
        return headers;
    };
    for key in js_sys::Object::keys(object).iter() {
        let Some(name) = key.as_string() else {
            continue;
        };
        if ![
            "Retry-After",
            "X-RateLimit-Remaining",
            "X-RateLimit-Reset-After",
        ]
        .iter()
        .any(|expected| name.eq_ignore_ascii_case(expected))
        {
            continue;
        }
        if let Some(text) = js_sys::Reflect::get(value, &key)
            .ok()
            .and_then(|value| value.as_string())
        {
            headers.insert(name, text);
        }
    }
    headers
}

#[wasm_bindgen]
pub fn discord_rate_limit_delay(
    #[wasm_bindgen(unchecked_param_type = "unknown")] headers: JsValue,
    text: &str,
) -> Result<f64, JsError> {
    Ok(discord::rate_limit_delay(
        &rate_limit_headers(&headers),
        text,
    ))
}
#[wasm_bindgen]
pub fn discord_reset_delay(
    #[wasm_bindgen(unchecked_param_type = "unknown")] headers: JsValue,
) -> Result<f64, JsError> {
    Ok(discord::reset_delay(&rate_limit_headers(&headers)))
}
#[wasm_bindgen]
pub fn discord_failure_notice(status: u32, method: &str) -> String {
    discord::failure_notice(status, method)
}
#[wasm_bindgen]
pub fn discord_network_error_message(method: &str, path: &str) -> String {
    discord::network_error_message(method, path)
}
#[wasm_bindgen]
pub fn discord_rate_limit_notice(delay: f64) -> String {
    discord::rate_limit_notice(delay)
}
#[wasm_bindgen]
pub fn discord_error_message(status: u32, method: &str, path: &str, text: &str) -> String {
    discord::error_message(status, method, path, text)
}
#[wasm_bindgen]
pub fn render_notification(
    template: &str,
    channel: Ts<DiscordChannelSettings>,
    count: usize,
) -> Result<String, JsError> {
    Ok(discord::notification(template, &channel.to_rust()?, count))
}
#[wasm_bindgen]
pub fn sync_completion_notice(count: usize, failures: usize) -> String {
    discord::completion(count, failures)
}
#[wasm_bindgen]
pub fn discord_retry_decision(
    status: Option<u32>,
    attempt: u32,
    #[wasm_bindgen(unchecked_param_type = "unknown")] headers: JsValue,
    text: &str,
) -> Result<Ts<discord::RetryDecision>, JsError> {
    Ok(discord::retry(status, attempt, &rate_limit_headers(&headers), text).into_ts()?)
}

#[wasm_bindgen]
pub fn prepare_sync(
    settings: Ts<DiscordPluginSettings>,
    zone: String,
) -> Result<Ts<sync::SyncPreparation>, JsError> {
    Ok(sync::prepare(settings.to_rust()?, zone)
        .map_err(error)?
        .into_ts()?)
}
#[wasm_bindgen]
pub fn sync_notification_text(
    templates: Ts<NotificationTemplates>,
    channel: Ts<DiscordChannelSettings>,
    count: usize,
) -> Result<String, JsError> {
    Ok(sync::notification_text(
        &templates.to_rust()?,
        &channel.to_rust()?,
        count,
    ))
}
#[wasm_bindgen]
pub fn sync_failure_notice(
    channel: Ts<DiscordChannelSettings>,
    reason: &str,
) -> Result<String, JsError> {
    Ok(sync::failure_notice(&channel.to_rust()?, reason))
}
#[wasm_bindgen]
pub fn sync_notification_failure_notice(
    channel: Ts<DiscordChannelSettings>,
    reason: &str,
) -> Result<String, JsError> {
    Ok(sync::notification_failure_notice(
        &channel.to_rust()?,
        reason,
    ))
}
