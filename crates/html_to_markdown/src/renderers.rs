pub mod code_block;
pub mod generic_block;
pub mod heading;
pub mod ignored_tags;
pub mod inline;
pub mod list;
pub mod paragraph;
pub mod table;

use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
    utils::normalize_html_text,
};
use std::collections::HashMap;
use std::default::Default;
use std::sync::LazyLock;

#[derive(Debug, Default)]
pub struct Context {
    pub list_depth: usize,
    pub in_table: bool,
    pub preserve_whitespace: bool,
    pub in_heading: bool,
}

pub trait Renderer: Send + Sync {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool;
    fn render(&self, dom: &Dom, id: NodeId, ctx: &mut Context) -> Result<String, ConvertError>;
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
        // span, del, ins, mark, sub, sup and small are not transformed to markdown
        for tag in [
            "strong", "b", "em", "i", "a", "span", "br", "del", "ins", "mark", "sub", "sup",
            "small", "img",
        ] {
            map.insert(tag, inline);
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

        let ignored_tags = &ignored_tags::IGNORED_TAGS as &'static dyn Renderer;
        for tag in ["script", "style", "noscript"] {
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

pub fn render_node(dom: &Dom, id: NodeId, ctx: &mut Context) -> Result<String, ConvertError> {
    // Check if the node is an element and has a registered renderer
    if let NodeData::Element { tag, .. } = &dom.node(id).data {
        if let Some(&renderer) = TAG_RENDERERS.get(tag.local.as_ref()) {
            if renderer.matches(dom, id) {
                return renderer.render(dom, id, ctx);
            }
        }
    }

    // generic renderers: check all registered renderers
    for &renderer in GENERIC_RENDERERS.iter() {
        if renderer.matches(dom, id) {
            return renderer.render(dom, id, ctx);
        }
    }

    // default case: render children recursively
    render_children(dom, id, ctx)
}

pub fn render_children(dom: &Dom, id: NodeId, ctx: &mut Context) -> Result<String, ConvertError> {
    match &dom.node(id).data {
        NodeData::Element { .. } => {
            let children = &dom.node(id).children;

            let mut result = String::with_capacity(children.len() * CHARS_PER_CHILD);

            for &child in children {
                result.push_str(&render_node(dom, child, ctx)?);
            }
            Ok(result)
        }
        NodeData::Text(text) => {
            if ctx.preserve_whitespace {
                Ok(text.clone())
            } else {
                Ok(normalize_html_text(text).unwrap_or_default())
            }
        }
        NodeData::Document => {
            let children = &dom.node(id).children;
            let mut result = String::with_capacity(children.len() * CHARS_PER_CHILD);

            for &child in children {
                result.push_str(&render_node(dom, child, ctx)?);
            }
            Ok(result)
        }
        _ => Ok(String::new()),
    }
}
