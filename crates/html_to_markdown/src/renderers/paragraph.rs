use super::{Context, Renderer};
use crate::{
    dom::{Dom, NodeId},
    error::ConvertError,
};

pub struct Paragraph;
impl Renderer for Paragraph {
    fn matches(&self, _dom: &Dom, _id: NodeId) -> bool {
        false
    }
    fn render(&self, _dom: &Dom, _id: NodeId, _ctx: &mut Context) -> Result<String, ConvertError> {
        unreachable!()
    }
}
