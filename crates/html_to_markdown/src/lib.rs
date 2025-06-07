pub mod dom;
pub mod error;
mod frontmatters;
mod parser;
mod renderers;

use error::ConvertError;
use frontmatters::FRONTMATTERS;

/// Convert HTML to Markdown with front-matter extraction
/// # Arguments
/// * `html` - The HTML content to convert.
/// * `keys` - The keys for front-matter extraction (e.g., "title", "tags", "date").
/// # Returns
/// * `Result<String, ConvertError>` - The converted Markdown content with front-matter, or an error.
/// # Example
/// ```rust
/// let html = "<h1>Title</h1><p>Content</p>";
/// let keys = ["title"];
/// let markdown = html_to_markdown::convert(html, &keys);
/// assert!(markdown.is_ok());
/// assert!(markdown.unwrap().contains("---\ntitle: Title\n---\n\n# Title\n\nContent"));
/// ```
/// if you don't need front-matter, you can pass an empty slice for `keys`.
/// ```rust
/// let html = "<h1>Title</h1><p>Content</p>";
/// let keys: Vec<&str> = vec![];
/// let markdown = html_to_markdown::convert(html, &keys);
/// assert!(markdown.is_ok());
/// assert!(markdown.unwrap().contains("# Title\n\nContent"));
/// ```
pub fn convert(html: &str, keys: &[&str]) -> Result<String, ConvertError> {
    // parse HTML
    let dom = parser::parse_html(html)?;

    // front-matter
    let mut fm = String::from("---\n");
    for k in keys {
        if let Some(ext) = FRONTMATTERS.iter().find(|e| e.key() == *k) {
            if let Some(val) = ext.extract(&dom) {
                if *k == "tags" {
                    fm.push_str(&format!("tags: [{}]\n", val));
                } else {
                    fm.push_str(&format!("{k}: {val}\n"));
                }
            }
        }
    }
    fm.push_str("---\n\n");

    // render body
    let mut ctx = renderers::Context;
    let body = renderers::render_node(&dom, dom.document, &mut ctx)?;
    Ok(fm + &body)
}
