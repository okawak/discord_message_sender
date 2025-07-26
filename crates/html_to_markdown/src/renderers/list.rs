use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

#[derive(Debug, Clone, Copy, PartialEq)]
enum ListType {
    Unordered,
    Ordered,
}

impl ListType {
    const fn indent_size(self) -> usize {
        match self {
            ListType::Unordered => 2, // size of "- "
            ListType::Ordered => 3,   // size of "1. "
        }
    }

    fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "ul" => Some(ListType::Unordered),
            "ol" => Some(ListType::Ordered),
            _ => None,
        }
    }
}

pub struct List;

impl List {
    fn is_ordered_list(&self, dom: &Dom, id: NodeId) -> bool {
        dom.get_parent(id)
            .ok()
            .flatten()
            .and_then(|parent_id| dom.get_element_data(parent_id).ok())
            .map(|(tag, _)| tag.local.as_ref() == "ol")
            .unwrap_or(false)
    }

    /// Check if this list item needs extra spacing before it
    fn needs_spacing_before_item(&self, dom: &Dom, id: NodeId) -> bool {
        let parent_id = match dom.get_parent(id).ok().flatten() {
            Some(id) => id,
            None => return false,
        };

        let children: Vec<_> = match dom.iter_children(parent_id) {
            Ok(iter) => iter.collect(),
            Err(_) => return false,
        };

        // Find the index of current item
        let Some(current_index) = children.iter().position(|&&child_id| child_id == id) else {
            return false;
        };

        // If this is the first item, no spacing needed
        if current_index == 0 {
            return false;
        }

        // Find the previous <li> element (skip text nodes)
        for &&prev_child_id in children[..current_index].iter().rev() {
            if let Some(prev_node) = dom.node(prev_child_id) {
                if let NodeData::Element { tag, .. } = &prev_node.data {
                    if tag.local.as_ref() == "li" {
                        return self.previous_item_ends_with_block_element(dom, &prev_child_id);
                    }
                }
            }
        }

        false
    }

    /// Check if the previous list item ends with a block element (p or pre)
    fn previous_item_ends_with_block_element(&self, dom: &Dom, item_id: &NodeId) -> bool {
        let children: Vec<_> = match dom.iter_children(*item_id) {
            Ok(iter) => iter.collect(),
            Err(_) => return false,
        };

        for &&child_id in children.iter().rev() {
            let Some(child_node) = dom.node(child_id) else {
                continue;
            };

            match &child_node.data {
                NodeData::Element { tag, .. } => {
                    return matches!(tag.local.as_ref(), "p" | "pre");
                }
                NodeData::Text(text) if !text.trim().is_empty() => {
                    return false;
                }
                _ => continue,
            }
        }

        false
    }

    // Custom rendering for list item children to handle proper spacing
    fn render_list_item_children(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        ctx: &mut Context,
    ) -> Result<String, ConvertError> {
        let mut result = String::new();
        let indent = " ".repeat(ctx.list_depth);

        let children: Vec<_> = dom.iter_children(id)?.collect();
        let mut prev_was_code_block = false;

        for (index, &child_id) in children.iter().enumerate() {
            let child_result = super::render_node(url, dom, *child_id, ctx)?;

            if prev_was_code_block && index > 0 && !child_result.trim().is_empty() {
                result.push_str(&format!("\n\n{indent}"));
            }

            result.push_str(&child_result);

            if !child_result.trim().is_empty() {
                ctx.list_first_item = false;
            }

            if let Some(child_node) = dom.node(*child_id) {
                prev_was_code_block = match &child_node.data {
                    NodeData::Element { tag, .. } => tag.local.as_ref() == "pre",
                    NodeData::Text(text) => {
                        if !text.trim().is_empty() {
                            false
                        } else {
                            prev_was_code_block
                        }
                    }
                    _ => false,
                };
            }
        }

        Ok(result)
    }
}

impl Renderer for List {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        dom.node(id)
            .and_then(|node| match &node.data {
                NodeData::Element { tag, .. } => Some(tag.local.as_ref()),
                _ => None,
            })
            .map(|tag| matches!(tag, "ul" | "ol" | "li"))
            .unwrap_or(false)
    }

    fn render(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        ctx: &mut Context,
    ) -> Result<String, ConvertError> {
        let (tag, _) = dom.get_element_data(id)?;
        let tag_name = tag.local.as_ref();

        match tag_name {
            "ul" | "ol" => self.render_list(url, dom, id, ctx, tag_name),
            "li" => self.render_list_item(url, dom, id, ctx),
            _ => render_children(url, dom, id, ctx),
        }
    }
}

impl List {
    fn render_list(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        ctx: &mut Context,
        tag: &str,
    ) -> Result<String, ConvertError> {
        let list_type = ListType::from_tag(tag)
            .ok_or_else(|| ConvertError::Unsupported(format!("Unknown list tag: {tag}")))?;

        ctx.list_depth += list_type.indent_size();
        let content = render_children(url, dom, id, ctx)?;
        ctx.list_depth -= list_type.indent_size();

        if ctx.list_depth == 0 {
            if content.trim().is_empty() {
                Ok(String::new())
            } else {
                Ok(format!("{}\n\n", content.trim_end()))
            }
        } else {
            Ok(format!("\n{}", content.trim_end()))
        }
    }

    fn render_list_item(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        ctx: &mut Context,
    ) -> Result<String, ConvertError> {
        let needs_spacing = self.needs_spacing_before_item(dom, id);
        let prefix = if needs_spacing { "\n" } else { "" };

        ctx.list_first_item = true;
        let content = self.render_list_item_children(url, dom, id, ctx)?;

        if content.trim().is_empty() {
            return Ok(String::new());
        }

        // list content should not have leading/trailing whitespace - saturating_sub(n)
        let marker = if self.is_ordered_list(dom, id) {
            format!("{}1.", " ".repeat(ctx.list_depth.saturating_sub(3)))
        } else {
            format!("{}-", " ".repeat(ctx.list_depth.saturating_sub(2)))
        };

        Ok(format!("{prefix}{marker} {content}\n"))
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
        "<ul><li>Before nested<ol><li>Ordered in unordered</li></ol><p>After nested</p></li></ul>",
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
    #[case(
        r#"<ol><li><p>Project setup:</p><div class="div-1"><div class="div-2"><pre><code class="language-bash">npm init -y</code></pre></div></div></li></ol>"#,
        indoc! {r#"
            1. Project setup:

               ```bash
               npm init -y
               ```

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
