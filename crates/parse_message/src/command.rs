use thiserror::Error;

#[derive(Debug, PartialEq, Eq)]
pub enum MessageAction {
    Message(String),
    Url(String),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CommandError {
    #[error("Command is missing")]
    MissingCommand,
    #[error("URL command requires a URL")]
    MissingUrl,
    #[error("Unknown command: {0}")]
    UnknownCommand(String),
}

pub fn parse_message(input: &str, prefix: &str) -> Result<MessageAction, CommandError> {
    let input = input.trim();
    let prefix = prefix.trim();

    let Some(rest) = input.strip_prefix(prefix) else {
        return Ok(MessageAction::Message(input.to_owned()));
    };
    parse_command(rest.trim_start())
}

fn parse_command(rest: &str) -> Result<MessageAction, CommandError> {
    let (cmd, arg) = rest
        .split_once(char::is_whitespace)
        .map_or((rest, None), |(cmd, arg)| (cmd, Some(arg.trim())));

    match cmd {
        "" => Err(CommandError::MissingCommand),
        "url" => arg
            .filter(|url| !url.is_empty())
            .map(|url| MessageAction::Url(url.to_owned()))
            .ok_or(CommandError::MissingUrl),
        _ => Err(CommandError::UnknownCommand(cmd.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::{CommandError, MessageAction, parse_message};

    #[test]
    fn parses_regular_messages() {
        assert_eq!(
            parse_message("  hello  ", "!"),
            Ok(MessageAction::Message("hello".to_owned()))
        );
    }

    #[test]
    fn parses_url_commands() {
        assert_eq!(
            parse_message("!url  https://example.com  ", "!"),
            Ok(MessageAction::Url("https://example.com".to_owned()))
        );
    }

    #[test]
    fn rejects_url_commands_without_an_argument() {
        assert_eq!(parse_message("!url", "!"), Err(CommandError::MissingUrl));
    }

    #[test]
    fn rejects_unknown_commands() {
        assert_eq!(
            parse_message("!task something", "!"),
            Err(CommandError::UnknownCommand("task".to_owned()))
        );
    }

    #[test]
    fn rejects_empty_commands() {
        assert_eq!(parse_message("!", "!"), Err(CommandError::MissingCommand));
    }
}
