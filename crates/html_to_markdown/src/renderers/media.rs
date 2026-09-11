use super::{Context, Renderer, is_block_element, render_children, render_node};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
    utils::{filtering, format_list_content},
};
use std::{borrow::Cow, collections::HashMap};

#[cfg(not(target_arch = "wasm32"))]
use url::Url;

// Use the host's WHATWG implementation instead of embedding IDNA tables in the WASM bundle.
#[cfg(target_arch = "wasm32")]
mod browser_url {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[allow(clippy::upper_case_acronyms)]
        pub type URL;

        #[wasm_bindgen(constructor, catch)]
        pub fn new(reference: &str, base: &str) -> Result<URL, JsValue>;

        #[wasm_bindgen(method, getter)]
        pub fn href(this: &URL) -> String;
    }
}

pub struct Media;

impl Media {
    /// Resolves a relative URL to an absolute URL using the base URL
    fn resolve_url(&self, base_url: &str, url: &str) -> Option<String> {
        let url = url.trim();
        if !self.is_safe_url(url) {
            return None;
        }

        if !base_url
            .trim()
            .get(..8)
            .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
        {
            return Some(url.to_string());
        }

        let resolved = Self::resolve_standard_url(base_url.trim(), url)?;
        self.is_safe_url(&resolved).then_some(resolved)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn resolve_standard_url(base_url: &str, url: &str) -> Option<String> {
        Url::parse(base_url).ok()?.join(url).ok().map(Into::into)
    }

    #[cfg(target_arch = "wasm32")]
    fn resolve_standard_url(base_url: &str, url: &str) -> Option<String> {
        let resolved = browser_url::URL::new(url, base_url).ok()?;
        Some(resolved.href())
    }

    /// Validates if the URL is safe to include in markdown
    fn is_safe_url(&self, url: &str) -> bool {
        let trimmed = url.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return false;
        }

        let Some(scheme) = Self::url_scheme(trimmed) else {
            return true;
        };

        ["https", "mailto", "tel", "ftp"]
            .iter()
            .any(|allowed| scheme.eq_ignore_ascii_case(allowed))
    }

