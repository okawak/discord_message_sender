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

/// Complete the domain decision after the host has fetched/converted a URL,
/// returning only messages that should be saved (zero or one per instruction).
pub fn complete_instruction(
    instruction: MessageInstruction,
    message: DiscordMessage,
    clipping_markdown: Option<String>,
    zone: &str,
) -> Result<ProcessedMessageList, String> {
    let (markdown, is_clipping) = match instruction {
        MessageInstruction::Skip => return Ok(ProcessedMessageList(Vec::new())),
        MessageInstruction::Message { markdown } => (markdown, false),
        MessageInstruction::Url { .. } => (
            clipping_markdown.ok_or("URL instructions require converted Markdown.")?,
            true,
        ),
    };
    // Preserve metadata validation even when the converted body is empty.
    let result = processed(markdown, is_clipping, message, zone)?;
    Ok(ProcessedMessageList(if result.markdown.is_empty() {
        Vec::new()
    } else {
        vec![result]
    }))
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

    fn message(timestamp: &str) -> DiscordMessage {
        DiscordMessage {
            id: "123".into(),
            content: "original content".into(),
            timestamp: timestamp.into(),
            author: None,
            member: None,
        }
    }

    #[test]
    fn completes_regular_and_url_instructions_using_the_correct_body() {
        let regular = complete_instruction(
            MessageInstruction::Message {
                markdown: "regular".into(),
            },
            message("2026-07-01T00:00:00Z"),
            Some("unused clipping".into()),
            "UTC",
        )
        .unwrap()
        .0;
        assert_eq!(regular.len(), 1);
        assert_eq!(regular[0].markdown, "regular");
        assert!(!regular[0].is_clipping);
        assert_eq!(regular[0].file_name, "20260701_000000_123");

        let url = || MessageInstruction::Url {
            url: "https://example.com".into(),
        };
        let clipping = complete_instruction(
            url(),
            message("2026-07-01T00:00:00Z"),
            Some("converted".into()),
            "UTC",
        )
        .unwrap()
        .0;
        assert_eq!(clipping.len(), 1);
        assert_eq!(clipping[0].markdown, "converted");
        assert!(clipping[0].is_clipping);
        assert!(complete_instruction(url(), message("2026-07-01T00:00:00Z"), None, "UTC").is_err());
    }

    #[test]
    fn skips_saved_clippings_without_validating_unused_metadata() {
        assert!(
            complete_instruction(
                MessageInstruction::Skip,
                message("invalid"),
                None,
                "Invalid/Zone",
            )
            .unwrap()
            .0
            .is_empty()
        );
    }

    #[test]
    fn omits_empty_bodies_but_preserves_metadata_errors() {
        for instruction in [
            MessageInstruction::Message {
                markdown: String::new(),
            },
            MessageInstruction::Url {
                url: "https://example.com".into(),
            },
        ] {
            assert!(
                complete_instruction(
                    instruction.clone(),
                    message("2026-07-01T00:00:00Z"),
                    Some(String::new()),
                    "UTC",
                )
                .unwrap()
                .0
                .is_empty()
            );
            assert!(
                complete_instruction(instruction, message("invalid"), Some(String::new()), "UTC",)
                    .is_err()
            );
        }
    }
}
