use crate::dom::{Dom, NodeData, NodeId};
use crate::error::ConvertError;
use html5ever::{
    interface::{Attribute, QualName},
    tendril::{StrTendril, TendrilSink},
    tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink},
};
use std::borrow::Cow;
use std::cell::{Ref, RefCell};
use std::collections::{HashMap, HashSet};
use std::default::Default;

pub fn parse_html(html: &str) -> Result<Dom, ConvertError> {
    let sink = VecSink {
        dom: RefCell::new(Dom::new()),
        template_contents: RefCell::new(HashMap::new()),
        mathml_integration_points: RefCell::new(HashSet::new()),
    };
    // The caller already provides valid UTF-8; feed it directly to the HTML parser.
    let sink = html5ever::parse_document(sink, Default::default()).one(html);
    Ok(RefCell::into_inner(sink.dom)) // RefCell<Dom> -> Dom
}

struct VecSink {
    /// RefCell wrapper for DOM manipulation during parsing
    dom: RefCell<Dom>,
    template_contents: RefCell<HashMap<NodeId, NodeId>>,
    mathml_integration_points: RefCell<HashSet<NodeId>>,
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

    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> NodeId {
        let id = self.with_mut(|dom| {
            let attrs_map = attrs
                .into_iter()
                .map(|a| (a.name.local.to_string(), a.value.to_string()))
                .collect();

            dom.create_without_parent(NodeData::Element {
                tag: name,
                attrs: attrs_map,
            })
        });
        if flags.template {
            let contents = self.with_mut(|dom| dom.create_without_parent(NodeData::Document));
            self.template_contents.borrow_mut().insert(id, contents);
        }
        if flags.mathml_annotation_xml_integration_point {
            self.mathml_integration_points.borrow_mut().insert(id);
        }
        id
    }

    fn create_comment(&self, text: html5ever::tendril::StrTendril) -> NodeId {
        self.with_mut(|dom| dom.create_without_parent(NodeData::Comment(text.to_string())))
    }

    fn append(&self, parent: &NodeId, child: NodeOrText<NodeId>) {
        self.with_mut(|dom| match child {
            NodeOrText::AppendNode(id) => dom.append_node(*parent, id),
            NodeOrText::AppendText(text) => dom.append_text(*parent, text.as_ref()),
        });
    }

    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        self.with_mut(|dom| {
            let doctype = dom.create_without_parent(NodeData::Doctype {
                name: name.to_string(),
                public_id: public_id.to_string(),
                system_id: system_id.to_string(),
            });
            dom.append_node(dom.document, doctype);
        });
    }

    fn append_based_on_parent_node(
        &self,
        element: &NodeId,
        prev_element: &NodeId,
        child: NodeOrText<NodeId>,
    ) {
        if self
            .dom
            .borrow()
            .get_parent(*element)
            .ok()
            .flatten()
            .is_some()
        {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }

    fn append_before_sibling(&self, sibling: &NodeId, child: NodeOrText<NodeId>) {
        self.with_mut(|dom| match child {
            NodeOrText::AppendNode(id) => dom.insert_node_before(*sibling, id),
            NodeOrText::AppendText(text) => dom.insert_text_before(*sibling, text.as_ref()),
        });
    }

    fn add_attrs_if_missing(&self, target: &NodeId, attrs: Vec<Attribute>) {
        self.with_mut(|dom| {
            let node = dom
                .node_mut(*target)
                .unwrap_or_else(|| panic!("Node {target} does not exist"));
            let NodeData::Element {
                attrs: existing, ..
            } = &mut node.data
            else {
                panic!("Node {target} is not an element");
            };
            for attr in attrs {
                existing
                    .entry(attr.name.local.to_string())
                    .or_insert_with(|| attr.value.to_string());
            }
        });
    }

    fn remove_from_parent(&self, target: &NodeId) {
        self.with_mut(|dom| dom.remove_from_parent(*target));
    }

    fn reparent_children(&self, node: &NodeId, new_parent: &NodeId) {
        self.with_mut(|dom| dom.reparent_children(*node, *new_parent));
    }

    fn mark_script_already_started(&self, _n: &NodeId) {}

    fn get_template_contents(&self, target: &NodeId) -> NodeId {
        *self
            .template_contents
            .borrow()
            .get(target)
            .unwrap_or_else(|| panic!("Node {target} is not a template element"))
    }

    fn create_pi(&self, target: StrTendril, data: StrTendril) -> NodeId {
        self.with_mut(|dom| {
            dom.create_without_parent(NodeData::ProcessingInstruction {
                target: target.to_string(),
                data: data.to_string(),
            })
        })
    }

    fn is_mathml_annotation_xml_integration_point(&self, target: &NodeId) -> bool {
        self.mathml_integration_points.borrow().contains(target)
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

    #[test]
    fn test_preserves_text_foster_parented_out_of_a_table() {
        let html = "<table>KEEP ME<tr><td>cell</td></tr></table>";
        let dom = parse_html(html).unwrap();
        let body = dom.find_body().expect("body should exist");

        assert_eq!(dom.collect_text_content(body), "KEEP MEcell");
        assert!(
            crate::convert("https://example.com", html, &[])
                .unwrap()
                .contains("KEEP ME")
        );
    }

    #[test]
    fn test_keeps_template_contents_out_of_the_document_tree() {
        let dom = parse_html("<template><p>hidden</p></template><p>shown</p>").unwrap();
        let body = dom.find_body().expect("body should exist");

        assert_eq!(dom.collect_text_content(body), "shown");
    }

    #[test]
    fn test_adds_missing_attributes_without_overwriting_existing_ones() {
        let dom = parse_html("<html lang=first><html lang=second dir=rtl>").unwrap();
        let html = dom
            .find_element_by_tag(dom.document, "html")
            .expect("html should exist");
        let (_, attrs) = dom.get_element_data(html).unwrap();

        assert_eq!(attrs.get("lang").map(String::as_str), Some("first"));
        assert_eq!(attrs.get("dir").map(String::as_str), Some("rtl"));
    }
}
