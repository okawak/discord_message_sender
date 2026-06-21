pub mod url;

use crate::error::MessageError;
use wasm_bindgen::JsValue;

pub async fn handle_command(
    rest: &str,
    timestamp: &str,
) -> Result<(String, bool, String), JsValue> {
    let (cmd, arg) = rest
        .split_once(char::is_whitespace)
        .map_or((rest, None), |(cmd, arg)| (cmd, Some(arg.trim_start())));

    match cmd {
        "url" => url::handle(arg, timestamp).await,
        // add more commands here as needed
        _ => Err(MessageError::UnknownCommand.into()),
    }
}
