pub mod command;

use command::handle_command;
use js_sys::Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn process_message(input: String, prefix: String) -> Result<Array, JsValue> {
    let input = input.trim();
    let prefix = prefix.trim();

    let (markdown, is_clipping) = if let Some(rest) = input.strip_prefix(prefix) {
        handle_command(rest.trim_start()).await?
    } else {
        (input.to_owned(), false)
    };

    Ok(Array::of2(
        &JsValue::from_str(&markdown),
        &JsValue::from_bool(is_clipping),
    ))
}
