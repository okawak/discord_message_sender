use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
    utils::format_list_content,
};
use std::collections::HashMap;

pub struct Media;

impl Media {
    /// Resolves a relative URL to an absolute URL using the base URL
    fn resolve_url(&self, base_url: &str, url: &str) -> Result<String, ConvertError> {
        if url.starts_with("https://")
            || url.starts_with("mailto:")
            || url.starts_with("tel:")
            || url.starts_with("ftp:")
        {
            return Ok(url.to_string());
        }

        let clean_base = base_url.split('#').next().unwrap_or(base_url);
        let clean_base = clean_base.split('?').next().unwrap_or(clean_base);

        // extension check
        let base_for_join = if self.has_file_extension(clean_base) {
            let trimmed = clean_base
                .rsplit_once('/')
                .map(|(base, _)| format!("{base}/"))
                .ok_or_else(|| {
                    ConvertError::InvalidUrl(format!("File extension seems invalid {clean_base}"))
                })?;
            trimmed.to_string()
        } else if clean_base.ends_with('/') {
            clean_base.to_string()
        } else {
            format!("{clean_base}/")
        };

        match self.url_join(&base_for_join, url) {
            Ok(resolved) => Ok(resolved),
            Err(_) => Ok(url.to_string()),
        }
    }

    fn has_file_extension(&self, url: &str) -> bool {
        // https:// included at lest two slashes
        if url.matches('/').count() <= 2 {
            return false;
        }

        url.split('/')
            .next_back()
            .map(|segment| segment.contains('.') && segment.split('.').count() > 1)
            .unwrap_or(false)
    }

    fn url_join(&self, base: &str, relative: &str) -> Result<String, ConvertError> {
        let (host, path) = self.parse_url(base)?;

        if relative.starts_with("/") {
            Ok(format!("https://{host}{relative}"))
        } else if relative.starts_with("./") {
            let rel_path = relative.strip_prefix("./").ok_or_else(|| {
                ConvertError::InvalidUrl(format!("Cannot strip ./ from {relative}"))
            })?;
            if path.ends_with('/') {
                Ok(format!("https://{host}{path}{rel_path}"))
            } else {
                Ok(format!("https://{host}{path}/{rel_path}"))
            }
        } else if relative.starts_with("../") {
            self.resolve_parent_path(&host, &path, relative)
        } else if path.ends_with('/') {
            Ok(format!("https://{host}{path}{relative}"))
        } else {
            Ok(format!("https://{host}{path}/{relative}"))
        }
    }

    fn parse_url(&self, url: &str) -> Result<(String, String), ConvertError> {
        if !url.starts_with("https://") {
            return Err(ConvertError::InvalidUrl(format!(
                "Not HTTPS protocol: {url}"
            )));
        }

        let rest_url = url.strip_prefix("https://").ok_or_else(|| {
            ConvertError::InvalidUrl(format!("Failed to strip https:// from {url}"))
        })?;
        let (host, path) = if let Some(slash_pos) = rest_url.find('/') {
            (&rest_url[..slash_pos], &rest_url[slash_pos..])
        } else {
            (rest_url, "/")
        };
        Ok((host.to_string(), path.to_string()))
    }

    fn resolve_parent_path(
        &self,
        host: &str,
        path: &str,
        relative: &str,
    ) -> Result<String, ConvertError> {
        // split by '/' and collect as Vec
        let mut path_parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut rel_parts: Vec<&str> = relative.split('/').collect();

        while let Some(&part) = rel_parts.first() {
            if part == ".." {
                if !path_parts.is_empty() {
                    path_parts.pop();
                }
                rel_parts.remove(0);
            } else if part == "." {
                rel_parts.remove(0);
            } else {
                break;
            }
        }

        path_parts.extend(rel_parts.iter().filter(|s| !s.is_empty()));

        let final_path = if path_parts.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", path_parts.join("/"))
        };

