mod command;

use command::{MessageAction, parse_message as parse_message_action};
use html_to_markdown::convert;
use js_sys::Array;
use wasm_bindgen::prelude::*;

const FRONTMATTER_KEYS: &[&str] = &["title", "source"];

#[wasm_bindgen]
pub fn parse_message(input: &str, prefix: &str) -> Result<Array, JsValue> {
    let (kind, value) = match parse_message_action(input, prefix) {
        Ok(MessageAction::Message(markdown)) => ("message", markdown),
        Ok(MessageAction::Url(url)) => ("url", url),
        Err(error) => return Err(JsValue::from_str(&error.to_string())),
    };

    Ok(Array::of2(
        &JsValue::from_str(kind),
        &JsValue::from_str(&value),
    ))
}

#[wasm_bindgen]
pub fn convert_html(url: &str, html: &str) -> Result<String, JsValue> {
    convert(url, html, FRONTMATTER_KEYS).map_err(|error| JsValue::from_str(&error.to_string()))
}
