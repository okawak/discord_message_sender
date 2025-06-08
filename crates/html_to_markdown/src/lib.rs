pub mod dom;
pub mod error;
mod frontmatters;
mod parser;
mod renderers;
mod utils;

use error::ConvertError;
use frontmatters::get_frontmatter_extractors;

/// Convert HTML to Markdown with front-matter extraction
///
/// # Arguments
///
/// * `html` - The HTML content to convert.
/// * `keys` - The keys for front-matter extraction (e.g., "title", "tags", "date").
///
/// # Returns
///
/// * `Result<String, ConvertError>` - The converted Markdown content with front-matter, or an error.
///
/// # Example
///
/// ```rust
/// let html = "<h1>Title</h1><p>Content</p>";
/// let keys = ["title"];
/// let markdown = html_to_markdown::convert(html, &keys);
/// assert!(markdown.is_ok());
/// //assert!(markdown.unwrap().contains("---\ntitle: Title\n---\n\n# Title\n\nContent"));
/// ```
///
/// if you don't need front-matter, you can pass an empty slice for `keys`.
///
/// ```rust
/// let html = "<h1>Title</h1><p>Content</p>";
/// let keys: Vec<&str> = vec![];
/// let markdown = html_to_markdown::convert(html, &keys);
/// assert!(markdown.is_ok());
/// //assert!(markdown.unwrap().contains("# Title\n\nContent"));
/// ```
///
pub fn convert(html: &str, keys: &[&str]) -> Result<String, ConvertError> {
    // parse HTML
    let dom = parser::parse_html(html)?;

    // front-matter
    let extractors = get_frontmatter_extractors(keys);
    let frontmatter_entries: Vec<String> = extractors
        .into_iter()
        .filter_map(|(key, extractor)| {
            extractor.extract(&dom).map(|val| {
                match key {
                    "tags" => format!("tags: [{}]", val), // e.g. val = "tag1, tag2"
                    _ => format!("{}: {}", key, val),
                }
            })
        })
        .collect();

    let mut result = String::new();
    if !frontmatter_entries.is_empty() {
        result.push_str("---\n");
        for entry in frontmatter_entries {
            result.push_str(&entry);
            result.push('\n');
        }
        result.push_str("---\n\n");
    }

    // render body
    let mut ctx = renderers::Context;
    let body = renderers::render_node(&dom, dom.document, &mut ctx)?;
    result.push_str(&body);
    Ok(result)
}
