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

    pub const INVALID: Self = Self(usize::MAX);

    pub fn is_valid(self) -> bool {
        self != Self::INVALID
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
    Element {
        tag: QualName,
        attrs: HashMap<String, String>,
    },
    Text(String),
    Comment(String),
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

    pub fn create(&mut self, data: NodeData, parent: NodeId) -> NodeId {
        let id = NodeId::new(self.arena.len());
        self.arena.push(Node {
            data,
            parent: Some(parent),
            children: Vec::new(),
        });
        self.arena[parent.as_usize()].children.push(id);
        id
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.arena[id.as_usize()]
    }

    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.arena[id.as_usize()]
    }

    pub fn node_count(&self) -> usize {
        self.arena.len()
    }
}

impl Default for Dom {
    fn default() -> Self {
        Self::new()
    }
}
