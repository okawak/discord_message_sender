use crate::{error::MessageError, message};
use html_to_markdown::convert;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "discordMsgSync"], js_name = fetchUrlContent)]
    async fn fetch_url_content(url: &str) -> JsValue;
}

pub async fn handle(arg: Option<&str>, timestamp: &str) -> Result<(String, bool, String), JsValue> {
    let url_str = arg.ok_or(MessageError::InvalidUrl)?;
    if !is_valid_url(url_str) {
        return Err(MessageError::InvalidUrl.into());
    }

    // Using TypeScript's fetchUrlContent function to get the content of the URL
    let url_content_js = fetch_url_content(url_str).await;
    let url_content = url_content_js.as_string().unwrap_or_default();

    // frontmatter keys, now for test
    // TODO: get from TypeScript Settings
    let keys = vec!["title"];

    let processed_md = convert(&url_content, &keys).map_err(|_| MessageError::ConversionError)?;
    // TODO: retrieve site name from front-matter
    let title = message::format_name(timestamp);

    Ok((processed_md, true, title))
}

fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}
