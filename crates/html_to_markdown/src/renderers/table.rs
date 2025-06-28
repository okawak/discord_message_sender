use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct Table;

impl Renderer for Table {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            matches!(
                tag.local.as_ref(),
                "table" | "thead" | "tbody" | "tr" | "th" | "td"
            )
        } else {
            false
        }
    }

    fn render(&self, dom: &Dom, id: NodeId, ctx: &mut Context) -> Result<String, ConvertError> {
        ctx.in_table = true;
        let content = render_children(dom, id, ctx)?;
        ctx.in_table = false;

        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            match tag.local.as_ref() {
                "table" => Ok(format!("{content}\n\n")),
                "tr" => Ok(format!("| {content} |\n")),
                "th" | "td" => Ok(format!("{} | ", content.trim())),
                _ => Ok(content),
            }
        } else {
            Ok(content)
        }
    }
}

pub static TABLE: Table = Table;
