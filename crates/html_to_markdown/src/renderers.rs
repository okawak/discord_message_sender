pub mod code_block;
pub mod heading;
pub mod inline;
pub mod paragraph;

use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct Context; // 状態が必要になったら追加

pub trait Renderer {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool;
    fn render(&self, dom: &Dom, id: NodeId, ctx: &mut Context) -> Result<String, ConvertError>;
}

fn table() -> Vec<Box<dyn Renderer>> {
    vec![
        Box::new(heading::Heading),
        Box::new(paragraph::Paragraph),
        Box::new(inline::Inline),
        Box::new(code_block::CodeBlock),
    ]
}

pub fn render_node(dom: &Dom, id: NodeId, ctx: &mut Context) -> Result<String, ConvertError> {
    for r in table() {
        if r.matches(dom, id) {
            return r.render(dom, id, ctx);
        }
    }
    // fallback: 子を連結
    if let NodeData::Element { .. } = &dom.node(id).data {
        dom.node(id)
            .children
            .iter()
            .map(|&c| render_node(dom, c, ctx))
            .collect::<Result<Vec<_>, _>>()
            .map(|v| v.concat())
    } else if let NodeData::Text(t) = &dom.node(id).data {
        Ok(t.clone())
    } else {
        Ok(String::new())
    }
}
