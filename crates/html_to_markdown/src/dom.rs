use crate::error::ConvertError;
use html5ever::QualName;
use std::collections::HashMap;

/// Represents a unique identifier for a node in the DOM tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(usize); // Use usize to represent node IDs

impl NodeId {
    pub(crate) fn new(id: usize) -> Self {
        NodeId(id)
    }
    pub(crate) fn as_usize(self) -> usize {
        self.0
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeId({})", self.0)
    }
}

/// Represents the data contained in a DOM node.
#[derive(Debug)]
pub enum NodeData {
    Document,
    Doctype {
        name: String,
        public_id: String,
        system_id: String,
    },
    Element {
        tag: QualName,
        attrs: HashMap<String, String>,
    },
    Text(String),
    Comment(String),
    ProcessingInstruction {
        target: String,
        data: String,
    },
}

/// Represents a node in the DOM tree.
#[derive(Debug)]
pub struct Node {
    pub data: NodeData,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

/// Custom DOM structure for HTML parsing and manipulation.
/// It use arena-based architecture to store nodes efficiently.
#[derive(Debug)]
pub struct Dom {
    pub arena: Vec<Node>, // all nodes are stored in a single arena
    pub document: NodeId, // root node of the document
}

impl Dom {
    pub fn new() -> Self {
        let arena = vec![Node {
            data: NodeData::Document,
            parent: None,
            children: Vec::new(),
        }];
        Self {
            arena,
            document: NodeId::new(0),
        }
    }

    pub fn get_node(&self, id: NodeId) -> Result<&Node, ConvertError> {
        self.node(id)
            .ok_or_else(|| ConvertError::InvalidNode(format!("Node {id} not found")))
    }

    pub fn get_element_data(
        &self,
        id: NodeId,
    ) -> Result<(&QualName, &HashMap<String, String>), ConvertError> {
        match &self.get_node(id)?.data {
            NodeData::Element { tag, attrs, .. } => Ok((tag, attrs)),
            _ => Err(ConvertError::Unsupported(format!(
                "Node {id} is not an element"
            ))),
        }
    }

    pub fn iter_children(&self, id: NodeId) -> Result<std::slice::Iter<'_, NodeId>, ConvertError> {
        Ok(self.get_node(id)?.children.iter())
    }

    pub fn node_exists(&self, id: NodeId) -> bool {
        self.node(id).is_some()
    }

    pub fn get_parent(&self, id: NodeId) -> Result<Option<NodeId>, ConvertError> {
        Ok(self.get_node(id)?.parent)
    }

    pub fn create(&mut self, data: NodeData, parent: NodeId) -> NodeId {
        let id = self.create_without_parent(data);
        self.arena[id.as_usize()].parent = Some(parent);
        self.arena[parent.as_usize()].children.push(id);
        id
    }

    /// Creates a node without setting parent-child relationships
    pub fn create_without_parent(&mut self, data: NodeData) -> NodeId {
        let id = NodeId::new(self.arena.len());
        self.arena.push(Node {
            data,
            parent: None,
            children: Vec::new(),
        });
        id
    }

    pub(crate) fn append_node(&mut self, parent: NodeId, child: NodeId) {
        assert!(
            self.node(parent).is_some(),
            "parent {parent} does not exist"
        );
        assert!(self.node(child).is_some(), "child {child} does not exist");
        assert_ne!(parent, child, "a node cannot be its own parent");
        self.remove_from_parent(child);
        self.arena[child.as_usize()].parent = Some(parent);
        self.arena[parent.as_usize()].children.push(child);
    }

    pub(crate) fn append_text(&mut self, parent: NodeId, text: &str) {
        let last_child = self.arena[parent.as_usize()].children.last().copied();
        if let Some(last_child) = last_child
            && let NodeData::Text(existing) = &mut self.arena[last_child.as_usize()].data
        {
            existing.push_str(text);
            return;
        }
        self.create(NodeData::Text(text.to_owned()), parent);
    }

    pub(crate) fn insert_node_before(&mut self, sibling: NodeId, child: NodeId) {
        let parent = self.arena[sibling.as_usize()]
            .parent
            .unwrap_or_else(|| panic!("sibling {sibling} has no parent"));
        self.remove_from_parent(child);
        let index = self.arena[parent.as_usize()]
            .children
            .iter()
            .position(|candidate| *candidate == sibling)
            .unwrap_or_else(|| panic!("sibling {sibling} is missing from parent {parent}"));
        self.arena[child.as_usize()].parent = Some(parent);
        self.arena[parent.as_usize()].children.insert(index, child);
    }

