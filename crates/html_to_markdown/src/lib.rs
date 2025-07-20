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
/// * 'url'  - The URL of the HTML content (used for context, e.g., links).
/// * `html` - The HTML content to convert.
/// * `keys` - The keys for front-matter extraction (e.g., "title", "tags", "date").
///
/// # Returns
///
/// * `Result<String, ConvertError>` - The converted Markdown content with front-matter (YAML format), or an error.
///
/// # Example
///
/// ```rust
/// let url = "https://example.com";
/// let html = "<h1>Title</h1><p>Content</p>";
/// let keys = ["title"];
/// let markdown = html_to_markdown::convert(url, html, &keys);
/// assert!(markdown.is_ok());
/// assert!(markdown.unwrap().contains("---\ntitle: Title\n---\n\n# Title\n\nContent"));
/// ```
///
/// if you don't need front-matter, you can pass an empty slice for `keys`.
///
/// ```rust
/// let url = "https://example.com";
/// let html = "<h1>Title</h1><p>Content</p>";
/// let keys: Vec<&str> = vec![];
/// let markdown = html_to_markdown::convert(url, html, &keys);
/// assert!(markdown.is_ok());
/// assert!(markdown.unwrap().contains("# Title\n\nContent"));
/// ```
///
pub fn convert(url: &str, html: &str, keys: &[&str]) -> Result<String, ConvertError> {
    // If you want to fetch HTML content from a URL,
    // you can use an HTTP client library like `reqwest` here.
    // (Obsidian need this API, `requestUrl`, so html content is passed directly)
    // example:
    // let client = reqwest::Client::new();
    // let html = client.request(reqwest::Method::GET, url)?
    //                  .send()
    //                  .await?
    //                  .error_for_status()?;

    // parse HTML
    let dom = parser::parse_html(html)?;

    // front-matter
    let extractors = get_frontmatter_extractors(keys);
    let frontmatter_entries: Vec<String> = extractors
        .into_iter()
        .filter_map(|(key, extractor)| {
            extractor
                .extract(url, &dom)
                .map(|val| format!("{key}: {val}"))
        })
        .collect();

    let mut markdown = String::new();
    if !frontmatter_entries.is_empty() {
        markdown.push_str("---\n");
        for entry in frontmatter_entries {
            markdown.push_str(&entry);
            markdown.push('\n');
        }
        markdown.push_str("---\n\n");
    }

    // render body
    let mut ctx = renderers::Context::default();
    let start_id = dom
        .find_article()
        .or_else(|| dom.find_body())
        .unwrap_or(dom.document);
    let body = renderers::render_node(url, &dom, start_id, &mut ctx)?;
    markdown.push_str(&body);
    Ok(markdown)
}
