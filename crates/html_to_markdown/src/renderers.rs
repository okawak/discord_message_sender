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
use std::cell::RefCell;
use std::collections::HashMap;
use std::default::Default;
use std::sync::LazyLock;

#[derive(Debug, Default)]
pub struct Context {
    pub inline_depth: usize,
    /// Depth of nested lists, used for rendering list items
    pub list_depth: usize,
    pub list_first_item: bool,
    pub in_table: bool,
    pub preserve_whitespace: bool,
    pub in_heading: bool,
    pub link_info: Option<LinkInfo>,
}

#[derive(Debug, Clone)]
pub struct LinkInfo {
    pub url: String,
    pub target_priority: Vec<&'static str>,
    pub applied: RefCell<bool>,
}

impl LinkInfo {
    pub fn new(url: String) -> Self {
        Self {
            url,
            target_priority: vec!["img", "h1", "h2", "h3", "h4", "h5", "h6", "p", "span"],
            applied: RefCell::new(false),
        }
    }

    pub fn is_target_candidate(&self, tag_name: &str) -> bool {
        self.target_priority.contains(&tag_name)
    }

    pub fn try_apply_link(&self, tag_name: &str) -> bool {
        if !*self.applied.borrow() && self.is_target_candidate(tag_name) {
            *self.applied.borrow_mut() = true;
            true
        } else {
            false
        }
    }
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
    if let NodeData::Element { tag, .. } = &node.data {
        if let Some(&renderer) = TAG_RENDERERS.get(tag.local.as_ref()) {
            if renderer.matches(dom, id) {
                return renderer.render(url, dom, id, ctx);
            }
        }
    }

    // generic renderers: check all registered renderers
    for &renderer in GENERIC_RENDERERS.iter() {
        if renderer.matches(dom, id) {
            return renderer.render(url, dom, id, ctx);
        }
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

            let mut result = String::with_capacity(children.len() * CHARS_PER_CHILD);

            for &child in children {
                result.push_str(&render_node(url, dom, child, ctx)?);
            }
            Ok(result)
        }
        NodeData::Text(text) => {
            if ctx.preserve_whitespace {
                Ok(text.clone())
            } else {
                let normalized = normalize_html_text(text, ctx.inline_depth > 0)
                    .map(cow_to_string)
                    .unwrap_or_default();

                if ctx.inline_depth > 0 {
                    return Ok(normalized);
                }

                // Handle list items
                if ctx.list_first_item && ctx.list_depth > 0 && !normalized.is_empty() {
                    // first item in a list
                    ctx.list_first_item = false;
                }

                Ok(normalized)
            }
        }
        NodeData::Document => {
            let children = &node.children;
            let mut result = String::with_capacity(children.len() * CHARS_PER_CHILD);

            for &child in children {
                result.push_str(&render_node(url, dom, child, ctx)?);
            }
            Ok(result)
        }
        _ => Ok(String::new()),
    }
}
