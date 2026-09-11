use super::{Context, Renderer, render_children};
use crate::{
    dom::{Dom, NodeData, NodeId},
    error::ConvertError,
};

pub struct Inline;

impl Inline {
    /// Updates the last output character in context
    fn update_last_char(&self, ctx: &mut Context, content: &str) {
        ctx.last_char = content.chars().last();
    }

    /// Keeps collapsible boundary whitespace outside Markdown emphasis markers.
    /// Whitespace inside the markers can prevent CommonMark parsers from
    /// recognizing the delimiter run.
    fn wrap_with_marker(&self, content: &str, marker: &str, preserve_whitespace: bool) -> String {
        if preserve_whitespace {
            return format!("{marker}{content}{marker}");
        }

        let trimmed = content.trim_matches(char::is_whitespace);
        if trimmed.is_empty() {
            return String::new();
        }

        let leading = content.starts_with(char::is_whitespace);
        let trailing = content.ends_with(char::is_whitespace);
        format!(
            "{}{}{}{}{}",
            if leading { " " } else { "" },
            marker,
            trimmed,
            marker,
            if trailing { " " } else { "" }
        )
    }
}

impl Renderer for Inline {
    fn matches(&self, dom: &Dom, id: NodeId) -> bool {
        let Some(node) = dom.node(id) else {
            return false;
        };

        if let NodeData::Element { tag, .. } = &node.data {
            matches!(
                tag.local.as_ref(),
                "strong" | "b" | "em" | "i" | "span" | "br"
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
        let (tag, _) = dom.get_element_data(id)?;
        let old_inline_status = ctx.in_inline;

        let result = match tag.local.as_ref() {
            "strong" | "b" => {
                ctx.in_inline = true;
                let content = render_children(url, dom, id, ctx)?;
                ctx.in_inline = old_inline_status;
                self.wrap_with_marker(&content, "**", ctx.preserve_whitespace)
            }
            "em" | "i" => {
                ctx.in_inline = true;
                let content = render_children(url, dom, id, ctx)?;
                ctx.in_inline = old_inline_status;
                self.wrap_with_marker(&content, "*", ctx.preserve_whitespace)
            }
            "br" => "<br>".to_string(),
            _ => {
                ctx.in_inline = true;
                let content = render_children(url, dom, id, ctx)?;
                ctx.in_inline = old_inline_status;
                content
            }
        };
        self.update_last_char(ctx, &result);
        Ok(result)
    }
}

pub static INLINE: Inline = Inline;
