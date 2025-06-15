use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct Heading;

impl Renderer for Heading {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            matches!(tag.local.as_ref(), "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
        } else {
            false
        }
    }

    fn render(&self, dom: &Dom, id: NodeId, ctx: &mut Context) -> Result<String, ConvertError> {
        let content = render_children(dom, id, ctx)?;
        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            let level = match tag.local.as_ref() {
                "h1" => "#",
                "h2" => "##",
                "h3" => "###",
                "h4" => "####",
                "h5" => "#####",
                "h6" => "######",
                _ => "#",
            };
            Ok(format!("{} {}\n\n", level, content.trim()))
        } else {
            Ok(content)
        }
    }
}

pub static HEADING: Heading = Heading;
