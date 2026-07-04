use crate::dom::{Dom, NodeData, NodeId};
use crate::error::ConvertError;
use html5ever::{
    interface::{Attribute, QualName},
    tendril::{StrTendril, TendrilSink},
    tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink},
};
use std::borrow::Cow;
use std::cell::{Ref, RefCell};
use std::default::Default;

pub fn parse_html(html: &str) -> Result<Dom, ConvertError> {
    let sink = VecSink {
        dom: RefCell::new(Dom::new()),
    };
    let sink = html5ever::parse_document(sink, Default::default())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .map_err(|e| ConvertError::Parse(e.to_string()))?;
    Ok(RefCell::into_inner(sink.dom)) // RefCell<Dom> -> Dom
}

struct VecSink {
    /// RefCell wrapper for DOM manipulation during parsing
    dom: RefCell<Dom>,
}

impl VecSink {
    fn with_mut<R>(&self, f: impl FnOnce(&mut Dom) -> R) -> R {
        let mut dom = self.dom.borrow_mut();
        f(&mut dom)
    }
}

impl TreeSink for VecSink {
    type Handle = NodeId;
    type Output = Self;
    type ElemName<'a> = Ref<'a, QualName>;

    fn finish(self) -> Self {
        self
    }

    fn parse_error(&self, _: Cow<'static, str>) {}

    fn get_document(&self) -> NodeId {
        self.dom.borrow().document
    }

    fn set_quirks_mode(&self, _: QuirksMode) {}

    fn same_node(&self, a: &NodeId, b: &NodeId) -> bool {
        a == b
    }

    fn elem_name<'a>(&'a self, id: &NodeId) -> Self::ElemName<'a> {
        Ref::map(self.dom.borrow(), |dom| {
            let node = dom
                .node(*id)
                .unwrap_or_else(|| panic!("Node {id} does not exist"));
            match &node.data {
                NodeData::Element { tag, .. } => tag,
                _ => panic!("Node {id} is not an element"),
            }
        })
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        _flags: ElementFlags,
    ) -> NodeId {
        self.with_mut(|dom| {
            let attrs_map = attrs
                .into_iter()
                .map(|a| (a.name.local.to_string(), a.value.to_string()))
                .collect();

            dom.create_without_parent(NodeData::Element {
                tag: name,
                attrs: attrs_map,
            })
        })
    }

    fn create_comment(&self, text: html5ever::tendril::StrTendril) -> NodeId {
        self.with_mut(|dom| dom.create_without_parent(NodeData::Comment(text.to_string())))
    }

    fn append(&self, parent: &NodeId, child: NodeOrText<NodeId>) {
        self.with_mut(|dom| match child {
            NodeOrText::AppendNode(id) => {
                if let Some(node) = dom.node_mut(id) {
                    node.parent.get_or_insert(*parent);
                }
                if let Some(parent_node) = dom.node_mut(*parent) {
                    parent_node.children.push(id);
                }
            }
            NodeOrText::AppendText(t) => {
                dom.create(NodeData::Text(t.to_string()), *parent);
            }
        });
    }

    fn append_doctype_to_document(&self, _n: StrTendril, _p: StrTendril, _s: StrTendril) {}
    fn append_based_on_parent_node(&self, _e: &NodeId, _p: &NodeId, _c: NodeOrText<NodeId>) {}
    fn append_before_sibling(&self, _s: &NodeId, _c: NodeOrText<NodeId>) {}
    fn add_attrs_if_missing(&self, _t: &NodeId, _a: Vec<Attribute>) {}
    fn remove_from_parent(&self, _t: &NodeId) {}
    fn reparent_children(&self, _n: &NodeId, _np: &NodeId) {}
    fn mark_script_already_started(&self, _n: &NodeId) {}
    fn get_template_contents(&self, _t: &NodeId) -> NodeId {
        self.dom.borrow().document
    }
    fn create_pi(&self, _t: StrTendril, _d: StrTendril) -> NodeId {
        self.get_document()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::*;

    #[rstest] // document, html, head, body
    #[case("<div><p>Hello <strong>world</strong></p></div>", 9)] // div, p, "Hello " strong, "world"
    #[case("<div><p>Unclosed tags", 7)] // div, p, "Unclosed tags" (auto-closing)
    #[case("", 4)]
    #[case(
        "<div class=\"container\" id=\"main\"><a href=\"https://example.com\">Link</a></div>",
        7
    )] // div, a, "Link"
    #[case("<div><!-- This is a comment --><p>Content</p></div>", 8)] // div, comment, p, "Content"
    #[case("<div><img src=\"test.jpg\" alt=\"test\"><br><hr></div>", 8)] // div, img, br, hr
    #[case("<p>Simple text</p>", 6)] // p, "Simple text"
    #[case("<div><span>Nested</span></div>", 7)] // div, span, "Nested"
    #[case("<h1>Header</h1><p>Paragraph</p>", 8)] // h1, "Header", p, "Paragraph"
    fn test_parse_html_success(#[case] html: &str, #[case] expected_nodes: usize) {
        let result = parse_html(html);
        assert_eq!(result.is_ok(), true);

        let dom = result.unwrap();
        assert_eq!(dom.node_count(), expected_nodes);
    }

    #[rstest] // document
    #[case(
        "<html><head><title>Test</title></head><body><div><ul><li>Item 1</li><li>Item 2</li></ul></div></body></html>",
        12 // html, head, title, "Test", body, div, ul, li, "Item 1", li, "Item 2"
    )]
    fn test_parse_html_complex_structure(#[case] html: &str, #[case] expected_nodes: usize) {
        let result = parse_html(html);
        assert_eq!(result.is_ok(), true);

        let dom = result.unwrap();
        assert_eq!(dom.node_count(), expected_nodes,);
    }
}
