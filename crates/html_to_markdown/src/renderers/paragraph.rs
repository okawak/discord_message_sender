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
        let old_paragraph_status = ctx.in_paragraph;
        ctx.in_paragraph = true;
        let rendered = render_children(url, dom, id, ctx);
        ctx.in_paragraph = old_paragraph_status;
        let content = rendered?;
        if content.trim().is_empty() {
            return Ok(String::new());
        }

        if ctx.in_link_label {
            return Ok(format!(" {} ", content.trim()));
        }

        // Handle the first item in a list differently
        Ok(format_list_content(ctx, &content))
    }
}

pub static PARAGRAPH: Paragraph = Paragraph;
