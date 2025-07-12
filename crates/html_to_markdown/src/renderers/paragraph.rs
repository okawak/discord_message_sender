use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct Paragraph;

impl Renderer for Paragraph {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        let Some(node) = dom.node(id) else {
            return false;
        };

        if let NodeData::Element { tag, .. } = &node.data {
            tag.local.as_ref() == "p"
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
        let old_inline_depth = ctx.inline_depth;
        ctx.inline_depth = 1;
        let content = render_children(url, dom, id, ctx)?;
        ctx.inline_depth = old_inline_depth;

        if content.trim().is_empty() {
            return Ok(String::new());
        }
        Ok(format!("{}\n\n", content.trim()))
    }
}

pub static PARAGRAPH: Paragraph = Paragraph;
