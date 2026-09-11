pub mod aside;
pub mod code_block;
pub mod generic_block;
pub mod heading;
pub mod ignored_tags;
pub mod inline;
pub mod list;
pub mod media;
pub mod paragraph;
pub mod table;

use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
    utils::{cow_to_string, normalize_html_text},
};
use std::collections::HashMap;
use std::default::Default;
use std::sync::LazyLock;

#[derive(Clone, Debug, Default)]
pub struct Context {
    pub in_inline: bool,
    /// Depth of nested lists, used for rendering list items
    pub list_depth: usize,
    pub list_first_item: bool,
    pub in_table: bool,
    pub preserve_whitespace: bool,
    pub in_heading: bool,
    pub in_link_label: bool,
    pub link_info: Option<String>,
    pub suppress_link_boundary_probe: bool,
    /// Last character of the previous output to determine if block separation is needed
    pub last_char: Option<char>,
}

pub trait Renderer: Send + Sync {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool;
    fn render(
        &self,
        url: &str,
        dom: &Dom,
        id: NodeId,
        ctx: &mut Context,
    ) -> Result<String, ConvertError>;
}

/// Number of renderers to preallocate in the map.
const MAP_CAPACITY: usize = 32;
/// Estimated character count per child element
const CHARS_PER_CHILD: usize = 64;

static TAG_RENDERERS: LazyLock<HashMap<&'static str, &'static dyn Renderer>> =
    LazyLock::new(|| {
        let mut map = HashMap::with_capacity(MAP_CAPACITY);

        let heading = &heading::HEADING as &'static dyn Renderer;
        for tag in ["h1", "h2", "h3", "h4", "h5", "h6"] {
            map.insert(tag, heading);
        }

        map.insert("p", &paragraph::PARAGRAPH as &'static dyn Renderer);

        let inline = &inline::INLINE as &'static dyn Renderer;
        // <span>, <del>, <ins>, <mark>, <sub>, <sup> and <small> are not transformed to markdown
        // (text is preserved as is)
        for tag in [
            "strong", "b", "em", "i", "span", "br", "del", "ins", "mark", "sub", "sup", "small",
        ] {
            map.insert(tag, inline);
        }

        let media = &media::MEDIA as &'static dyn Renderer;
        for tag in ["a", "img", "audio", "video"] {
            map.insert(tag, media);
        }

        let code_block = &code_block::CODE_BLOCK as &'static dyn Renderer;
        for tag in ["pre", "code"] {
            map.insert(tag, code_block);
        }

        let table = &table::TABLE as &'static dyn Renderer;
        for tag in ["table", "thead", "tbody", "tr", "th", "td"] {
            map.insert(tag, table);
        }

        let list = &list::LIST as &'static dyn Renderer;
        for tag in ["ul", "ol", "li"] {
            map.insert(tag, list);
        }

        map.insert("aside", &aside::ASIDE as &'static dyn Renderer);

        let ignored_tags = &ignored_tags::IGNORED_TAGS as &'static dyn Renderer;
        for tag in ["script", "style", "noscript", "footer", "nav"] {
            map.insert(tag, ignored_tags);
        }

        map
    });

static GENERIC_RENDERERS: LazyLock<Vec<&'static dyn Renderer>> = LazyLock::new(|| {
    // priority order of renderers
    vec![
        &code_block::CODE_BLOCK as &'static dyn Renderer, // for elements like <div class="code-block">
        &generic_block::BLOCK as &'static dyn Renderer,
    ]
});

pub(crate) fn is_block_element(tag_name: &str) -> bool {
    matches!(
        tag_name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "details"
            | "div"
            | "dl"
            | "dt"
            | "dd"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "form"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "p"
            | "pre"
            | "section"
            | "summary"
            | "table"
            | "ul"
    )
}

fn rendered_node_is_inline(dom: &Dom, id: NodeId, rendered: &str) -> bool {
    if rendered == "<br>" {
        return false;
    }

    matches!(
        dom.node(id).map(|node| &node.data),
        Some(NodeData::Element { tag, .. }) if tag.local.as_ref() == "img"
    ) || (!rendered.starts_with('\n') && !rendered.ends_with('\n'))
}

fn render_child_nodes(
    url: &str,
    dom: &Dom,
    children: &[NodeId],
    ctx: &mut Context,
) -> Result<String, ConvertError> {
    render_child_nodes_with(url, dom, children, ctx, |_, _, _, _| {})
}

