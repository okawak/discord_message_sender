use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
    utils::filtering,
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
                "div" | "section" | "article" | "main" | "header"
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
        let (tag, attrs) = dom.get_element_data(id)?;

        if tag.local.as_ref() == "div" {
            // Check if class should be ignored
            if let Some(class_value) = attrs.get("class")
                && filtering::should_ignore_class(class_value)
            {
                return Ok(String::new());
            }

            // div elements are treated transparently - just render children without any formatting
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