    pub(crate) fn insert_text_before(&mut self, sibling: NodeId, text: &str) {
        let parent = self.arena[sibling.as_usize()]
            .parent
            .unwrap_or_else(|| panic!("sibling {sibling} has no parent"));
        let index = self.arena[parent.as_usize()]
            .children
            .iter()
            .position(|candidate| *candidate == sibling)
            .unwrap_or_else(|| panic!("sibling {sibling} is missing from parent {parent}"));
        if index > 0 {
            let previous = self.arena[parent.as_usize()].children[index - 1];
            if let NodeData::Text(existing) = &mut self.arena[previous.as_usize()].data {
                existing.push_str(text);
                return;
            }
        }
        let child = self.create_without_parent(NodeData::Text(text.to_owned()));
        self.insert_node_before(sibling, child);
    }

    pub(crate) fn remove_from_parent(&mut self, target: NodeId) {
        let Some(parent) = self.arena[target.as_usize()].parent else {
            return;
        };
        let children = &mut self.arena[parent.as_usize()].children;
        let index = children
            .iter()
            .position(|candidate| *candidate == target)
            .unwrap_or_else(|| panic!("child {target} is missing from parent {parent}"));
        children.remove(index);
        self.arena[target.as_usize()].parent = None;
    }

    pub(crate) fn reparent_children(&mut self, node: NodeId, new_parent: NodeId) {
        assert_ne!(
            node, new_parent,
            "a node cannot reparent children to itself"
        );
        let children = std::mem::take(&mut self.arena[node.as_usize()].children);
        for child in &children {
            self.arena[child.as_usize()].parent = Some(new_parent);
        }
        self.arena[new_parent.as_usize()].children.extend(children);
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.arena.get(id.as_usize())
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.arena.get_mut(id.as_usize())
    }

    pub fn node_count(&self) -> usize {
        self.arena.len()
    }

    pub fn find_element_by_tag(&self, start_id: NodeId, tag_name: &str) -> Option<NodeId> {
        let node = self.node(start_id)?;

        if let NodeData::Element { tag, .. } = &node.data
            && tag.local.as_ref() == tag_name
        {
            return Some(start_id);
        }

        for &child_id in &node.children {
            if let Some(found) = self.find_element_by_tag(child_id, tag_name) {
                return Some(found);
            }
        }
        None
    }

    pub fn find_all_elements_by_tag(&self, start_id: NodeId, tag_name: &str) -> Vec<NodeId> {
        let mut results = Vec::new();
        self.find_all_elements_by_tag_recursive(start_id, tag_name, &mut results);
        results
    }

    fn find_all_elements_by_tag_recursive(
        &self,
        node_id: NodeId,
        tag_name: &str,
        results: &mut Vec<NodeId>,
    ) {
        let Some(node) = self.node(node_id) else {
            return;
        };

        if let NodeData::Element { tag, .. } = &node.data
            && tag.local.as_ref() == tag_name
        {
            results.push(node_id);
        }

        for &child_id in &node.children {
            self.find_all_elements_by_tag_recursive(child_id, tag_name, results);
        }
    }

    pub fn collect_text_content(&self, node_id: NodeId) -> String {
        let Some(node) = self.node(node_id) else {
            return String::new();
        };

        match &node.data {
            NodeData::Text(content) => content.clone(),
            NodeData::Element { .. } => {
                let mut text = String::new();
                for &child_id in &node.children {
                    text.push_str(&self.collect_text_content(child_id));
                }
                text
            }
            _ => String::new(),
        }
    }

    pub fn find_elements_with_attribute(
        &self,
        start_id: NodeId,
        attr_name: &str,
        attr_value: Option<&str>,
    ) -> Vec<NodeId> {
        let mut results = Vec::new();
        self.find_elements_with_attribute_recursive(start_id, attr_name, attr_value, &mut results);
        results
    }

    fn find_elements_with_attribute_recursive(
        &self,
        node_id: NodeId,
        attr_name: &str,
        attr_value: Option<&str>,
        results: &mut Vec<NodeId>,
    ) {
        let Some(node) = self.node(node_id) else {
            return;
        };

        if let NodeData::Element { attrs, .. } = &node.data
            && let Some(value) = attrs.get(attr_name)
        {
            match attr_value {
                Some(expected) => {
                    if value == expected {
                        results.push(node_id);
                    }
                }
                None => {
                    results.push(node_id);
                }
            }
        }

        for &child_id in &node.children {
            self.find_elements_with_attribute_recursive(child_id, attr_name, attr_value, results);
        }
    }

    pub fn find_head(&self) -> Option<NodeId> {
        self.find_element_by_tag(self.document, "head")
    }

    pub fn find_body(&self) -> Option<NodeId> {
        self.find_element_by_tag(self.document, "body")
    }

    pub fn find_article(&self) -> Option<NodeId> {
        self.find_element_by_tag(self.document, "article")
    }

    pub fn find_all_meta(&self) -> Vec<NodeId> {
        self.find_all_elements_by_tag(self.document, "meta")
    }
}

impl Default for Dom {
    fn default() -> Self {
        Self::new()
    }
}
