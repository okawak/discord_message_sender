use thiserror::Error;

/// Error types for HTML to Markdown conversion.
#[derive(Error, Debug)]
pub enum ConvertError {
    #[error("HTML parse error: {0}")]
    Parse(String),

    #[error("Invalid node: {0}")]
    InvalidNode(String),

    #[error("Unsupported tag: <{0}>")]
    Unsupported(String),

    #[error("Unknown error")]
    Unknown,
}
