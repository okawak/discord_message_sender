use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct Inline;

impl Renderer for Inline {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            matches!(
                tag.local.as_ref(),
                "strong" | "b" | "em" | "i" | "a" | "span" | "br"
            )
        } else {
            false
        }
    }

    fn render(&self, dom: &Dom, id: NodeId, ctx: &mut Context) -> Result<String, ConvertError> {
        if let NodeData::Element { tag, attrs, .. } = &dom.node(id).data {
            match tag.local.as_ref() {
                "strong" | "b" => {
                    let content = render_children(dom, id, ctx)?;
                    Ok(format!("**{content}**"))
                }
                "em" | "i" => {
                    let content = render_children(dom, id, ctx)?;
                    Ok(format!("*{content}*"))
                }
                "a" => {
                    let content = render_children(dom, id, ctx)?;
                    if let Some(href) = attrs.get("href") {
                        Ok(format!("[{content}]({href})"))
                    } else {
                        Ok(content)
                    }
                }
                "br" => Ok("\n".to_string()),
                _ => render_children(dom, id, ctx),
            }
        } else {
            render_children(dom, id, ctx)
        }
    }
}

pub static INLINE: Inline = Inline;