fn render_child_nodes_with<F>(
    url: &str,
    dom: &Dom,
    children: &[NodeId],
    ctx: &mut Context,
    mut before_append: F,
) -> Result<String, ConvertError>
where
    F: FnMut(&mut String, NodeId, &str, &mut Context),
{
    let mut result = String::with_capacity(children.len() * CHARS_PER_CHILD);
    let mut previous_was_inline = false;
    let mut pending_whitespace = false;

    for &child_id in children {
        if !ctx.preserve_whitespace
            && matches!(
                dom.node(child_id).map(|child| &child.data),
                Some(NodeData::Text(text))
                    if !text.is_empty() && text.chars().all(char::is_whitespace)
            )
        {
            pending_whitespace |= previous_was_inline;
            continue;
        }

        let (starts_with_whitespace, ends_with_whitespace) = if ctx.preserve_whitespace {
            (false, false)
        } else {
            match dom.node(child_id).map(|child| &child.data) {
                Some(NodeData::Text(text)) => (
                    text.starts_with(char::is_whitespace),
                    text.ends_with(char::is_whitespace),
                ),
                _ => (false, false),
            }
        };
        pending_whitespace |= starts_with_whitespace && previous_was_inline;

        let rendered = render_node(url, dom, child_id, ctx)?;
        if rendered.is_empty() {
            pending_whitespace |= ends_with_whitespace && previous_was_inline;
            continue;
        }

        let current_is_inline = rendered_node_is_inline(dom, child_id, &rendered);
        if rendered != "<br>" && pending_whitespace && previous_was_inline {
            if current_is_inline {
                let already_separated = result.chars().next_back().is_some_and(char::is_whitespace)
                    || rendered.chars().next().is_some_and(char::is_whitespace);
                if !already_separated {
                    result.push(' ');
                }
            } else if !result.ends_with('\n') && !rendered.starts_with('\n') {
                result.push_str("\n\n");
            }
        }
        before_append(&mut result, child_id, &rendered, ctx);
        result.push_str(&rendered);
        previous_was_inline = current_is_inline;
        pending_whitespace = ends_with_whitespace && current_is_inline;
    }

    Ok(result)
}

pub fn render_node(
    url: &str,
    dom: &Dom,
    id: NodeId,
    ctx: &mut Context,
) -> Result<String, ConvertError> {
    let Some(node) = dom.node(id) else {
        return Err(ConvertError::InvalidNode(format!("Node {id} not found")));
    };

    // Check if the node is an element and has a registered renderer
    if let NodeData::Element { tag, .. } = &node.data
        && let Some(&renderer) = TAG_RENDERERS.get(tag.local.as_ref())
        && renderer.matches(dom, id)
    {
        return renderer.render(url, dom, id, ctx);
    }

    // generic renderers: check all registered renderers
    for &renderer in GENERIC_RENDERERS.iter() {
        if renderer.matches(dom, id) {
            return renderer.render(url, dom, id, ctx);
        }
    }

    if let NodeData::Element { tag, .. } = &node.data
        && ctx.in_link_label
        && is_block_element(tag.local.as_ref())
    {
        let content = render_children(url, dom, id, ctx)?;
        return Ok(format!(" {} ", content.trim()));
    }

    // default case: render children recursively
    render_children(url, dom, id, ctx)
}

pub fn render_children(
    url: &str,
    dom: &Dom,
    id: NodeId,
    ctx: &mut Context,
) -> Result<String, ConvertError> {
    let Some(node) = dom.node(id) else {
        return Err(ConvertError::InvalidNode(format!("Node {id} not found")));
    };
    match &node.data {
        NodeData::Element { .. } => {
            let children = &node.children;
            render_child_nodes(url, dom, children, ctx)
        }
        NodeData::Text(text) => {
            if ctx.preserve_whitespace {
                Ok(text.clone())
                //Ok(format_list_content(ctx, text))
            } else {
                let normalized = normalize_html_text(text, ctx.in_inline)
                    .map(cow_to_string)
                    .unwrap_or_default();

                //Ok(format_list_content(ctx, &normalized))
                Ok(normalized)
            }
        }
        NodeData::Document => {
            let children = &node.children;
            render_child_nodes(url, dom, children, ctx)
        }
        _ => Ok(String::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(
        "<p><strong>Hello</strong> <em>world</em></p>",
        "**Hello** *world*\n\n"
    )]
    #[case("<p><span>Hello</span>\n\t<span>world</span></p>", "Hello world\n\n")]
    #[case(
        "<p><strong>Hello</strong> <span></span> <script>ignored</script> <em>world</em></p>",
        "**Hello** *world*\n\n"
    )]
    #[case("<p><span>Hello </span> <span>world</span></p>", "Hello world\n\n")]
    #[case("<p><span>Hello</span> <span> world</span></p>", "Hello world\n\n")]
    #[case("<p>A&nbsp;B 👩‍💻 A‌B</p>", "A B 👩‍💻 A‌B\n\n")]
    #[case("<p>A&nbsp;<strong>B</strong></p>", "A **B**\n\n")]
    #[case("<p><strong>A</strong>&nbsp;B</p>", "**A** B\n\n")]
    #[case("<p>A   <strong>B</strong></p>", "A **B**\n\n")]
    #[case("<p>Hello \n<br>\n world</p>", "Hello<br>world\n\n")]
    #[case(
        "<p>A <img src=\"https://example.com/x\" alt=\"X\"> B</p>",
        "A ![X](https://example.com/x)\n\n B\n\n"
    )]
    #[case(
        "<div><span>Published</span> <div><span>Updated</span></div></div>",
        "Published Updated"
    )]
    #[case("<span>Hello</span> <div><p>world</p></div>", "Hello\n\nworld\n\n")]
    fn preserves_semantic_html_whitespace(#[case] html: &str, #[case] expected: &str) {
        let dom = parser::parse_html(html).expect("HTML should parse");
        let output = render_node("", &dom, dom.document, &mut Context::default())
            .expect("HTML should render");

        assert_eq!(output, expected);
    }
}
