use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct List;

impl Renderer for List {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            matches!(tag.local.as_ref(), "ul" | "ol" | "li")
        } else {
            false
        }
    }

    fn render(&self, dom: &Dom, id: NodeId, ctx: &mut Context) -> Result<String, ConvertError> {
        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            match tag.local.as_ref() {
                "ul" | "ol" => {
                    ctx.list_depth += 1;
                    let content = render_children(dom, id, ctx)?;
                    ctx.list_depth -= 1;
                    Ok(format!("{content}\n"))
                }
                "li" => {
                    let indent = "  ".repeat(ctx.list_depth.saturating_sub(1));
                    let content = render_children(dom, id, ctx)?;
                    let marker = if self.is_ordered_list(dom, id) {
                        "1."
                    } else {
                        "-"
                    };
                    Ok(format!("{}{} {}\n", indent, marker, content.trim()))
                }
                _ => render_children(dom, id, ctx),
            }
        } else {
            render_children(dom, id, ctx)
        }
    }
}

impl List {
    fn is_ordered_list(&self, dom: &Dom, id: NodeId) -> bool {
        // 親要素を辿ってolタグがあるかチェック
        if let Some(parent_id) = dom.node(id).parent {
            if let NodeData::Element { tag, .. } = &dom.node(parent_id).data {
                return tag.local.as_ref() == "ol";
            }
        }
        false
    }
}

pub static LIST: List = List;
