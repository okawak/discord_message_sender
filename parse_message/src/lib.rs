use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn to_md(author: &str, content: &str, iso_ts: &str) -> String {
    format!("- **{}** ({})\n{}\n\n", author, iso_ts, content)
}
