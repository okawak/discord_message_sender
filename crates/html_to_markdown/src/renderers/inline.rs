use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct Inline;

impl Inline {
    /// Updates the last output character in context
    fn update_last_char(&self, ctx: &mut Context, content: &str) {
        ctx.last_char = content.chars().last();
    }
}

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
        let old_inline_status = ctx.in_inline;

        let result = match tag.local.as_ref() {
            "strong" | "b" => {
                ctx.in_inline = true;
                let content = render_children(url, dom, id, ctx)?;
                ctx.in_inline = old_inline_status;
                format!("**{content}**")
            }
            "em" | "i" => {
                ctx.in_inline = true;
                let content = render_children(url, dom, id, ctx)?;
                ctx.in_inline = old_inline_status;
                format!("*{content}*")
            }
            "br" => "<br>".to_string(),
            _ => {
                ctx.in_inline = true;
                let content = render_children(url, dom, id, ctx)?;
                ctx.in_inline = old_inline_status;
                content
            }
        };
        self.update_last_char(ctx, &result);
        Ok(result)
    }
}

pub static INLINE: Inline = Inline;
