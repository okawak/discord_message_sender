pub mod url;

use wasm_bindgen::JsValue;

pub async fn handle_command(rest: &str) -> Result<(String, bool), JsValue> {
    let (cmd, arg) = rest
        .split_once(char::is_whitespace)
        .map_or((rest, None), |(cmd, arg)| (cmd, Some(arg.trim_start())));

    match cmd {
        "url" => url::handle(arg).await,
        _ => Err(JsValue::from_str("Unknown command")),
    }
}
