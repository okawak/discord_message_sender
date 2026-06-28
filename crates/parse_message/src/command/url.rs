use html_to_markdown::convert;
use wasm_bindgen::prelude::*;

const FRONTMATTER_KEYS: &[&str] = &["title", "source"];

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = ["window", "discordMsgSync"], js_name = fetchUrlContent)]
    async fn fetch_url_content(url: &str) -> Result<JsValue, JsValue>;
}

pub async fn handle(arg: Option<&str>) -> Result<(String, bool), JsValue> {
    let invalid_url = || JsValue::from_str("Invalid URL");
    let url_str = arg.ok_or_else(invalid_url)?;
    if !is_valid_url(url_str) {
        return Err(invalid_url());
    }

    // Using TypeScript's fetchUrlContent function to get the content of the URL
    let url_content = fetch_url_content(url_str)
        .await
        .map_err(|_| JsValue::from_str("Network request failed"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("URL response must be text"))?;

    let processed_md = convert(url_str, &url_content, FRONTMATTER_KEYS)
        .map_err(|_| JsValue::from_str("HTML conversion error"))?;
    Ok((processed_md, true))
}

fn is_valid_url(url: &str) -> bool {
    // accept HTTPS secure URLs only
    url.starts_with("https://")
}
