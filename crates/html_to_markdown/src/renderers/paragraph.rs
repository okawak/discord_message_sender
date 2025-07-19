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
        let is_first_in_list = ctx.list_depth > 0 && ctx.list_first_item;
        let indent = " ".repeat(ctx.list_depth);
        let content = render_children(url, dom, id, ctx)?;
        if content.trim().is_empty() {
            return Ok(String::new());
        }

        // Handle the first item in a list differently
        if is_first_in_list {
            Ok(content)
        } else if ctx.list_depth > 0 {
            Ok(format!("\n\n{indent}{content}"))
        } else {
            Ok(format!("{content}\n\n"))
        }
    }
}

pub static PARAGRAPH: Paragraph = Paragraph;
