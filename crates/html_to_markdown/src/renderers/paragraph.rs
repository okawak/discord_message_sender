use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
    utils::format_list_content,
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
        let content = render_children(url, dom, id, ctx)?;
        if content.trim().is_empty() {
            return Ok(String::new());
        }

        // Handle the first item in a list differently
        Ok(format_list_content(ctx, &content))
    }
}

pub static PARAGRAPH: Paragraph = Paragraph;
