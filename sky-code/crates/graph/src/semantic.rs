use crate::storage::GraphStorage;
use crate::{Edge, EdgeType, Node, NodeType};
use std::collections::HashSet;

fn tokenize(label: &str) -> HashSet<String> {
    label
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter_map(|s| {
            let token = s.trim().to_lowercase();
            if token.len() >= 3 {
                Some(token)
            } else {
                None
            }
        })
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let intersection = a.intersection(b).count() as f32;
    let union = a.union(b).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn semantic_candidate(node: &Node) -> bool {
    matches!(
        node.node_type,
        NodeType::Class | NodeType::Function | NodeType::Method
    )
}

/// Infer semantic similarity edges between code entities.
///
/// `threshold` controls minimum similarity score to emit an edge.
/// `max_edges_per_node` caps emitted edges from each source node.
pub fn infer_semantic_edges(
    storage: &GraphStorage,
    threshold: f32,
    max_edges_per_node: usize,
) -> Vec<Edge> {
    let (nodes, _existing_edges) = storage.snapshot_data();
    let semantic_nodes: Vec<&Node> = nodes.iter().filter(|n| semantic_candidate(n)).collect();

    let tokenized: Vec<(&Node, HashSet<String>)> = semantic_nodes
        .iter()
        .map(|n| (*n, tokenize(&n.label)))
        .collect();

    let mut inferred = Vec::new();

    for (i, (source, source_tokens)) in tokenized.iter().enumerate() {
        let mut ranked = Vec::new();

        for (j, (target, target_tokens)) in tokenized.iter().enumerate() {
            if i == j {
                continue;
            }
            if source.id == target.id {
                continue;
            }
            if source.file_path == target.file_path {
                continue;
            }

            let score = jaccard(source_tokens, target_tokens);
            if score < threshold {
                continue;
            }

            let mut shared = source_tokens
                .intersection(target_tokens)
                .cloned()
                .collect::<Vec<_>>();
            shared.sort();
            let shared_preview = shared.into_iter().take(4).collect::<Vec<_>>().join(", ");
            ranked.push((
                score,
                Edge {
                    source: source.id.clone(),
                    target: target.id.clone(),
                    edge_type: EdgeType::SemanticallySimilarTo,
                    confidence: score,
                    evidence: Some(format!("shared_tokens: {shared_preview}")),
                },
            ));
        }

        ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        for (_, edge) in ranked.into_iter().take(max_edges_per_node) {
            inferred.push(edge);
        }
    }

    inferred
}

#[cfg(test)]
mod tests {
    use super::infer_semantic_edges;
    use crate::storage::GraphStorage;
    use crate::{Node, NodeType};

    #[test]
    fn infers_similarity_for_related_names_across_files() {
        let mut storage = GraphStorage::new();
        storage.add_node(Node {
            id: "a::build_user_profile".to_string(),
            node_type: NodeType::Function,
            label: "build_user_profile".to_string(),
            file_path: "a.py".to_string(),
            line: 1,
            docstring: None,
        });
        storage.add_node(Node {
            id: "b::user_profile_builder".to_string(),
            node_type: NodeType::Function,
            label: "user_profile_builder".to_string(),
            file_path: "b.ts".to_string(),
            line: 10,
            docstring: None,
        });

        let edges = infer_semantic_edges(&storage, 0.30, 5);
        assert!(!edges.is_empty());
        assert!(edges.iter().all(|e| e.confidence < 1.0));
    }
}
