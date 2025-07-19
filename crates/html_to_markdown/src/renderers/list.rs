use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct List;

impl List {
    fn is_ordered_list(&self, dom: &Dom, id: NodeId) -> bool {
        let Ok(Some(parent_id)) = dom.get_parent(id) else {
            return false;
        };

        if let Ok((parent_tag, _)) = dom.get_element_data(parent_id) {
            parent_tag.local.as_ref() == "ol"
        } else {
            false
        }
    }
}

impl Renderer for List {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        let Some(node) = dom.node(id) else {
            return false;
        };

        if let NodeData::Element { tag, .. } = &node.data {
            matches!(tag.local.as_ref(), "ul" | "ol" | "li")
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
        let (tag, _) = dom.get_element_data(id)?;

        match tag.local.as_ref() {
            "ul" => {
                ctx.list_depth += 2; // "- " is 2 characters
                let content = render_children(url, dom, id, ctx)?;
                ctx.list_depth -= 2;

                // Outermost list performs final formatting
                if ctx.list_depth == 0 {
                    if content.trim().is_empty() {
                        Ok(String::new())
                    } else {
                        Ok(format!("{}\n\n", content.trim_end()))
                    }
                } else {
                    // Nested list - add leading newline
                    Ok(format!("\n{content}"))
                }
            }
            "ol" => {
                ctx.list_depth += 3; // "1. " is 3 characters
                let content = render_children(url, dom, id, ctx)?;
                ctx.list_depth -= 3;

                // Outermost list performs final formatting
                if ctx.list_depth == 0 {
                    if content.trim().is_empty() {
                        Ok(String::new())
                    } else {
                        Ok(format!("{}\n\n", content.trim_end()))
                    }
                } else {
                    // Nested list - add leading newline
                    Ok(format!("\n{content}"))
                }
            }
            "li" => {
                ctx.list_first_item = true;
                let content = render_children(url, dom, id, ctx)?;

                if content.trim().is_empty() {
                    return Ok(String::new());
                }

                // list content should not have leading/trailing whitespace - saturating_sub(n)
                let marker = if self.is_ordered_list(dom, id) {
                    format!("{}1.", " ".repeat(ctx.list_depth.saturating_sub(3)))
                } else {
                    format!("{}-", " ".repeat(ctx.list_depth.saturating_sub(2)))
                };

                Ok(format!("{marker} {content}\n"))
            }
            _ => render_children(url, dom, id, ctx),
        }
    }
}

