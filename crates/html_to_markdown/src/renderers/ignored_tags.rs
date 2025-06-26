use super::{Context, Renderer};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct IgnoredTags;

impl Renderer for IgnoredTags {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            matches!(tag.local.as_ref(), "script" | "style" | "noscript")
        } else {
            false
        }
    }

    fn render(&self, _dom: &Dom, _id: NodeId, _ctx: &mut Context) -> Result<String, ConvertError> {
        Ok(String::new())
    }
}

pub static IGNORED_TAGS: IgnoredTags = IgnoredTags;