    fn url_scheme(url: &str) -> Option<Cow<'_, str>> {
        let (raw_scheme, _) = url.split_once(':')?;
        let scheme = if raw_scheme.contains(['\t', '\n', '\r']) {
            Cow::Owned(
                raw_scheme
                    .chars()
                    .filter(|character| !matches!(character, '\t' | '\n' | '\r'))
                    .collect(),
            )
        } else {
            Cow::Borrowed(raw_scheme)
        };
        (!scheme.is_empty()
            && scheme
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_alphabetic())
            && scheme
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.')))
        .then_some(scheme)
    }

    /// Extracts and cleans alt text for images
    fn get_alt_text(&self, attrs: &HashMap<String, String>) -> String {
        attrs
            .get("alt")
            .map(|alt| alt.trim().to_string())
            .unwrap_or_default()
    }

    fn has_block_content(&self, dom: &Dom, node_id: NodeId) -> bool {
        let Ok(children) = dom.iter_children(node_id) else {
            return false;
        };

        children.clone().any(|&child_id| {
            let Some(child_node) = dom.node(child_id) else {
                return false;
            };
            let NodeData::Element { tag, attrs } = &child_node.data else {
                return false;
            };
            let tag_name = tag.local.as_ref();

            !Self::is_ignored_element(tag_name, attrs)
                && (is_block_element(tag_name) || self.has_block_content(dom, child_id))
        })
    }

    fn is_card_link_element(tag_name: &str) -> bool {
        matches!(tag_name, "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "img")
    }

    fn is_ignored_element(tag_name: &str, attrs: &HashMap<String, String>) -> bool {
        matches!(tag_name, "script" | "style" | "noscript" | "footer" | "nav")
            || matches!(tag_name, "div" | "aside")
                && attrs
                    .get("class")
                    .is_some_and(|class| filtering::should_ignore_class(class))
    }

    fn has_heading_link_content(&self, dom: &Dom, node_id: NodeId) -> bool {
        let Ok(children) = dom.iter_children(node_id) else {
            return false;
        };

        children.clone().any(|&child_id| {
            let Some(child_node) = dom.node(child_id) else {
                return false;
            };

            match &child_node.data {
                NodeData::Text(text) => !text.trim().is_empty(),
                NodeData::Element { tag, attrs } => {
                    let tag_name = tag.local.as_ref();
                    if Self::is_ignored_element(tag_name, attrs) {
                        false
                    } else if matches!(tag_name, "code" | "pre") {
                        !dom.collect_text_content(child_id).trim().is_empty()
                    } else if tag_name == "img" {
                        !self.get_alt_text(attrs).is_empty()
                    } else {
                        self.has_heading_link_content(dom, child_id)
                    }
                }
                _ => false,
            }
        })
    }

    fn has_card_link_content(&self, url: &str, dom: &Dom, node_id: NodeId) -> bool {
        let Ok(children) = dom.iter_children(node_id) else {
            return false;
        };

        children.clone().any(|&child_id| {
            let Some(child_node) = dom.node(child_id) else {
                return false;
            };
            let NodeData::Element { tag, attrs } = &child_node.data else {
                return false;
            };

            let tag_name = tag.local.as_ref();
            if Self::is_ignored_element(tag_name, attrs) {
                return false;
            }
            if tag_name == "code"
                || Self::is_structured_block_element(tag_name)
                || attrs.contains_key("data-lang")
                || attrs
                    .get("class")
                    .is_some_and(|class| class.contains("code-frame"))
            {
                return false;
            }
            if tag_name == "img" {
                return !self.get_alt_text(attrs).is_empty()
                    || attrs
                        .get("src")
                        .and_then(|src| self.resolve_url(url, src))
                        .is_some();
            }
            if Self::is_card_link_element(tag_name) {
                return self.has_heading_link_content(dom, child_id);
            }

            self.has_card_link_content(url, dom, child_id)
        })
    }

    fn is_structured_block_element(tag_name: &str) -> bool {
        matches!(
            tag_name,
            "aside"
                | "blockquote"
                | "dl"
                | "fieldset"
                | "form"
                | "li"
                | "ol"
                | "pre"
                | "table"
                | "ul"
        )
    }

    fn has_structured_block_content(&self, dom: &Dom, node_id: NodeId) -> bool {
        let Ok(children) = dom.iter_children(node_id) else {
            return false;
        };

        children.clone().any(|&child_id| {
            let Some(child_node) = dom.node(child_id) else {
                return false;
            };
            let NodeData::Element { tag, attrs } = &child_node.data else {
                return false;
            };

            let tag_name = tag.local.as_ref();
            if Self::is_ignored_element(tag_name, attrs) {
                return false;
            }

            Self::is_structured_block_element(tag_name)
                || attrs.contains_key("data-lang")
                || attrs
                    .get("class")
                    .is_some_and(|class| class.contains("code-frame"))
                || self.has_structured_block_content(dom, child_id)
        })
    }

    fn append_standalone_destination(
        content: String,
        resolved_url: &str,
        list_depth: usize,
        has_following_content: bool,
    ) -> String {
        let content = content.trim_end();
        let indent = " ".repeat(list_depth);
        let trailing_separator = if list_depth == 0 {
            "\n\n".to_string()
        } else if has_following_content {
            format!("\n{indent}")
        } else {
            String::new()
        };
        if content.is_empty() {
            format!("{indent}[{resolved_url}]({resolved_url}){trailing_separator}")
        } else {
            format!("{content}\n\n{indent}[{resolved_url}]({resolved_url}){trailing_separator}")
        }
    }

    fn has_following_content(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        ctx: &Context,
    ) -> Result<bool, ConvertError> {
        let mut current_id = id;
        let mut probe_ctx = ctx.clone();
        probe_ctx.suppress_link_boundary_probe = true;

        while let Ok(Some(parent_id)) = dom.get_parent(current_id) {
            let Ok(children) = dom.iter_children(parent_id) else {
                return Ok(false);
            };
            for &child_id in children
                .skip_while(|&&child_id| child_id != current_id)
                .skip(1)
            {
                if !render_node(url, dom, child_id, &mut probe_ctx)?
                    .trim()
                    .is_empty()
                {
                    return Ok(true);
                }
            }

            if dom.node(parent_id).is_some_and(|node| {
                matches!(&node.data, NodeData::Element { tag, .. } if tag.local.as_ref() == "li")
            }) {
                break;
            }
            current_id = parent_id;
        }

        Ok(false)
    }

    fn normalize_link_label(content: &str) -> String {
        let mut result = String::with_capacity(content.len());
        let mut chars = content.chars().peekable();
        let mut code_delimiter = None;
        let mut pending_space = false;

        while let Some(character) = chars.next() {
            if character == '`' {
                if pending_space && !result.is_empty() {
                    result.push(' ');
                }
                pending_space = false;

                let mut delimiter_length = 1;
                while chars.next_if_eq(&'`').is_some() {
                    delimiter_length += 1;
                }
                result.extend(std::iter::repeat_n('`', delimiter_length));
                code_delimiter = match code_delimiter {
                    None => Some(delimiter_length),
                    Some(opening_length) if opening_length == delimiter_length => None,
                    current => current,
                };
                continue;
            }

            if code_delimiter.is_none() && character.is_whitespace() {
                pending_space = true;
                continue;
            }

            if pending_space && !result.is_empty() {
                result.push(' ');
            }
            pending_space = false;
            result.push(character);
        }

        result
    }

    fn render_complex_link(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        ctx: &mut Context,
        resolved_url: String,
    ) -> Result<String, ConvertError> {
        let old_link_info = ctx.link_info.take();
        let old_inline_status = ctx.in_inline;

        ctx.link_info = Some(resolved_url);
        ctx.in_inline = true;

        let result = render_children(url, dom, id, ctx)?;

        ctx.link_info = old_link_info;
        ctx.in_inline = old_inline_status;

        Ok(result)
    }
}

