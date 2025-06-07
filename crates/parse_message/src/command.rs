pub mod url;

use crate::error::MessageError;
use wasm_bindgen::JsValue;

pub async fn handle_command(rest: &str) -> Result<(String, bool, String), JsValue> {
    let mut parts = rest.splitn(3, ' ');
    let cmd = parts.next().unwrap_or("");
    match cmd {
        "url" => url::handle(parts.next()).await,
        // add more commands here as needed
        _ => Err(MessageError::UnknownCommand.into()),
    }
}
