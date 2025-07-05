use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct List;

impl List {
    fn is_ordered_list(&self, dom: &Dom, id: NodeId) -> bool {
        let Ok(Some(parent_id)) = dom.get_parent(id) else {
            return false;
        };

        if let Ok((parent_tag, _)) = dom.get_element_data(parent_id) {
            parent_tag.local.as_ref() == "ol"
        } else {
            false
        }
    }
}

impl Renderer for List {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        let Some(node) = dom.node(id) else {
            return false;
        };

        if let NodeData::Element { tag, .. } = &node.data {
            matches!(tag.local.as_ref(), "ul" | "ol" | "li")
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
            "ul" | "ol" => {
                ctx.list_depth += 1;
                let content = render_children(url, dom, id, ctx)?;
                ctx.list_depth -= 1;
                Ok(format!("{content}\n"))
            }
            "li" => {
                let indent = "  ".repeat(ctx.list_depth.saturating_sub(1));
                let content = render_children(url, dom, id, ctx)?;
                let marker = if self.is_ordered_list(dom, id) {
                    "1."
                } else {
                    "-"
                };
                Ok(format!("{}{} {}\n", indent, marker, content.trim()))
            }
            _ => render_children(url, dom, id, ctx),
        }
    }
}

pub static LIST: List = List;