impl Renderer for Media {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        let Some(node) = dom.node(id) else {
            return false;
        };

        if let NodeData::Element { tag, .. } = &node.data {
            matches!(tag.local.as_ref(), "a" | "img")
        } else {
            false
        }
    }

    fn render(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        ctx: &mut Context,
    ) -> Result<String, ConvertError> {
        let (tag, attrs) = dom.get_element_data(id)?;
        let old_inline_status = ctx.in_inline;

        match tag.local.as_ref() {
            "a" => {
                if let Some(resolved_url) = attrs
                    .get("href")
                    .and_then(|href| self.resolve_url(url, href))
                {
                    // in case of complex links, like bookmark, example:
                    // <a href="https://example.com/path?query#fragment">
                    //   <img src="/assets/image.png" alt="Image">
                    //   <span>Link Text</span>
                    //   <p>Additional Info</p>
                    // </a>
                    let has_block_content = self.has_block_content(dom, id);
                    if has_block_content && self.has_card_link_content(url, dom, id) {
                        return self.render_complex_link(url, dom, id, ctx, resolved_url);
                    }

                    if self.has_structured_block_content(dom, id) {
                        let content = render_children(url, dom, id, ctx)?;
                        let has_following_content = !ctx.suppress_link_boundary_probe
                            && self.has_following_content(url, dom, id, ctx)?;
                        return Ok(Self::append_standalone_destination(
                            content,
                            &resolved_url,
                            ctx.list_depth,
                            has_following_content,
                        ));
                    }

                    ctx.in_inline = true;
                    let old_in_link_label = ctx.in_link_label;
                    ctx.in_link_label = true;
                    let rendered = render_children(url, dom, id, ctx);
                    ctx.in_inline = old_inline_status;
                    ctx.in_link_label = old_in_link_label;
                    let content = rendered?;

                    let content = if has_block_content {
                        Self::normalize_link_label(&content)
                    } else {
                        content
                    };

                    let trailing_separator = if !ctx.suppress_link_boundary_probe
                        && has_block_content
                        && self.has_following_content(url, dom, id, ctx)?
                    {
                        if ctx.list_depth == 0 {
                            "\n\n".to_string()
                        } else {
                            format!("\n{}", " ".repeat(ctx.list_depth))
                        }
                    } else {
                        String::new()
                    };

                    Ok(format!("[{content}]({resolved_url}){trailing_separator}"))
                } else {
                    ctx.in_inline = true;
                    let content = render_children(url, dom, id, ctx)?;
                    ctx.in_inline = old_inline_status;
                    Ok(content)
                }
            }
            "img" => {
                if ctx.in_heading {
                    return Ok(self.get_alt_text(attrs));
                }

                let alt = self.get_alt_text(attrs);
                let src = attrs.get("src").unwrap_or(&String::new()).clone();

                // check link context
                let result = if let Some(link_info) = &ctx.link_info {
                    if let Some(resolved_src) = self.resolve_url(url, &src) {
                        format!("[![{alt}]({resolved_src})]({link_info})",)
                    } else {
                        format!("[{alt}]({link_info})")
                    }
                } else if let Some(resolved_src) = self.resolve_url(url, &src) {
                    format!("![{alt}]({resolved_src})")
                } else {
                    alt
                };

                if (ctx.in_inline || ctx.in_paragraph) && ctx.link_info.is_none() {
                    Ok(result)
                } else {
                    Ok(format_list_content(ctx, &result))
                }
            }
            _ => render_children(url, dom, id, ctx),
        }
    }
}

