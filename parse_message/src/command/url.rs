use crate::error::MessageError;
use url::Url;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = globalThis, js_name = fetchUrlContent)]
    async fn fetch_url_content(url: &str) -> JsValue;
}

pub async fn handle(arg: Option<&str>) -> Result<(String, bool, String), JsValue> {
    let url_str = arg.ok_or(MessageError::InvalidUrl)?;
    let url = Url::parse(url_str).map_err(|_| MessageError::InvalidUrl)?;

    // TypeScript側のfetchUrlContent関数を呼び出し
    let url_content_js = fetch_url_content(url_str).await;
    let url_content = url_content_js.as_string().unwrap_or_default();

    // HTMLをマークダウンに変換
    let processed_md =
        process_html_to_markdown(&url_content, url_str, url.host_str().unwrap_or("unknown"));
    let site_name = url.host_str().unwrap_or("unknown").to_owned();

    Ok((processed_md, true, site_name))
}

fn process_html_to_markdown(html: &str, url: &str, site_name: &str) -> String {
    let title = extract_title(html).unwrap_or_else(|| site_name.to_string());
    let description = extract_meta_description(html);

    let mut markdown = format!("# {}\n\n", title);

    if let Some(desc) = description {
        markdown.push_str(&format!("> {}\n\n", desc));
    }

    markdown.push_str(&format!("[元のURL]({}) - {}\n\n", url, site_name));

    let content = extract_main_content(html);
    if !content.is_empty() {
        markdown.push_str("## 内容\n\n");
        markdown.push_str(&content);
    }

    markdown
}

fn extract_title(html: &str) -> Option<String> {
    if let Some(start) = html.find("<title>") {
        if let Some(end) = html[start + 7..].find("</title>") {
            let title = &html[start + 7..start + 7 + end];
            return Some(title.trim().to_string());
        }
    }
    None
}

fn extract_meta_description(html: &str) -> Option<String> {
    if let Some(start) = html.find(r#"<meta name="description""#) {
        if let Some(content_start) = html[start..].find(r#"content=""#) {
            let content_pos = start + content_start + 9;
            if let Some(content_end) = html[content_pos..].find('"') {
                let description = &html[content_pos..content_pos + content_end];
                return Some(description.trim().to_string());
            }
        }
    }
    None
}

fn extract_main_content(html: &str) -> String {
    let mut content = String::new();
    let mut current_pos = 0;

    while let Some(p_start) = html[current_pos..].find("<p>") {
        let abs_start = current_pos + p_start + 3;
        if let Some(p_end) = html[abs_start..].find("</p>") {
            let paragraph = &html[abs_start..abs_start + p_end];
            content.push_str(&strip_html_tags(paragraph));
            content.push_str("\n\n");
            current_pos = abs_start + p_end + 4;
        } else {
            break;
        }
    }

    content.trim().to_string()
}

fn strip_html_tags(text: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    result
}