        Ok(format!("https://{host}{final_path}"))
    }

    /// Validates if the URL is safe to include in markdown
    fn is_safe_url(&self, url: &str) -> bool {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return false;
        }

        let lower_url = trimmed.to_lowercase();

        !lower_url.starts_with("http://") // accept only https
            && !lower_url.starts_with("#") // anchor links are safe but not treated as links in markdown
            && !lower_url.starts_with("javascript:")
            && !lower_url.starts_with("data:")
            && !lower_url.starts_with("vbscript:")
    }

    /// Extracts and cleans alt text for images
    fn get_alt_text(&self, attrs: &HashMap<String, String>) -> String {
        attrs
            .get("alt")
            .map(|alt| alt.trim().to_string())
            .unwrap_or_default()
    }

    fn has_multiple_elements(&self, dom: &Dom, link_id: NodeId) -> bool {
        let Ok(children) = dom.iter_children(link_id) else {
            return false;
        };

        let mut element_count = 0;
        for &child_id in children {
            if let Some(child_node) = dom.node(child_id)
                && let NodeData::Element { .. } = &child_node.data
            {
                element_count += 1;
                if element_count > 1 {
                    return true;
                }
            }
        }
        false
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
                if let Some(href) = attrs.get("href")
                    && self.is_safe_url(href)
                {
                    let resolved_url = self.resolve_url(url, href)?;

                    // in case of complex links, like bookmark, example:
                    // <a href="https://example.com/path?query#fragment">
                    //   <img src="/assets/image.png" alt="Image">
                    //   <span>Link Text</span>
                    //   <p>Additional Info</p>
                    // </a>
                    if self.has_multiple_elements(dom, id) {
                        return self.render_complex_link(url, dom, id, ctx, resolved_url);
                    }

                    ctx.in_inline = true;
                    let content = render_children(url, dom, id, ctx)?;
                    ctx.in_inline = old_inline_status;

                    Ok(format!("[{content}]({resolved_url})"))
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
                    if self.is_safe_url(&src) {
                        let resolved_src = self.resolve_url(url, &src)?;
                        format!("[![{alt}]({resolved_src})]({link_info})",)
                    } else {
                        format!("[{alt}]({link_info})")
                    }
                } else if self.is_safe_url(&src) {
                    let resolved_src = self.resolve_url(url, &src)?;
                    format!("![{alt}]({resolved_src})")
                } else {
                    alt
                };

                if ctx.in_inline && ctx.link_info.is_none() {
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
        "https://example.com/blog/image.jpg"
    )]
    #[case(
        "https://example.com/blog/",
        "thumbnail.png",
        "https://example.com/blog/thumbnail.png"
    )]
    #[case(
        "https://example.com/docs/api",
        "diagram.svg",
        "https://example.com/docs/api/diagram.svg"
    )]
    #[case(
        "https://example.com/category/subcategory",
        "images/icon.gif",
        "https://example.com/category/subcategory/images/icon.gif"
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
        "https://example.com/blog/image.jpg"
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
        "https://example.com/blog/assets/image.jpg"
    )]
    #[case(
        "https://example.com/docs/api/",
        "../images/logo.png",
        "https://example.com/docs/images/logo.png"
    )]
    #[case(
        "https://example.com/a/b/c",
        "../../shared/icon.svg",
        "https://example.com/a/shared/icon.svg"
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
    fn test_absolute_urls_passthrough(
        #[case] base_url: &str,
        #[case] absolute_url: &str,
        #[case] expected: &str,
    ) {
        let media = Media;
        assert_eq!(media.resolve_url(base_url, absolute_url).unwrap(), expected);
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
        "https://example.com/page/image.jpg"
    )]
    #[case(
        "https://example.com/blog?page=2&sort=date",
        "../images/header.jpg",
        "https://example.com/images/header.jpg"
    )]
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
            ![Performance Chart](https://example.com/reports/images/chart.svg)

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

            ![Rust Programming Language Logo](https://blog.example.com/posts/assets/rust-logo.png)

            For more information, visit the [official Rust website](https://www.rust-lang.org/).

            "#}
    )]
    #[case(
        r#"<a href="/dir1"><img alt="" src="https://example.com" /></a>"#,
        "https://example.com",
        "[![](https://example.com)](https://example.com/dir1)"
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