pub static MEDIA: Media = Media;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::renderers;
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// resolve url test from root directory
    #[rstest]
    #[case(
        "https://example.com",
        "/assets/logo.png",
        "https://example.com/assets/logo.png"
    )]
    #[case(
        "https://example.com/",
        "/css/style.css",
        "https://example.com/css/style.css"
    )]
    #[case(
        "https://blog.example.com/subpage",
        "/images/header.jpg",
        "https://blog.example.com/images/header.jpg"
    )]
    #[case(
        "https://blog.example.com/tags/",
        "/category/index.html",
        "https://blog.example.com/category/index.html"
    )]
    #[case(
        "https://blog.example.com/deep/nested/path",
        "/from/root/index.html",
        "https://blog.example.com/from/root/index.html"
    )]
    #[case(
        "https://blog.example.com/file/index.html",
        "/root/index.html",
        "https://blog.example.com/root/index.html"
    )]
    fn test_root_relative_urls(
        #[case] base_url: &str,
        #[case] relative_url: &str,
        #[case] expected: &str,
    ) {
        let media = Media;
        assert_eq!(media.resolve_url(base_url, relative_url).unwrap(), expected);
    }

    /// relative URL path test
    #[rstest]
    #[case(
        "https://example.com/blog",
        "image.jpg",
        "https://example.com/image.jpg"
    )]
    #[case(
        "https://example.com/blog/",
        "thumbnail.png",
        "https://example.com/blog/thumbnail.png"
    )]
    #[case(
        "https://example.com/docs/api",
        "diagram.svg",
        "https://example.com/docs/diagram.svg"
    )]
    #[case(
        "https://example.com/category/subcategory",
        "images/icon.gif",
        "https://example.com/category/images/icon.gif"
    )]
    #[case(
        "https://example.com/category/subcategory/index.html",
        "images/icon.gif",
        "https://example.com/category/subcategory/images/icon.gif"
    )]
    fn test_relative_urls_from_subpages(
        #[case] base_url: &str,
        #[case] relative_url: &str,
        #[case] expected: &str,
    ) {
        let media = Media;
        assert_eq!(media.resolve_url(base_url, relative_url).unwrap(), expected);
    }

    /// relative URL from current directory test
    #[rstest]
    #[case(
        "https://example.com/blog",
        "./image.jpg",
        "https://example.com/image.jpg"
    )]
    #[case(
        "https://example.com/docs/",
        "./assets/diagram.png",
        "https://example.com/docs/assets/diagram.png"
    )]
    #[case(
        "https://example.com/docs/index.html",
        "./assets/diagram.png",
        "https://example.com/docs/assets/diagram.png"
    )]
    fn test_current_directory_relative_urls(
        #[case] base_url: &str,
        #[case] relative_url: &str,
        #[case] expected: &str,
    ) {
        let media = Media;
        assert_eq!(media.resolve_url(base_url, relative_url).unwrap(), expected);
    }

    /// upstream relative URL test
    #[rstest]
    #[case(
        "https://example.com/blog/post",
        "../assets/image.jpg",
        "https://example.com/assets/image.jpg"
    )]
    #[case(
        "https://example.com/docs/api/",
        "../images/logo.png",
        "https://example.com/docs/images/logo.png"
    )]
    #[case(
        "https://example.com/a/b/c",
        "../../shared/icon.svg",
        "https://example.com/shared/icon.svg"
    )]
    #[case(
        "https://example.com/deep/nested/path",
        "../../../root.jpg",
        "https://example.com/root.jpg"
    )]
    #[case(
        "https://example.com/a/b/index.html",
        "../c/root.jpg",
        "https://example.com/a/c/root.jpg"
    )]
    fn test_parent_directory_relative_urls(
        #[case] base_url: &str,
        #[case] relative_url: &str,
        #[case] expected: &str,
    ) {
        let media = Media;
        assert_eq!(media.resolve_url(base_url, relative_url).unwrap(), expected);
    }

    /// absolute URL passthrough test
    #[rstest]
    #[case(
        "https://example.com",
        "https://cdn.example.com/image.jpg",
        "https://cdn.example.com/image.jpg"
    )]
    #[case(
        "https://example.com",
        "https://external-site.com/api/data",
        "https://external-site.com/api/data"
    )]
    #[case(
        "https://example.com",
        "HTTPS://cdn.example.com/image.jpg",
        "https://cdn.example.com/image.jpg"
    )]
    fn test_absolute_urls(
        #[case] base_url: &str,
        #[case] absolute_url: &str,
        #[case] expected: &str,
    ) {
        let media = Media;
        assert_eq!(media.resolve_url(base_url, absolute_url).unwrap(), expected);
    }

    #[rstest]
    #[case(
        "https://example.com/articles/page",
        "//cdn.example.com/image.jpg",
        "https://cdn.example.com/image.jpg"
    )]
    #[case(
        "HTTPS://example.com/articles/page",
        "//cdn.example.com/image.jpg",
        "https://cdn.example.com/image.jpg"
    )]
    fn test_protocol_relative_urls(
        #[case] base_url: &str,
        #[case] relative_url: &str,
        #[case] expected: &str,
    ) {
        let media = Media;
        assert_eq!(media.resolve_url(base_url, relative_url).unwrap(), expected);
    }

    /// special schemes test
    #[rstest]
    #[case(
        "https://example.com",
        "mailto:contact@example.com",
        "mailto:contact@example.com"
    )]
    #[case("https://example.com", "tel:+1234567890", "tel:+1234567890")]
    #[case(
        "https://example.com",
        "ftp://files.example.com/doc.pdf",
        "ftp://files.example.com/doc.pdf"
    )]
    fn test_special_schemes(#[case] base_url: &str, #[case] url: &str, #[case] expected: &str) {
        let media = Media;
        assert_eq!(media.resolve_url(base_url, url).unwrap(), expected);
    }

    /// query parameters and fragments test
    #[rstest]
    #[case(
        "https://example.com/search?q=test",
        "/assets/logo.png",
        "https://example.com/assets/logo.png"
    )]
    #[case(
        "https://example.com/page#section",
        "image.jpg",
        "https://example.com/image.jpg"
    )]
    #[case(
        "https://example.com/blog?page=2&sort=date",
        "../images/header.jpg",
        "https://example.com/images/header.jpg"
    )]
    #[case(
        "https://example.com/docs/page?old=1#old",
        "?new=2#results",
        "https://example.com/docs/page?new=2#results"
    )]
    #[case("https://example.com?old=1", "?new=2", "https://example.com/?new=2")]
    fn test_base_url_with_query_and_fragment(
        #[case] base_url: &str,
        #[case] relative_url: &str,
        #[case] expected: &str,
    ) {
        let media = Media;
        assert_eq!(media.resolve_url(base_url, relative_url).unwrap(), expected);
    }

    /// file-based relative URLs test
    #[rstest]
    #[case(
        "https://example.com/a/index.html",
        "../b/image.png",
        "https://example.com/b/image.png"
    )]
    #[case(
        "https://example.com/docs/guide.html",
        "../assets/logo.svg",
        "https://example.com/assets/logo.svg"
    )]
    #[case(
        "https://example.com/blog/posts/article.html",
        "../../images/header.jpg",
        "https://example.com/images/header.jpg"
    )]
    fn test_file_based_relative_urls(
        #[case] base_url: &str,
        #[case] relative_url: &str,
        #[case] expected: &str,
    ) {
        let media = Media;
        assert_eq!(media.resolve_url(base_url, relative_url).unwrap(), expected);
    }

    /// anchor links test
    #[rstest]
    #[case(
        r##"<a href="#introduction">Introduction</a>"##,
        "https://example.com/guide",
        "Introduction"
    )]
    #[case(
        r##"<a href="#section-1">Go to Section 1</a>"##,
        "https://docs.example.com/api",
        "Go to Section 1"
    )]
    #[case(
        r##"<a href="#top">Back to Top</a>"##,
        "https://blog.example.com/post/123",
        "Back to Top"
    )]
    fn test_internal_anchor_links(
        #[case] html: &str,
        #[case] base_url: &str,
        #[case] expected: &str,
    ) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(base_url, &dom, dom.document, &mut context)
            .expect("Failed to render internal anchor link");
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case("https://example.com/image.jpg", true)]
    #[case("http://example.com/legacy.gif", false)] // http is not safe
    #[case("/assets/local-image.png", true)]
    #[case("image.jpg", true)]
    #[case("./media/video.mp4", true)]
    #[case("../shared/icon.svg", true)]
    #[case("mailto:test@example.com", true)]
    #[case("tel:+1234567890", true)]
    #[case("#anchor-link", false)]
    #[case("#", false)]
    #[case("", false)]
    #[case("   ", false)]
    #[case("\t\n", false)]
    #[case("javascript:alert('xss')", false)]
    #[case("vbscript:msgbox('xss')", false)]
    #[case("data:image/png;base64,iVBORw0K", false)]
    #[case("JAVASCRIPT:alert(1)", false)]
    #[case("java\nscript:alert(1)", false)]
    #[case("jav\tascript:alert(1)", false)]
    fn test_url_safety(#[case] url: &str, #[case] expected: bool) {
        let media = Media;
        assert_eq!(media.is_safe_url(url), expected);
    }

    /// exrernal links and special links test
    #[rstest]
    #[case(
        r#"<p>Visit <a href="https://github.com/rust-lang/rust">Rust on GitHub</a> for more info.</p>"#,
        "https://example.com",
        "Visit [Rust on GitHub](https://github.com/rust-lang/rust) for more info.\n\n"
    )]
    #[case(
        r#"<p>Email us at <a href="mailto:support@example.com">support@example.com</a></p>"#,
        "https://example.com",
        "Email us at [support@example.com](mailto:support@example.com)\n\n"
    )]
    fn test_external_and_special_links(
        #[case] html: &str,
        #[case] base_url: &str,
        #[case] expected: &str,
    ) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(base_url, &dom, dom.document, &mut context)
            .expect("Failed to render external links");
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case(
        r#"<a href="https://target.example.com"><span>Hello</span><span>World</span></a>"#,
        "[HelloWorld](https://target.example.com/)"
    )]
    #[case(
        r#"<a href="/formatted"><strong>Bold</strong> and <em>italic</em></a>"#,
        "[**Bold** and *italic*](https://example.com/formatted)"
    )]
    #[case(
        r#"<a href="/product"><img src="/product.png" alt="Product"><span>Details</span></a>"#,
        "[![Product](https://example.com/product.png)Details](https://example.com/product)"
    )]
    #[case(
        r#"<a href="/target"><div class="sidebar">ignored</div>Text</a>After"#,
        "[Text](https://example.com/target)After"
    )]
    fn test_inline_children_keep_link_destination(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(
            "https://example.com/articles/page",
            &dom,
            dom.document,
            &mut context,
        )
        .expect("Failed to render inline link children");
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case(
        r#"<a href="/target"><div>Details</div></a>"#,
        "[Details](https://example.com/target)"
    )]
    #[case(
        r#"<a href="/target"><p>First</p><p>Second</p></a>"#,
        "[First Second](https://example.com/target)"
    )]
    #[case(
        r#"<a href="/target"><p><code>a  b</code></p></a>"#,
        "[`a  b`](https://example.com/target)"
    )]
    #[case(
        r#"<a href="/target">Before<p>After</p></a>"#,
        "[Before After](https://example.com/target)"
    )]
    #[case(
        r#"<a href="/target"><p>Before</p>After</a>"#,
        "[Before After](https://example.com/target)"
    )]
    #[case(
        r#"<a href="/target">Before<figure>After</figure></a>"#,
        "[Before After](https://example.com/target)"
    )]
    #[case(
        r#"<a href="/target"><figure>Before</figure>After</a>"#,
        "[Before After](https://example.com/target)"
    )]
    #[case(
        r#"<a href="/target">Before<address>Middle</address>After</a>"#,
        "[Before Middle After](https://example.com/target)"
    )]
    #[case(
        r#"<a href="/target">Before<details>Middle</details>After</a>"#,
        "[Before Middle After](https://example.com/target)"
    )]
    #[case(
        r#"<a href="/target">Before<hr>After</a>"#,
        "[Before After](https://example.com/target)"
    )]
    fn test_text_block_children_keep_link_destination(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(
            "https://example.com/articles/page",
            &dom,
            dom.document,
            &mut context,
        )
        .expect("Failed to render text block link children");
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case(
        "<a href=\"/target\"><pre><code>line1\n  line2</code></pre></a>",
        "```\nline1\n  line2\n```\n\n[https://example.com/target](https://example.com/target)\n\n"
    )]
    #[case(
        "<a href=\"/target\"><div class=\"code-frame\"><code>line1\n  line2</code></div></a>",
        "```\nline1\n  line2\n```\n\n[https://example.com/target](https://example.com/target)\n\n"
    )]
    #[case(
        "<a href=\"/target\"><div data-lang=\"rust\"><code>fn main() {}</code></div></a>",
        "```rust\nfn main() {}\n```\n\n[https://example.com/target](https://example.com/target)\n\n"
    )]
    #[case(
        "<a href=\"/target\"><ul><li><p>First</p><p>Second</p></li></ul></a>",
        "- First\n\n  Second\n\n[https://example.com/target](https://example.com/target)\n\n"
    )]
    fn test_structured_block_link_preserves_code_and_destination(
        #[case] html: &str,
        #[case] expected: &str,
    ) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(
            "https://example.com/articles/page",
            &dom,
            dom.document,
            &mut context,
        )
        .expect("Failed to render structured block link");
        assert_eq!(result, expected);
    }

    #[rstest]
    #[case(
        "<a href=\"/target\"><pre><code>x</code></pre></a>After",
        "```\nx\n```\n\n[https://example.com/target](https://example.com/target)\n\nAfter"
    )]
    #[case(
        "<a href=\"/target\"><p>Before</p></a>After",
        "[Before](https://example.com/target)\n\nAfter"
    )]
    #[case(
        "<a href=\"/target\"><h2></h2><p>Details</p></a>",
        "[Details](https://example.com/target)"
    )]
    #[case(
        "<a href=\"/target\"><img><p>Details</p></a>",
        "[Details](https://example.com/target)"
    )]
    #[case(
        "<a href=\"/target\"><img src=\"javascript:alert(1)\"><p>Details</p></a>",
        "[Details](https://example.com/target)"
    )]
    #[case(
        "<a href=\"/target\"><h2><img src=\"/icon.png\"></h2><p>Details</p></a>",
        "[Details](https://example.com/target)"
    )]
    #[case(
        "<ul><li><a href=\"/target\"><pre><code>x</code></pre></a></li><li>Next</li></ul>",
        "- \n\n  ```\n  x\n  ```\n\n  [https://example.com/target](https://example.com/target)\n- Next\n\n"
    )]
    #[case(
        "<ul><li><a href=\"/target\"><pre><code>x</code></pre></a><h2>After</h2></li></ul>",
        "- \n\n  ```\n  x\n  ```\n\n  [https://example.com/target](https://example.com/target)\n  ## After\n\n"
    )]
    #[case(
        "<ul><li><div><a href=\"/target\"><pre><code>x</code></pre></a></div><h2>After</h2></li></ul>",
        "- \n\n  ```\n  x\n  ```\n\n  [https://example.com/target](https://example.com/target)\n  ## After\n\n"
    )]
    #[case(
        "<ul><li><a href=\"/target\"><pre><code>x</code></pre></a><div class=\"sidebar\">ignored</div></li><li>Next</li></ul>",
        "- \n\n  ```\n  x\n  ```\n\n  [https://example.com/target](https://example.com/target)\n- Next\n\n"
    )]
    #[case(
        "<ul><li><div><a href=\"/target\"><pre><code>x</code></pre></a></div><section></section></li><li>Next</li></ul>",
        "- \n\n  ```\n  x\n  ```\n\n  [https://example.com/target](https://example.com/target)\n- Next\n\n"
    )]
    #[case(
        "<a href=\"/target\"><div><div class=\"sidebar\"><h2>Ignored</h2></div><span>Details</span></div></a>",
        "[Details](https://example.com/target)"
    )]
    #[case(
        "<a href=\"/target\"><div class=\"sidebar\"><pre>ignored</pre></div><span>Details</span></a>",
        "[Details](https://example.com/target)"
    )]
    #[case(
        "<a href=\"/target\"><div><code><img src=\"/icon.png\" alt=\"Icon\"></code><span>Details</span></div></a>",
        "[`![Icon](https://example.com/icon.png)`Details](https://example.com/target)"
    )]
    fn test_complex_link_boundaries(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(
            "https://example.com/articles/page",
            &dom,
            dom.document,
            &mut context,
        )
        .expect("Failed to render complex link boundaries");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_many_sibling_block_links_without_recursive_boundary_probes() {
        const LINK_COUNT: usize = 800;
        let html = (0..LINK_COUNT)
            .map(|index| format!(r#"<a href="/{index}"><p>{index}</p></a>"#))
            .collect::<String>();
        let expected = (0..LINK_COUNT)
            .map(|index| format!("[{index}](https://example.com/{index})"))
            .collect::<Vec<_>>()
            .join("\n\n");
        let dom = parser::parse_html(&html).expect("Failed to parse HTML");
        let mut context = Context::default();

        let result = renderers::render_node(
            "https://example.com/articles/page",
            &dom,
            dom.document,
            &mut context,
        )
        .expect("Failed to render sibling block links");

        assert_eq!(result, expected);
    }

    /// image rendering tests
    #[rstest]
    #[case(
        r#"<article><p>Here's a diagram:</p><img src="/assets/architecture-diagram.png" alt="System Architecture"></article>"#,
        "https://docs.example.com/guide/introduction",
        indoc! {r#"
            Here's a diagram:

            ![System Architecture](https://docs.example.com/assets/architecture-diagram.png)

            "#}
    )]
    #[case(
        r#"<figure><img src="../images/chart.svg" alt="Performance Chart"><figcaption>Q4 Performance</figcaption></figure>"#,
        "https://example.com/reports/2024",
        indoc! {r#"
            ![Performance Chart](https://example.com/images/chart.svg)

            Q4 Performance"#}
    )]
    fn test_article_images(#[case] html: &str, #[case] base_url: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(base_url, &dom, dom.document, &mut context)
            .expect("Failed to render article images");
        assert_eq!(result, expected);
    }

    /// CDN images test
    #[rstest]
    #[case(
        r#"<img src="https://cdn.example.com/uploads/2024/header-image.jpg" alt="Header Image">"#,
        "https://blog.example.com/post/123",
        indoc! {r#"
            ![Header Image](https://cdn.example.com/uploads/2024/header-image.jpg)

            "#}
    )]
    #[case(
        r#"<img src="https://images.unsplash.com/photo-1234567890" alt="Stock Photo">"#,
        "https://example.com",
        "![Stock Photo](https://images.unsplash.com/photo-1234567890)\n\n"
    )]
    fn test_cdn_images(#[case] html: &str, #[case] base_url: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(base_url, &dom, dom.document, &mut context)
            .expect("Failed to render CDN images");
        assert_eq!(result, expected);
    }

    /// complex nested media test
    #[rstest]
    #[case(
        r#"<div class="card">
            <a href="/products/laptop">
                <img src="/images/products/laptop-thumb.jpg" alt="Gaming Laptop">
                <h3>Gaming Laptop</h3>
                <p>High-performance laptop for gaming</p>
            </a>
        </div>"#,
        "https://shop.example.com/category/computers",
        indoc! {r#"
            [![Gaming Laptop](https://shop.example.com/images/products/laptop-thumb.jpg)](https://shop.example.com/products/laptop)

            ### [Gaming Laptop](https://shop.example.com/products/laptop)

            High-performance laptop for gaming

            "#}
    )]
    #[case(
        r#"<div class="card-no-image">
            <a href="/products/laptop">
                <h3>Gaming Laptop</h3>
                <p>High-performance laptop for gaming</p>
            </a>
        </div>"#,
        "https://shop.example.com/category/computers",
        indoc! {r#"
            ### [Gaming Laptop](https://shop.example.com/products/laptop)

            High-performance laptop for gaming

            "#}
    )]
    fn test_complex_nested_media(
        #[case] html: &str,
        #[case] base_url: &str,
        #[case] expected: &str,
    ) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(base_url, &dom, dom.document, &mut context)
            .expect("Failed to render complex nested media");
        assert_eq!(result, expected);
    }

    /// images in headings test
    #[rstest]
    #[case(
        r#"<h1><img src="/assets/logo.png" alt="Company Logo"> Welcome to Our Site</h1>"#,
        "https://example.com",
        "# Company Logo Welcome to Our Site\n\n"
    )]
    #[case(
        r#"<h2>Section <img src="icon.svg" alt="📊"> Analytics</h2>"#,
        "https://dashboard.example.com",
        "## Section 📊 Analytics\n\n"
    )]
    fn test_images_in_headings(#[case] html: &str, #[case] base_url: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(base_url, &dom, dom.document, &mut context)
            .expect("Failed to render images in headings");
        assert_eq!(result, expected);
    }

    /// security cases: unsafe links and images
    #[rstest]
    #[case(
        r#"<a href="javascript:alert('XSS')">Malicious Link</a>"#,
        "https://example.com",
        "Malicious Link"
    )]
    #[case(
        r#"<img src="javascript:alert('XSS')" alt="Malicious Image">"#,
        "https://example.com",
        "Malicious Image\n\n"
    )]
    #[case(
        r#"<img src="data:image/svg+xml;base64,PHN2Zz4KPC9zdmc+" alt="Data URI Image">"#,
        "https://example.com",
        "Data URI Image\n\n"
    )]
    #[case(
        r#"<a href="java&#10;script:alert(1)">Encoded Malicious Link</a>"#,
        "https://example.com",
        "Encoded Malicious Link"
    )]
    #[case(
        r#"<img src="java&#10;script:alert(1)" alt="Encoded Malicious Image">"#,
        "https://example.com",
        "Encoded Malicious Image\n\n"
    )]
    fn test_security_cases(#[case] html: &str, #[case] base_url: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(base_url, &dom, dom.document, &mut context)
            .expect("Failed to render security test case");
        assert_eq!(result, expected);
    }

    /// edge cases: empty href, src, and no attributes
    #[rstest]
    #[case(
        r#"<a>Link without href</a>"#,
        "https://example.com",
        "Link without href"
    )]
    #[case(r#"<a href="">Empty href</a>"#, "https://example.com", "Empty href")]
    #[case(r#"<img alt="without src">"#, "https://example.com", "without src\n\n")]
    #[case(
        r#"<img src="" alt="Empty src">"#,
        "https://example.com",
        "Empty src\n\n"
    )]
    fn test_edge_cases(#[case] html: &str, #[case] base_url: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(base_url, &dom, dom.document, &mut context)
            .expect("Failed to render edge case");
        assert_eq!(result, expected);
    }

    /// realistic blog structure test
    #[rstest]
    #[case(
        r#"<article>
            <header>
                <h1>How to Use Rust for Web Development</h1>
                <p>Published on <a href="/blog/2024">2024</a></p>
            </header>
            <main>
                <p>Rust is becoming popular for web development. Here's why:</p>
                <img src="../assets/rust-logo.png" alt="Rust Programming Language Logo">
                <p>For more information, visit the <a href="https://www.rust-lang.org/">official Rust website</a>.</p>
            </main>
        </article>"#,
        "https://blog.example.com/posts/rust-web-dev",
        indoc! {r#"
            # How to Use Rust for Web Development

            Published on [2024](https://blog.example.com/blog/2024)

            Rust is becoming popular for web development. Here's why:

            ![Rust Programming Language Logo](https://blog.example.com/assets/rust-logo.png)

            For more information, visit the [official Rust website](https://www.rust-lang.org/).

            "#}
    )]
    #[case(
        r#"<a href="/dir1"><img alt="" src="https://example.com" /></a>"#,
        "https://example.com",
        "[![](https://example.com/)](https://example.com/dir1)"
    )]
    fn test_realistic_blog_structure(
        #[case] html: &str,
        #[case] base_url: &str,
        #[case] expected: &str,
    ) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(base_url, &dom, dom.document, &mut context)
            .expect("Failed to render blog structure");
        assert_eq!(result, expected);
    }

    /// Lists with images and links
    #[rstest]
    #[case(
        "<ul><li><img src=\"/icon.png\" alt=\"Icon\"><p>Item with image</p></li></ul>",
        indoc! {r#"
            - ![Icon](https://example.com/icon.png)

              Item with image

            "#}
    )]
    #[case(
        "<ul><li><a href=\"/page\"><img src=\"thumb.jpg\" alt=\"Thumbnail\"><p>Link with image</p></a></li></ul>",
        indoc! {r#"
            - [![Thumbnail](https://example.com/thumb.jpg)](https://example.com/page)

              Link with image

            "#}
    )]
    fn test_lists_with_media(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result =
            renderers::render_node("https://example.com", &dom, dom.document, &mut context)
                .expect("Failed to render list with media");
        assert_eq!(result, expected);
    }
}
