use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
    utils::normalize_heading_content,
};

pub struct Heading;

impl Renderer for Heading {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            matches!(tag.local.as_ref(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
        } else {
            false
        }
    }

    fn render(&self, dom: &Dom, id: NodeId, ctx: &mut Context) -> Result<String, ConvertError> {
        let old_preserve = ctx.preserve_whitespace;
        let old_in_heading = ctx.in_heading;

        ctx.preserve_whitespace = true; // Preserve whitespace for headings
        ctx.in_heading = true; // Process in heading context

        let content = render_children(dom, id, ctx)?;

        ctx.preserve_whitespace = old_preserve; // Restore original setting
        ctx.in_heading = old_in_heading;

        let normalized_content = normalize_heading_content(&content);

        if normalized_content.trim().is_empty() {
            return Ok(String::new());
        }

        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            let level = match tag.local.as_ref() {
                "h1" => "#",
                "h2" => "##",
                "h3" => "###",
                "h4" => "####",
                "h5" => "#####",
                "h6" => "######",
                _ => "#",
            };
            Ok(format!("{} {}\n\n", level, normalized_content.trim()))
        } else {
            Ok(normalized_content)
        }
    }
}

pub static HEADING: Heading = Heading;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::renderers;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// basic heading tests
    #[rstest]
    #[case("<h1>Main Title</h1>", "# Main Title\n\n")]
    #[case("<h2>Section Title</h2>", "## Section Title\n\n")]
    #[case("<h3>Subsection</h3>", "### Subsection\n\n")]
    #[case("<h4>Sub-subsection</h4>", "#### Sub-subsection\n\n")]
    #[case("<h5>Deep Level</h5>", "##### Deep Level\n\n")]
    #[case("<h6>Deepest Level</h6>", "###### Deepest Level\n\n")]
    fn test_basic_headings(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(&dom, dom.document, &mut context)
            .expect("Failed to render heading");
        assert_eq!(result, expected);
    }

    /// whitespace normalization tests
    #[rstest]
    #[case("<h1>  Title with spaces  </h1>", "# Title with spaces\n\n")]
    #[case("<h1>\n  Multiline\n  Title\n</h1>", "# Multiline Title\n\n")]
    #[case("<h1>\t\tTab Title\t\t</h1>", "# Tab Title\n\n")]
    #[case("<h1>Title\nwith\nnewlines</h1>", "# Title with newlines\n\n")]
    fn test_whitespace_normalization(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(&dom, dom.document, &mut context)
            .expect("Failed to render heading");
        assert_eq!(result, expected);
    }

    /// empty headings tests
    #[rstest]
    #[case("<h1></h1>", "")]
    #[case("<h1>   </h1>", "")]
    #[case("<h1>\n\n\n</h1>", "")]
    #[case("<h1>\t\t\t</h1>", "")]
    fn test_empty_headings(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(&dom, dom.document, &mut context)
            .expect("Failed to render heading");
        assert_eq!(result, expected);
    }

    /// nested formatting tests
    #[rstest]
    #[case("<h1><strong>Bold</strong> Title</h1>", "# **Bold** Title\n\n")]
    #[case("<h1><em>Italic</em> Text</h1>", "# *Italic* Text\n\n")]
    #[case(
        "<h1>Getting Started with <strong>React</strong></h1>",
        "# Getting Started with **React**\n\n"
    )]
    #[case(
        "<h2><span>Section</span> <strong>Number</strong> <em>One</em></h2>",
        "## Section **Number** *One*\n\n"
    )]
    #[case(
        "<h3><code>function</code> Declaration</h3>",
        "### `function` Declaration\n\n"
    )]
    fn test_nested_formatting(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(&dom, dom.document, &mut context)
            .expect("Failed to render heading");
        assert_eq!(result, expected);
    }

    /// links in headings tests
    #[rstest]
    #[case(
        "<h1><a href=\"https://example.com\">External Link</a></h1>",
        "# [External Link](https://example.com)\n\n"
    )]
    #[case(
        "<h1><a href=\"/blog/post-1\">Blog Post Title</a></h1>",
        "# [Blog Post Title](/blog/post-1)\n\n"
    )]
    #[case(
        "<h2>Section with <a href=\"#anchor\">Internal Link</a></h2>",
        "## Section with [Internal Link](#anchor)\n\n"
    )]
    #[case(
        "<h1>Before <a href=\"/path\">Link</a> After</h1>",
        "# Before [Link](/path) After\n\n"
    )]
    fn test_headings_with_links(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(&dom, dom.document, &mut context)
            .expect("Failed to render heading");
        assert_eq!(result, expected);
    }

    /// nested and complex structures in headings
    #[rstest]
    #[case(
        "<h1><span class=\"prefix\">Chapter 1:</span> <strong>Introduction</strong> to <em>Web Development</em></h1>",
        "# Chapter 1: **Introduction** to *Web Development*\n\n"
    )]
    #[case(
        "<h2><a href=\"/section\"><strong>Important</strong> Section</a></h2>",
        "## [**Important** Section](/section)\n\n"
    )]
    #[case(
        "<h3><code>const</code> vs <code>let</code> in <strong>JavaScript</strong></h3>",
        "### `const` vs `let` in **JavaScript**\n\n"
    )]
    fn test_complex_nested_structures(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(&dom, dom.document, &mut context)
            .expect("Failed to render heading");
        assert_eq!(result, expected);
    }

    /// HTML entities and special characters in headings
    #[rstest]
    #[case("<h1>Using &lt;script&gt; tags</h1>", "# Using <script> tags\n\n")]
    #[case("<h1>Math: 2 &gt; 1 &amp; 1 &lt; 2</h1>", "# Math: 2 > 1 & 1 < 2\n\n")]
    #[case(
        "<h1>Quotes: &quot;Hello&quot; &amp; 'World'</h1>",
        "# Quotes: \"Hello\" & 'World'\n\n"
    )]
    #[case("<h1>Non-breaking&nbsp;space</h1>", "# Non-breaking space\n\n")]
    fn test_html_entities_and_special_chars(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(&dom, dom.document, &mut context)
            .expect("Failed to render heading");
        assert_eq!(result, expected);
    }

    /// heading attributes and classes (ignored in rendering)
    #[rstest]
    #[case("<h1 id=\"main-title\">Title with ID</h1>", "# Title with ID\n\n")]
    #[case(
        "<h1 class=\"large-heading\">Title with Class</h1>",
        "# Title with Class\n\n"
    )]
    #[case("<h1 style=\"color: red;\">Styled Title</h1>", "# Styled Title\n\n")]
    #[case(
        "<h1 data-test=\"heading\">Title with Data</h1>",
        "# Title with Data\n\n"
    )]
    fn test_headings_with_attributes(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(&dom, dom.document, &mut context)
            .expect("Failed to render heading");
        assert_eq!(result, expected);
    }

    /// real-world heading structures
    #[rstest]
    #[case(
        r#"<h1><span class="icon">📚</span> Documentation <small>v2.0</small></h1>"#,
        "# 📚 Documentation v2.0\n\n"
    )]
    #[case(
        r#"<h2><span class="number">01.</span> Getting Started</h2>"#,
        "## 01. Getting Started\n\n"
    )]
    #[case(
        r#"<h3><img src="/icon.png" alt="Star"> Featured Article</h3>"#,
        "### Star Featured Article\n\n"
    )]
    #[case(
        r#"<h1><time datetime="2024-01-01">Jan 1, 2024</time>: New Year Post</h1>"#,
        "# Jan 1, 2024: New Year Post\n\n"
    )]
    fn test_real_world_heading_structures(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(&dom, dom.document, &mut context)
            .expect("Failed to render heading");
        assert_eq!(result, expected);
    }

    /// heading with line breaks
    #[rstest]
    #[case("<h1>Multi<br>Line<br>Title</h1>", "# Multi Line Title\n\n")]
    #[case("<h1>Title<br><small>Subtitle</small></h1>", "# Title Subtitle\n\n")]
    fn test_headings_with_line_breaks(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(&dom, dom.document, &mut context)
            .expect("Failed to render heading");
        assert_eq!(result, expected);
    }

    /// error cases for headings
    #[rstest]
    #[case("<h1><script>alert('xss')</script>Title</h1>", "# Title\n\n")]
    #[case("<h1><style>h1{color:red}</style>Title</h1>", "# Title\n\n")]
    #[case("<h1><!-- comment -->Title</h1>", "# Title\n\n")]
    fn test_headings_with_ignored_content(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(&dom, dom.document, &mut context)
            .expect("Failed to render heading");
        assert_eq!(result, expected);
    }

    /// long heading tests
    #[rstest]
    #[case(
        "<h1>This is a very long heading that spans multiple words and might need special handling for formatting and line breaks</h1>",
        "# This is a very long heading that spans multiple words and might need special handling for formatting and line breaks\n\n"
    )]
    fn test_long_headings(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node(&dom, dom.document, &mut context)
            .expect("Failed to render heading");
        assert_eq!(result, expected);
    }
}
