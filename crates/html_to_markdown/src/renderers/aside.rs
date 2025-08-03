use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
    utils::filtering,
};

pub struct Aside;

impl Renderer for Aside {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        let Some(node) = dom.node(id) else {
            return false;
        };

        if let NodeData::Element { tag, .. } = &node.data {
            tag.local.as_ref() == "aside"
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
        let (_, attrs) = dom.get_element_data(id)?;

        // Check if class should be ignored
        if let Some(class_value) = attrs.get("class")
            && filtering::should_ignore_class(class_value)
        {
            return Ok(String::new());
        }

        // Render children for other aside elements
        let content = render_children(url, dom, id, ctx)?;
        if content.trim().is_empty() {
            Ok(String::new())
        } else {
            let indent = " ".repeat(ctx.list_depth);
            Ok(format!("{indent}{}\n\n", content.trim()))
        }
    }
}

pub static ASIDE: Aside = Aside;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::renderers;
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// Test ignored classes
    #[rstest]
    #[case(r#"<aside class="sidebar">Content</aside>"#, "")]
    #[case(r#"<aside class="author-info">Author content</aside>"#, "")]
    #[case(r#"<aside class="publication-meta">Publication content</aside>"#, "")]
    #[case(r#"<aside class="mobile-nav">Mobile content</aside>"#, "")]
    #[case(r#"<aside class="SIDEBAR">Content</aside>"#, "")] // Case insensitive
    #[case(r#"<aside class="Author">Author content</aside>"#, "")]
    #[case(r#"<aside class="my-sidebar-widget">Content</aside>"#, "")] // Contains keyword
    #[case(r#"<aside class="main-author-bio">Content</aside>"#, "")] // Contains keyword
    fn test_ignored_aside_classes(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render aside");
        assert_eq!(result, expected);
    }

    /// Test rendered classes
    #[rstest]
    #[case(
        r#"<aside class="note">Important note content</aside>"#,
        "Important note content\n\n"
    )]
    #[case(
        r#"<aside class="warning">Warning message</aside>"#,
        "Warning message\n\n"
    )]
    #[case(
        r#"<aside class="info">Information box</aside>"#,
        "Information box\n\n"
    )]
    #[case(r#"<aside>No class content</aside>"#, "No class content\n\n")] // No class attribute
    fn test_rendered_aside_classes(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render aside");
        assert_eq!(result, expected);
    }

    /// Test empty aside elements
    #[rstest]
    #[case(r#"<aside class="note"></aside>"#, "")]
    #[case(r#"<aside class="sidebar"></aside>"#, "")]
    #[case(r#"<aside></aside>"#, "")]
    fn test_empty_aside_elements(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render aside");
        assert_eq!(result, expected);
    }

    /// Test multiple classes
    #[rstest]
    #[case(r#"<aside class="widget sidebar important">Content</aside>"#, "")] // Contains ignored keyword
    #[case(r#"<aside class="note warning info">Content</aside>"#, "Content\n\n")] // No ignored keywords
    #[case(r#"<aside class="custom-author-widget">Content</aside>"#, "")] // Contains author
    fn test_multiple_classes(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render aside");
        assert_eq!(result, expected);
    }

    /// Test nested content
    #[rstest]
    #[case(
        r#"<aside class="note"><p>Important information</p><ul><li>Item 1</li><li>Item 2</li></ul></aside>"#,
        indoc! { r#"
            Important information

            - Item 1
            - Item 2

            "#
        }
    )]
    #[case(r#"<aside class="sidebar"><p>This should be ignored</p></aside>"#, "")]
    fn test_nested_content(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render aside");
        assert_eq!(result, expected);
    }
}
