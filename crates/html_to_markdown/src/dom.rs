use html5ever::interface::QualName;
use std::collections::HashMap;

pub type NodeId = usize;

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

#[derive(Debug)]
pub struct Node {
    pub data: NodeData,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

#[derive(Debug)]
pub struct Dom {
    pub arena: Vec<Node>,
    pub document: NodeId,
}

impl Dom {
    pub fn new() -> Self {
        let arena = vec![Node {
            data: NodeData::Document,
            parent: None,
            children: Vec::new(),
        }];
        Self { arena, document: 0 }
    }

    pub fn create(&mut self, data: NodeData, parent: NodeId) -> NodeId {
        let id = self.arena.len();
        self.arena.push(Node {
            data,
            parent: Some(parent),
            children: Vec::new(),
        });
        self.arena[parent].children.push(id);
        id
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.arena[id]
    }

    pub fn node_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.arena[id]
    }
}

impl Default for Dom {
    fn default() -> Self {
        Self::new()
    }
}
