use crate::dom::{Dom, NodeData};
use crate::frontmatters::FrontMatter;
use crate::utils::normalize_html_text;

pub struct TitleExtractor;

impl FrontMatter for TitleExtractor {
    fn key(&self) -> &'static str {
        "title"
    }

    fn extract(&self, dom: &Dom) -> Option<String> {
        let extractors = [
            extract_head_title,      // 1. <head><title>
            extract_meta_name_title, // 2. <meta name="title">
            extract_ogp_title,       // 3. OGP title
            extract_twitter_title,   // 4. Twitter title
        ];

        for extractor in &extractors {
            if let Some(title) = extractor(dom) {
                return Some(title);
            }
        }

        // 5. first heading in <body>
        ["h1", "h2", "h3", "h4", "h5", "h6"]
            .iter()
            .find_map(|&tag| extract_first_heading(dom, tag))
    }
}

fn extract_head_title(dom: &Dom) -> Option<String> {
    let head_id = dom.find_head()?;
    let title_id = dom.find_element_by_tag(head_id, "title")?;

    let text = dom.collect_text_content(title_id);
    normalize_html_text(&text)
}

fn extract_meta_content(dom: &Dom, attr_name: &str, attr_value: &str) -> Option<String> {
    for meta_id in dom.find_all_meta() {
        let node = dom.node(meta_id);

        if let NodeData::Element { attrs, .. } = &node.data {
            if let (Some(value), Some(content)) = (attrs.get(attr_name), attrs.get("content")) {
                if value == attr_value {
                    return normalize_html_text(content);
                }
            }
        }
    }
    None
}

fn extract_meta_name_title(dom: &Dom) -> Option<String> {
    extract_meta_content(dom, "name", "title")
}

fn extract_ogp_title(dom: &Dom) -> Option<String> {
    extract_meta_content(dom, "property", "og:title")
}

fn extract_twitter_title(dom: &Dom) -> Option<String> {
    extract_meta_content(dom, "name", "twitter:title")
}

fn extract_first_heading(dom: &Dom, tag: &str) -> Option<String> {
    let body_id = dom.find_body()?;
    let heading_id = dom.find_element_by_tag(body_id, tag)?;

    let text = dom.collect_text_content(heading_id);
    normalize_html_text(&text)
}

pub static EXTRACTOR: TitleExtractor = TitleExtractor;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use pretty_assertions::assert_eq;
    use rstest::*;

    #[rstest]
    #[case(
        r#"<html><head><title>Page Title</title></head><body><h1>Body Title</h1></body></html>"#,
        Some("Page Title")
    )]
    #[case(
        r#"<html><head><meta name="title" content="Meta Name Title"/></head><body><h1>Body Title</h1></body></html>"#,
        Some("Meta Name Title")
    )]
    #[case(
        r#"<html><head><meta property="og:title" content="OGP Title"/></head><body><h1>Body Title</h1></body></html>"#,
        Some("OGP Title")
    )]
    #[case(
        r#"<html><head><meta name="twitter:title" content="Twitter Title"/></head><body><h1>Body Title</h1></body></html>"#,
        Some("Twitter Title")
    )]
    #[case(
        r#"<html><body><h1>Main Heading</h1><h2>Sub Heading</h2></body></html>"#,
        Some("Main Heading")
    )]
    #[case(
        r#"<html><body><p>No h1 here</p><h2>First H2</h2><h2>Second H2</h2></body></html>"#,
        Some("First H2")
    )]
    #[case(
        r#"<html><body><p>No headings or titles here</p><div>Just content</div></body></html>"#,
        None
    )]
    #[case(
        r#"<html><head><title>  Trimmed Title  </title></head></html>"#,
        Some("Trimmed Title")
    )]
    #[case(r#"<html><head><title>  </title></head></html>"#, None)] // Empty title
    fn test_title_extraction(#[case] html: &str, #[case] expected: Option<&str>) {
        let dom = parser::parse_html(html).unwrap();
        let result = EXTRACTOR.extract(&dom);

        assert_eq!(result, expected.map(|s| s.to_string()));
    }

    #[rstest]
    #[case(
        r#"<html>
            <head>
                <title>Head Title</title>
                <meta name="twitter:title" content="Twitter Title"/>
                <meta name="title" content="Meta Name Title"/>
                <meta property="og:title" content="OGP Title"/>
            </head>
            <body><h1>Body H1</h1></body>
        </html>"#,
        "Head Title"
    )]
    #[case(
        r#"<html>
            <head>
                <meta name="title" content="Meta Name Title"/>
                <meta property="og:title" content="OGP Title"/>
                <meta name="twitter:title" content="Twitter Title"/>
            </head>
            <body><h1>Body H1</h1></body>
        </html>"#,
        "Meta Name Title"
    )]
    #[case(
        r#"<html>
            <head>
                <meta property="og:title" content="OGP Title"/>
                <meta name="twitter:title" content="Twitter Title"/>
            </head>
            <body><h1>Body H1</h1></body>
        </html>"#,
        "OGP Title"
    )]
    #[case(
        r#"<html>
            <head><meta name="twitter:title" content="Twitter Title"/></head>
            <body><h1>Body H1</h1></body>
        </html>"#,
        "Twitter Title"
    )]
    #[case(
        r#"<html>
            <head><meta property="og:type" content="OGP Type"/></head>
            <body><h1>Body H1</h1></body>
        </html>"#,
        "Body H1"
    )]
    fn test_priority_order(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).unwrap();
        let result = EXTRACTOR.extract(&dom);

        assert_eq!(result, Some(expected.to_string()));
    }

    #[rstest]
    #[case("h1", "Heading 1")]
    #[case("h2", "Heading 2")]
    #[case("h3", "Heading 3")]
    #[case("h4", "Heading 4")]
    #[case("h5", "Heading 5")]
    #[case("h6", "Heading 6")]
    fn test_heading_fallback_order(#[case] tag: &str, #[case] expected: &str) {
        let html = format!(r#"<html><body><p>Content</p><{tag}>{expected}</{tag}></body></html>"#);
        let dom = parser::parse_html(&html).unwrap();
        let result = EXTRACTOR.extract(&dom);

        assert_eq!(result, Some(expected.to_string()));
    }

    #[rstest]
    #[case(
        r#"<title>   Multi
    line
        title   </title>"#,
        Some("Multi line title")
    )]
    #[case(r#"<title>Title	with	tabs</title>"#, Some("Title with tabs"))]
    #[case(
        r#"<title>Title&#10;with&#13;newlines</title>"#,
        Some("Title with newlines")
    )]
    #[case(
        r#"<title>&#160;&#8203;&#65279;</title>"#, // Empty title with invisible characters
        None
    )]
    fn test_whitespace_normalization(#[case] html: &str, #[case] expected: Option<&str>) {
        let dom = parser::parse_html(html).unwrap();
        let result = EXTRACTOR.extract(&dom);
        assert_eq!(result, expected.map(|s| s.to_string()));
    }
}
