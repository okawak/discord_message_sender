use crate::dom::{Dom, Node, NodeData, NodeId};
use crate::error::ConvertError;
use html5ever::{
    ExpandedName,
    interface::{Attribute, QualName},
    tendril::{StrTendril, TendrilSink},
    tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink},
};
use std::cell::RefCell;
use std::default::Default;

pub fn parse_html(html: &str) -> Result<Dom, ConvertError> {
    let sink = VecSink {
        dom: RefCell::new(Dom::new()),
    };
    let sink = html5ever::parse_document(sink, Default::default())
        .from_utf8()
        .read_from(&mut html.as_bytes())
        .map_err(|e| ConvertError::Parse(e.to_string()))?;
    Ok(RefCell::into_inner(sink.dom))
}

struct VecSink {
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
    type ElemName<'a> = ExpandedName<'static>;

    fn finish(self) -> Self {
        self
    }
    fn get_document(&self) -> NodeId {
        self.dom.borrow().document
    }
    fn parse_error(&self, _msg: std::borrow::Cow<'static, str>) {}
    fn set_quirks_mode(&self, _m: QuirksMode) {}

    fn elem_name<'a>(&'a self, _h: &NodeId) -> Self::ElemName<'a> {
        unimplemented!("elem_name is not implemented for VecSink");
        // let dom = self.dom.borrow();
        // match &dom.node(*h).data {
        // NodeData::Element { tag, .. } => ExpandedName {
        // ns: Box::leak(Box::new(tag.ns.clone())),
        // local: Box::leak(Box::new(tag.local.clone())),
        // },
        // _ => {
        // static EMPTY: ExpandedName<'static> = ExpandedName {
        // ns: &ns!(),
        // local: &local_name!(""),
        // };
        // EMPTY
        // }
        // }
    }

    fn create_element(
        &self,
        name: QualName,
        attrs: Vec<Attribute>,
        _flags: ElementFlags,
    ) -> NodeId {
        self.with_mut(|dom| {
            let map = attrs
                .into_iter()
                .map(|a| (a.name.local.to_string(), a.value.to_string()))
                .collect();

            let node = Node {
                data: NodeData::Element {
                    tag: name.clone(),
                    attrs: map,
                },
                parent: None,
                children: Vec::new(),
            };

            let id = dom.arena.len();
            dom.arena.push(node);
            id
        })
    }

    fn create_comment(&self, text: html5ever::tendril::StrTendril) -> NodeId {
        self.with_mut(|d| d.create(NodeData::Comment(text.to_string()), d.document))
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
    fn same_node(&self, a: &NodeId, b: &NodeId) -> bool {
        a == b
    }
}
