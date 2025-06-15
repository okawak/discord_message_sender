use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct GenericBlock;

impl Renderer for GenericBlock {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            matches!(
                tag.local.as_ref(),
                "div" | "section" | "article" | "aside" | "nav" | "main" | "header" | "footer"
            )
        } else {
            false
        }
    }

    fn render(&self, dom: &Dom, id: NodeId, ctx: &mut Context) -> Result<String, ConvertError> {
        let content = render_children(dom, id, ctx)?;
        if content.trim().is_empty() {
            Ok(String::new())
        } else {
            Ok(format!("{}\n\n", content.trim()))
        }
    }
}

pub static BLOCK: GenericBlock = GenericBlock;
