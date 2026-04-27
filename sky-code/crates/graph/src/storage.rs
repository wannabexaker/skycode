//! Graph storage with petgraph + JSON serialization

use crate::{Edge, EdgeType, Node, NodeType};
use anyhow::Result;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// In-memory directed graph of code entities
pub struct GraphStorage {
    pub(crate) graph: DiGraph<Node, Edge>,
    pub(crate) node_index: HashMap<String, NodeIndex>,
}

/// Serializable snapshot of the graph (what is written to graph.json)
#[derive(Serialize, Deserialize)]
struct GraphSnapshot {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl GraphStorage {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: Node) {
        if !self.node_index.contains_key(&node.id) {
            let idx = self.graph.add_node(node.clone());
            self.node_index.insert(node.id, idx);
        }
    }

    pub fn add_edge(&mut self, edge: Edge) {
        // Ensure both endpoints exist (create stub nodes if needed)
        let src_idx = self.get_or_create_stub(&edge.source);
        let tgt_idx = self.get_or_create_stub(&edge.target);
        self.graph.add_edge(src_idx, tgt_idx, edge);
    }

    fn get_or_create_stub(&mut self, id: &str) -> NodeIndex {
        if let Some(&idx) = self.node_index.get(id) {
            return idx;
        }
        let stub = Node {
            id: id.to_string(),
            node_type: NodeType::Module,
            label: id.to_string(),
            file_path: String::new(),
            line: 0,
            docstring: None,
        };
        let idx = self.graph.add_node(stub);
        self.node_index.insert(id.to_string(), idx);
        idx
    }

    pub fn node_count(&self) -> usize { self.graph.node_count() }
    pub fn edge_count(&self) -> usize { self.graph.edge_count() }

    /// Clone all nodes and edges for downstream rendering/export operations.
    pub fn snapshot_data(&self) -> (Vec<Node>, Vec<Edge>) {
        let nodes: Vec<Node> = self.graph.node_weights().cloned().collect();
        let edges: Vec<Edge> = self.graph.edge_weights().cloned().collect();
        (nodes, edges)
    }

    /// Save the graph to JSON
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let (nodes, edges) = self.snapshot_data();
        let snapshot = GraphSnapshot { nodes, edges };
        let json = serde_json::to_string_pretty(&snapshot)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load a graph from JSON
    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let snapshot: GraphSnapshot = serde_json::from_str(&json)?;
        let mut storage = Self::new();
        for node in snapshot.nodes { storage.add_node(node); }
        for edge in snapshot.edges { storage.add_edge(edge); }
        Ok(storage)
    }

    /// Return top N nodes by degree (outgoing + incoming)
    pub fn god_nodes(&self, top_n: usize) -> Vec<(&Node, usize)> {
        use petgraph::graph::NodeIndex;
        use petgraph::visit::IntoNodeReferences;
        let mut scored: Vec<(&Node, usize)> = self
            .graph
            .node_references()
            .map(|(idx, node): (NodeIndex, &Node)| {
                let degree = self.graph.edges(idx).count()
                    + self.graph.edges_directed(idx, petgraph::Direction::Incoming).count();
                (node, degree)
            })
            .collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.truncate(top_n);
        scored
    }
}

impl Default for GraphStorage {
    fn default() -> Self { Self::new() }
}
