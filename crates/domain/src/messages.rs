use super::{
    command::{MessageAction, parse_message},
    dates,
    models::*,
    trim,
};

pub fn instruction(
    input: &str,
    prefix: &str,
    clipping_already_saved: bool,
) -> Result<MessageInstruction, String> {
    match parse_message(input, prefix).map_err(|e| e.to_string())? {
        MessageAction::Message(markdown) => Ok(MessageInstruction::Message { markdown }),
        MessageAction::Url(_) if clipping_already_saved => Ok(MessageInstruction::Skip),
        MessageAction::Url(url) => Ok(MessageInstruction::Url { url }),
    }
}
pub fn processed(
    markdown: String,
    is_clipping: bool,
    message: DiscordMessage,
    zone: &str,
) -> Result<ProcessedMessage, String> {
    let author = message.author.as_ref();
    let name = [
        message.member.as_ref().and_then(|m| m.nick.as_deref()),
        author.and_then(|a| a.global_name.as_deref()),
        author.and_then(|a| a.username.as_deref()),
    ]
    .into_iter()
    .flatten()
    .map(trim)
    .find(|s| !s.is_empty())
    .or_else(|| {
        author
            .and_then(|a| a.id.as_deref())
            .filter(|s| !s.is_empty())
    })
    .unwrap_or("Unknown")
    .to_string();
    Ok(ProcessedMessage {
        file_name: format!(
            "{}_{}",
            dates::local(&message.timestamp, zone)?.file_timestamp,
            message.id
        ),
        author_id: author.and_then(|a| a.id.clone()).unwrap_or_default(),
        author_name: name,
        message_id: message.id,
        timestamp: message.timestamp,
        markdown,
        is_clipping,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_only_saved_url_commands() {
        assert!(matches!(
            instruction("!url https://example.com", "!", true).unwrap(),
            MessageInstruction::Skip
        ));
        assert!(matches!(
            instruction("!url https://example.com", "?", true).unwrap(),
            MessageInstruction::Message { .. }
        ));
        assert!(matches!(
            instruction("regular message", "!", true).unwrap(),
            MessageInstruction::Message { .. }
        ));
    }
}
