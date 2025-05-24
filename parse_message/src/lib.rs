use serde::Serialize;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
pub struct ParseMessageResult<'a> {
    pub md: &'a str,
    pub is_clip: bool,
    pub name: &'a str, // no need .md extension
}

#[wasm_bindgen]
pub fn process_message(input: &str, prefix: &str) -> JsValue {
    let dummy = ParseMessageResult {
        md: "> **Test Markdown**\n\n— *Dummy Bot*",
        is_clip: false,
        name: input,
    };
    to_value(&dummy).unwrap()
}

#[wasm_bindgen(start)]
pub fn init() {}
