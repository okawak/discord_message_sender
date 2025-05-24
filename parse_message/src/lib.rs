pub mod command;
mod error;
mod message;

use command::handle_command;
use message::ParseMessageResult;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn process_message(
    input: String,
    prefix: String,
    timestamp: String,
) -> Result<JsValue, JsValue> {
    let input = input.trim();
    let prefix = prefix.trim();

    // "!url hoge".strip_prefix("!") -> "url hoge"
    if let Some(rest) = input.strip_prefix(prefix) {
        // currently only support "!url" command
        let (md, is_clip, name) = handle_command(rest.trim_start()).await?;
        let result = ParseMessageResult { md, is_clip, name };
        to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    } else {
        let name = message::format_name(&timestamp);
        let result = ParseMessageResult {
            md: input.to_string(),
            is_clip: false,
            name,
        };
        to_value(&result).map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
