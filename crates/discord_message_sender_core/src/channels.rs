use super::{js_whitespace, models::DiscordChannelSettings, trim};
use std::collections::HashSet;
#[cfg(not(target_arch = "wasm32"))]
use unicode_normalization::UnicodeNormalization;

pub const INVALID_NAME: &str = "Channel name is invalid. Forbidden characters are: \\ / : * ? \" < > | # ^ [ ]. The names \".\" and \"..\" are also not allowed.";
pub fn display_name(channel: &DiscordChannelSettings) -> &str {
    if channel.name.is_empty() {
        &channel.id
    } else {
        &channel.name
    }
}
pub fn validation_error(name: &str) -> Option<&'static str> {
    (name == "."
        || name == ".."
        || name.contains([
            '\\', '/', ':', '*', '?', '"', '<', '>', '|', '#', '^', '[', ']',
        ]))
    .then_some(INVALID_NAME)
}
fn sanitize(value: &str) -> String {
    let mut result = String::new();
    let mut unsafe_run = false;
    let mut space_run = false;
    for c in trim(value).chars() {
        let unsafe_char = "\\/:*?\"<>|#^".contains(c);
        if unsafe_char {
            if !unsafe_run {
                result.push('-');
            }
        } else if c == '[' || c == ']' {
            result.push('-');
        } else if js_whitespace(c) {
            if !space_run {
                result.push(' ');
            }
        } else {
            result.push(c);
        }
        unsafe_run = unsafe_char;
        space_run = js_whitespace(c);
    }
    result.trim_matches('-').into()
}
pub fn path_segment(channel: &DiscordChannelSettings) -> String {
    let name = sanitize(&channel.name);
    if !name.is_empty() && name != "." && name != ".." {
        return name;
    }
    let id = sanitize(&channel.id);
    if id.is_empty() { "channel".into() } else { id }
}
fn canonical_path(value: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::JsString::from(value)
            .normalize("NFC")
            .to_lower_case()
            .into()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        value.nfc().collect::<String>().to_lowercase()
    }
}
pub fn duplicate_path(channels: &[DiscordChannelSettings]) -> Option<String> {
    let mut paths = HashSet::new();
    for channel in channels {
        let path = path_segment(channel);
        if !paths.insert(canonical_path(&path)) {
            return Some(path);
        }
    }
    None
}
pub fn directory(base: &str, channel: &DiscordChannelSettings) -> String {
    let base = base.trim_end_matches('/');
    let segment = path_segment(channel);
    if base.is_empty() {
        segment
    } else {
        format!("{base}/{segment}")
    }
}
pub fn rename(
    channels: &[DiscordChannelSettings],
    index: usize,
    value: &str,
) -> Result<String, String> {
    let name = trim(value);
    if let Some(error) = validation_error(name) {
        return Err(error.into());
    }
    let mut updated = channels.to_vec();
    updated
        .get_mut(index)
        .ok_or("Channel no longer exists.")?
        .name = name.into();
    if let Some(duplicate) = duplicate_path(&updated) {
        return Err(format!(
            "Channel name is already in use as folder \"{duplicate}\". Enter a unique channel name."
        ));
    }
    Ok(name.into())
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
    fn channel_rename_rejects_collisions_without_mutating_original_data() {
        let channels = vec![channel("1", "first"), channel("2", "second")];
        assert!(rename(&channels, 1, "FIRST").is_err());
        assert!(rename(&channels, 1, "../bad").is_err());
        assert_eq!(rename(&channels, 1, "  new  ").unwrap(), "new");
        assert_eq!(channels[1].name, "second");
    }
}
