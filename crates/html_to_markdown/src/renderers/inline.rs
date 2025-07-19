use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct Inline;

impl Renderer for Inline {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        let Some(node) = dom.node(id) else {
            return false;
        };

        if let NodeData::Element { tag, .. } = &node.data {
            matches!(
                tag.local.as_ref(),
                "strong" | "b" | "em" | "i" | "span" | "br"
            )
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
            "strong" | "b" => {
                ctx.inline_depth += 1;
                let content = render_children(url, dom, id, ctx)?;
                ctx.inline_depth -= 1;
                Ok(format!("**{content}**"))
            }
            "em" | "i" => {
                ctx.inline_depth += 1;
                let content = render_children(url, dom, id, ctx)?;
                ctx.inline_depth -= 1;
                Ok(format!("*{content}*"))
            }
            "br" => Ok("<br>".to_string()),
            _ => {
                ctx.inline_depth += 1;
                let content = render_children(url, dom, id, ctx)?;
                ctx.inline_depth -= 1;
                Ok(content)
            }
        }
    }
}

pub static INLINE: Inline = Inline;
