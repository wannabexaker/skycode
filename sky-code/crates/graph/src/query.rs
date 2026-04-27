//! Graph query operations: keyword search + shortest path

use crate::storage::GraphStorage;
use crate::{Edge, EdgeType, Node, NodeType};
use anyhow::Result;
use petgraph::algo::dijkstra;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;

fn find_node_index(storage: &GraphStorage, needle: &str) -> Option<NodeIndex> {
    storage
        .graph
        .node_weights()
        .find(|n| n.label == needle || n.id.contains(needle))
        .and_then(|n| storage.node_index.get(&n.id).copied())
}

/// Simple keyword search — returns nodes whose label/id contains the query.
pub fn search<'a>(storage: &'a GraphStorage, keyword: &str) -> Vec<&'a Node> {
    let kw = keyword.to_lowercase();
    storage
        .graph
        .node_weights()
        .filter(|n| n.label.to_lowercase().contains(&kw) || n.id.to_lowercase().contains(&kw))
        .collect()
}

/// Find the shortest path between two node IDs. Returns the list of node labels.
pub fn shortest_path(storage: &GraphStorage, from: &str, to: &str) -> Result<Option<Vec<String>>> {
    let start_idx = match storage.node_index.get(from) {
        Some(&idx) => idx,
        None => {
            // Try partial match on label/id
            let found = storage
                .graph
                .node_weights()
                .find(|n| n.id.contains(from) || n.label.contains(from))
                .and_then(|n| storage.node_index.get(&n.id).copied());
            match found {
                Some(idx) => idx,
                None => return Ok(None),
            }
        }
    };

    let end_idx = match storage.node_index.get(to) {
        Some(&idx) => idx,
        None => {
            let found = storage
                .graph
                .node_weights()
                .find(|n| n.id.contains(to) || n.label.contains(to))
                .and_then(|n| storage.node_index.get(&n.id).copied());
            match found {
                Some(idx) => idx,
                None => return Ok(None),
            }
        }
    };

    let costs = dijkstra(&storage.graph, start_idx, Some(end_idx), |_| 1u32);

    if !costs.contains_key(&end_idx) {
        return Ok(None);
    }

    // Reconstruct path by BFS
    let path = reconstruct_path(&storage.graph, start_idx, end_idx)?;
    let labels: Vec<String> = path
        .iter()
        .map(|&idx| storage.graph[idx].label.clone())
        .collect();

    Ok(Some(labels))
}

fn reconstruct_path(
    graph: &petgraph::graph::DiGraph<Node, Edge>,
    start: NodeIndex,
    end: NodeIndex,
) -> Result<Vec<NodeIndex>> {
    use std::collections::{HashMap, VecDeque};

    let mut visited: HashMap<NodeIndex, NodeIndex> = HashMap::new();
    let mut queue = VecDeque::new();
    queue.push_back(start);

    while let Some(current) = queue.pop_front() {
        if current == end {
            // Rebuild path
            let mut path = vec![end];
            let mut node = end;
            while node != start {
                node = *visited.get(&node).unwrap();
                path.push(node);
            }
            path.reverse();
            return Ok(path);
        }
        for edge in graph.edges(current) {
            let next = edge.target();
            if !visited.contains_key(&next) {
                visited.insert(next, current);
                queue.push_back(next);
            }
        }
    }

    Ok(vec![])
}

/// List all direct neighbors of a node (by label or id fragment)
pub fn neighbors<'a>(storage: &'a GraphStorage, node_label: &str) -> Vec<(&'a Node, String)> {
    let idx = find_node_index(storage, node_label);

    let Some(idx) = idx else { return vec![]; };

    storage.graph.edges(idx)
        .filter_map(|e| {
            let target_node = storage.graph.node_weight(e.target())?;
            let edge_label = e.weight().edge_type.to_string();
            Some((target_node, edge_label))
        })
        .collect()
}

/// Return incoming callers for a node (by label or id fragment).
/// Includes only `calls` edges, i.e. "who calls this function/method".
pub fn callers<'a>(storage: &'a GraphStorage, node_label: &str) -> Vec<(&'a Node, String)> {
    let idx = find_node_index(storage, node_label);

    let Some(idx) = idx else { return vec![]; };

    storage
        .graph
        .edges_directed(idx, petgraph::Direction::Incoming)
        .filter(|e| e.weight().edge_type == EdgeType::Calls)
        .filter_map(|e| {
            let source_node = storage.graph.node_weight(e.source())?;
            let edge_label = e.weight().edge_type.to_string();
            Some((source_node, edge_label))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::callers;
    use crate::storage::GraphStorage;
    use crate::{Edge, EdgeType, Node, NodeType};

    #[test]
    fn callers_returns_incoming_call_edges_only() {
        let mut storage = GraphStorage::new();

        storage.add_node(Node {
            id: "module::target".to_string(),
            node_type: NodeType::Function,
            label: "target".to_string(),
            file_path: "x.py".to_string(),
            line: 1,
            docstring: None,
        });
        storage.add_node(Node {
            id: "module::caller".to_string(),
            node_type: NodeType::Function,
            label: "caller".to_string(),
            file_path: "x.py".to_string(),
            line: 2,
            docstring: None,
        });

        storage.add_edge(Edge {
            source: "module::caller".to_string(),
            target: "module::target".to_string(),
            edge_type: EdgeType::Calls,
            confidence: 1.0,
            evidence: Some("target()".to_string()),
        });
        storage.add_edge(Edge {
            source: "module::caller".to_string(),
            target: "module::target".to_string(),
            edge_type: EdgeType::Contains,
            confidence: 1.0,
            evidence: None,
        });

        let results = callers(&storage, "target");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0.label, "caller");
        assert_eq!(results[0].1, "calls");
    }
}
