use crate::error::MessageError;
use reqwest::Client;
use url::Url;
use wasm_bindgen::JsValue;

pub async fn handle(arg: Option<&str>) -> Result<(String, bool, String), JsValue> {
    let url_str = arg.ok_or(MessageError::InvalidUrl)?;
    let url = Url::parse(url_str).map_err(|_| MessageError::InvalidUrl)?;

    let client = Client::new();
    let resp = client
        .get(url.clone())
        .send()
        .await
        .map_err(|e| MessageError::Network(e.to_string()))?;
    let body = resp
        .text()
        .await
        .map_err(|e| MessageError::Network(e.to_string()))?;

    let site_name = url.host_str().unwrap_or("unknown").to_owned();
    Ok((body, true, site_name))
}
