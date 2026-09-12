mod bindings;
mod command;
pub mod core;
#[cfg(test)]
#[path = "../tests/compatibility/mod.rs"]
mod compatibility_tests;

use html_to_markdown::convert;
use wasm_bindgen::prelude::*;

const FRONTMATTER_KEYS: &[&str] = &["title", "source"];

#[wasm_bindgen]
pub fn convert_html(url: &str, html: &str) -> Result<String, JsValue> {
    convert(url, html, FRONTMATTER_KEYS).map_err(|error| JsValue::from_str(&error.to_string()))
}
