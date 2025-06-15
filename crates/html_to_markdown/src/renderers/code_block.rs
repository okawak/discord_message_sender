use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct CodeBlock;

impl Renderer for CodeBlock {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            matches!(tag.local.as_ref(), "pre" | "code")
        } else {
            false
        }
    }

    fn render(&self, dom: &Dom, id: NodeId, ctx: &mut Context) -> Result<String, ConvertError> {
        if let NodeData::Element { tag, .. } = &dom.node(id).data {
            match tag.local.as_ref() {
                "pre" => {
                    // preタグ内では空白を保持
                    let old_preserve = ctx.preserve_whitespace;
                    ctx.preserve_whitespace = true;
                    let content = render_children(dom, id, ctx)?;
                    ctx.preserve_whitespace = old_preserve;
                    Ok(format!("```\n{}\n```\n\n", content))
                }
                "code" => {
                    // インラインcodeの場合
                    let old_preserve = ctx.preserve_whitespace;
                    ctx.preserve_whitespace = true;
                    let content = render_children(dom, id, ctx)?;
                    ctx.preserve_whitespace = old_preserve;
                    Ok(format!("`{}`", content))
                }
                _ => render_children(dom, id, ctx),
            }
        } else {
            render_children(dom, id, ctx)
        }
    }
}

pub static CODE_BLOCK: CodeBlock = CodeBlock;
