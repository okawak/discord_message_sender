use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct GenericBlock;

impl Renderer for GenericBlock {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        let Some(node) = dom.node(id) else {
            return false;
        };

        if let NodeData::Element { tag, .. } = &node.data {
            matches!(
                tag.local.as_ref(),
                "div" | "section" | "article" | "aside" | "main" | "header"
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

        // div elements are treated transparently - just render children without any formatting
        if tag.local.as_ref() == "div" {
            return render_children(url, dom, id, ctx);
        }

        let indent = " ".repeat(ctx.list_depth);
        let content = render_children(url, dom, id, ctx)?;
        if content.trim().is_empty() {
            Ok(String::new())
        } else {
            Ok(format!("{indent}{}\n\n", content.trim()))
        }
    }
}

pub static BLOCK: GenericBlock = GenericBlock;
