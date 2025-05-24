use thiserror::Error;
use wasm_bindgen::JsValue;

#[derive(Debug, Error)]
pub enum MessageError {
    #[error("Invalid URL")]
    InvalidUrl,
    #[error("Network request failed: {0}")]
    Network(String),
    #[error("Unknown command")]
    UnknownCommand,
    #[error("Unexpected error: {0}")]
    Other(String),
}

impl From<MessageError> for JsValue {
    fn from(e: MessageError) -> JsValue {
        JsValue::from_str(&e.to_string())
    }
}
