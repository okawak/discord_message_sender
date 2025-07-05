use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct Table;

impl Renderer for Table {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        let Some(node) = dom.node(id) else {
            return false;
        };

        if let NodeData::Element { tag, .. } = &node.data {
            matches!(
                tag.local.as_ref(),
                "table" | "thead" | "tbody" | "tr" | "th" | "td"
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
        ctx.in_table = true;
        let content = render_children(url, dom, id, ctx)?;
        ctx.in_table = false;

        let (tag, _) = dom.get_element_data(id)?;

        match tag.local.as_ref() {
            "table" => Ok(format!("{content}\n\n")),
            "tr" => Ok(format!("| {content} |\n")),
            "th" | "td" => Ok(format!("{} | ", content.trim())),
            _ => Ok(content),
        }
    }
}

pub static TABLE: Table = Table;