pub static LIST: List = List;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::renderers;
    use indoc::indoc;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    /// Basic unordered list tests
    #[rstest]
    #[case(
        "<ul><li>First item</li><li>Second item</li></ul>",
        indoc! {r#"
            - First item
            - Second item

            "#}
    )]
    #[case("<ul><li>Single item</li></ul>", "- Single item\n\n")]
    #[case("<ul></ul>", "")]
    fn test_basic_unordered_lists(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render unordered list");
        assert_eq!(result, expected);
    }

    /// Basic ordered list tests
    #[rstest]
    #[case(
        "<ol><li>First item</li><li>Second item</li></ol>",
        indoc! {r#"
            1. First item
            1. Second item

            "#}
    )]
    #[case("<ol><li>Single item</li></ol>", "1. Single item\n\n")]
    #[case("<ol></ol>", "")]
    fn test_basic_ordered_lists(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render ordered list");
        assert_eq!(result, expected);
    }

    /// Nested list tests
    #[rstest]
    #[case(
        "<ul><li>Top level<ul><li>Nested item</li></ul></li></ul>",
        indoc! {r#"
            - Top level
              - Nested item

            "#}
    )]
    #[case(
        "<ol><li>First<ul><li>Nested unordered</li></ul></li><li>Second</li></ol>",
        indoc! {r#"
            1. First
               - Nested unordered
            1. Second

            "#}
    )]
    #[case(
        "<ul><li>Level 1<ul><li>Level 2<ul><li>Level 3</li></ul></li></ul></li></ul>",
        indoc! {r#"
            - Level 1
              - Level 2
                - Level 3

            "#}
    )]
    fn test_nested_lists(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render nested list");
        assert_eq!(result, expected);
    }

    /// Lists with inline formatting
    #[rstest]
    #[case(
        "<ul><li><strong>Bold</strong> item</li><li><em>Italic</em> item</li></ul>",
        indoc! {r#"
            - **Bold** item
            - *Italic* item

            "#}
    )]
    #[case(
        "<ul><li><code>inline code</code> in list</li></ul>",
        indoc! {r#"
            - `inline code` in list

            "#}
    )]
    #[case(
        "<ul><li><a href=\"https://example.com\">Link</a> in list</li></ul>",
        indoc! {r#"
            - [Link](https://example.com) in list

            "#}
    )]
    fn test_lists_with_inline_formatting(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render list with inline formatting");
        assert_eq!(result, expected);
    }

    /// Lists with code blocks
    #[rstest]
    #[case(
        "<ul><li>Install CLI tool<pre><code>npm install -g tool</code></pre></li></ul>",
        indoc! {r#"
            - Install CLI tool

              ```
              npm install -g tool
              ```

            "#}
    )]
    #[case(
        r#"<ul><li>Setup configuration<pre><code class="language-bash">cp config.example config.json</code></pre></li></ul>"#,
        indoc! {r#"
            - Setup configuration

              ```bash
              cp config.example config.json
              ```

            "#}
    )]
    #[case(
        "<ol><li>Step one<pre><code>command one</code></pre></li><li>Step two<pre><code>command two</code></pre></li></ol>",
        indoc! {r#"
            1. Step one

               ```
               command one
               ```

            1. Step two

               ```
               command two
               ```

            "#}
    )]
    fn test_lists_with_code_blocks(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render list with code blocks");
        assert_eq!(result, expected);
    }

    /// Lists with complex nested content
    #[rstest]
    #[case(
        "<ul><li>Installation guide:<br>Follow these steps:<pre><code>npm install package</code></pre></li></ul>",
        indoc! {r#"
            - Installation guide:<br>Follow these steps:

              ```
              npm install package
              ```

            "#}
    )]
    #[case(
        "<ul><li>First paragraph<p>Second paragraph</p><pre><code>code block</code></pre></li></ul>",
        indoc! {r#"
            - First paragraph

              Second paragraph

              ```
              code block
              ```

            "#}
    )]
    fn test_lists_with_complex_content(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render list with complex content");
        assert_eq!(result, expected);
    }

    /// Lists with multiple paragraphs
    #[rstest]
    #[case(
        "<ul><li><p>First paragraph</p><p>Second paragraph</p></li></ul>",
        indoc! {r#"
            - First paragraph

              Second paragraph

            "#}
    )]
    #[case(
        "<ol><li><p>Introduction text</p><p>More details here</p></li><li><p>Another item</p></li></ol>",
        indoc! {r#"
            1. Introduction text

               More details here

            1. Another item

            "#}
    )]
    fn test_lists_with_multiple_paragraphs(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render list with multiple paragraphs");
        assert_eq!(result, expected);
    }

    /// Empty and edge case tests
    #[rstest]
    #[case("<ul><li></li></ul>", "")]
    #[case("<ul><li>   </li></ul>", "")]
    #[case("<ul><li><p></p></li></ul>", "")]
    #[case("<ol><li></li><li>Second</li></ol>", "1. Second\n\n")]
    fn test_empty_and_edge_cases(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render edge case list");
        assert_eq!(result, expected);
    }

    /// Mixed list types
    #[rstest]
    #[case(
        "<div><ul><li>Unordered first</li></ul><ol><li>Ordered second</li></ol></div>",
        indoc! {r#"
            - Unordered first

            1. Ordered second

            "#}
    )]
    #[case(
        "<ul><li>Before nested<ol><li>Ordered in unordered</li></ol>After nested</li></ul>",
        indoc! {r#"
            - Before nested

              1. Ordered in unordered

              After nested

            "#}
    )]
    fn test_mixed_list_types(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render mixed list types");
        assert_eq!(result, expected);
    }

    /// Real-world complex structures
    #[rstest]
    #[case(
        r#"<ol>
            <li>
                Install the CLI tool:
                <br>
                Use your preferred package manager:
                <pre><code class="language-bash">npm install -g my-tool</code></pre>
                Verify installation:
                <pre><code class="language-bash">my-tool --version</code></pre>
            </li>
            <li>
                Configure the tool with your settings.
            </li>
        </ol>"#,
        indoc! {r#"
            1. Install the CLI tool:<br>Use your preferred package manager:

               ```bash
               npm install -g my-tool
               ```

               Verify installation:

               ```bash
               my-tool --version
               ```

            1. Configure the tool with your settings.

            "#}
    )]
    fn test_real_world_complex_structure(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render real-world structure");
        assert_eq!(result, expected);
    }

    /// Deeply nested lists with various content
    #[rstest]
    #[case(
        "<ul><li>Level 1<ul><li>Level 2 with <strong>bold</strong><ul><li>Level 3 with <code>code</code></li></ul></li></ul></li></ul>",
        indoc! {r#"
            - Level 1
              - Level 2 with **bold**
                - Level 3 with `code`

            "#}
    )]
    #[case(
        "<ol><li>Step 1<ol><li>Sub-step A<pre><code>command</code></pre></li><li>Sub-step B</li></ol></li></ol>",
        indoc! {r#"
            1. Step 1
               1. Sub-step A

                  ```
                  command
                  ```

               1. Sub-step B

            "#}
    )]
    fn test_deeply_nested_complex_lists(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render deeply nested list");
        assert_eq!(result, expected);
    }

    // Lists with images and links
    //#[rstest]
    //#[case(
    //    "<ul><li><img src=\"/icon.png\" alt=\"Icon\">Item with image</li></ul>",
    //    indoc! {r#"
    //        - ![Icon](https://example.com/icon.png)

    //          Item with image

    //        "#}
    //)]
    //#[case(
    //    "<ul><li><a href=\"/page\"><img src=\"thumb.jpg\" alt=\"Thumbnail\">Link with image</a></li></ul>",
    //    indoc! {r#"
    //        - [![Thumbnail]](https://example.com/thumb.jpg) Link with image

    //        "#}
    //)]
    //fn test_lists_with_media(#[case] html: &str, #[case] expected: &str) {
    //    let dom = parser::parse_html(html).expect("Failed to parse HTML");
    //    let mut context = Context::default();
    //    let result =
    //        renderers::render_node("https://example.com", &dom, dom.document, &mut context)
    //            .expect("Failed to render list with media");
    //    assert_eq!(result, expected);
    //}

    /// Lists with line breaks
    #[rstest]
    #[case(
        "<ul><li>First line<br>Second line</li></ul>",
        indoc! {r#"
            - First line<br>Second line

            "#}
    )]
    #[case(
        "<ul><li>Multiple<br>line<br>breaks</li></ul>",
        indoc! {r#"
            - Multiple<br>line<br>breaks

            "#}
    )]
    fn test_lists_with_line_breaks(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("Failed to parse HTML");
        let mut context = Context::default();
        let result = renderers::render_node("", &dom, dom.document, &mut context)
            .expect("Failed to render list with line breaks");
        assert_eq!(result, expected);
    }
}
