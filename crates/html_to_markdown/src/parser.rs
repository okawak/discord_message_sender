use crate::dom::{Dom, NodeData, NodeId};
use crate::error::ConvertError;
use html5ever::{
    ExpandedName,
    interface::{Attribute, QualName},
    tendril::{StrTendril, TendrilSink},
    tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink},
};
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::default::Default;

pub fn parse_html(html: &str) -> Result<Dom, ConvertError> {
    let sink = VecSink {
        dom: RefCell::new(Dom::new()),
        // Store leaked QualName references for efficient elem_name lookups
        element_names: RefCell::new(HashMap::new()),
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
    /// Cache of leaked QualName references for efficient elem_name implementation
    /// Following the approach demonstrated in html5ever's official examples
    element_names: RefCell<HashMap<NodeId, &'static QualName>>,
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
    type ElemName<'a> = ExpandedName<'a>;

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

    fn elem_name<'a>(&'a self, id: &NodeId) -> ExpandedName<'a> {
        self.element_names
            .borrow()
            .get(id)
            .unwrap_or_else(|| panic!("Node {id} is not an element"))
            .expanded()
    }

    /// Creates a new element node in the DOM.
    ///
    /// Note: This implementation intentionally leaks memory to minimize implementation
    /// complexity, following the approach demonstrated in html5ever's official examples.
    /// For Discord Message Sender's use case (short-lived parsing operations), this
    /// approach provides a good balance between implementation simplicity and performance.
    ///
    /// Reference: https://github.com/servo/html5ever/blob/main/examples/noop-tree-builder.rs
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

            // Intentionally leak the QualName to obtain a 'static reference
            // This approach is used in html5ever's official examples to handle
            // the lifetime constraints of the TreeSink trait
            let leaked_name: &'static QualName = Box::leak(Box::new(name.clone()));

            let id = dom.create_without_parent(NodeData::Element {
                tag: name,
                attrs: attrs_map,
            });

            // Store the leaked reference for efficient elem_name lookups
            self.element_names.borrow_mut().insert(id, leaked_name);

            id
        })
    }

    fn create_comment(&self, text: html5ever::tendril::StrTendril) -> NodeId {
        self.with_mut(|dom| dom.create(NodeData::Comment(text.to_string()), dom.document))
    }

    fn append(&self, parent: &NodeId, child: NodeOrText<NodeId>) {
        self.with_mut(|dom| match child {
            NodeOrText::AppendNode(id) => {
                dom.node_mut(id).parent.get_or_insert(*parent);
                dom.node_mut(*parent).children.push(id);
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

    #[fixture]
    fn valid_html() -> &'static str {
        "<div><p>Hello <strong>world</strong></p></div>"
    }

    #[fixture]
    fn invalid_html() -> &'static str {
        "<div><p>Unclosed tags"
    }

    #[rstest]
    fn test_parse_valid_html(valid_html: &str) {
        let result = parse_html(valid_html);
        assert_eq!(result.is_ok(), true);

        let dom = result.unwrap();
        assert_eq!(dom.node_count() > 0, true);
    }
}
