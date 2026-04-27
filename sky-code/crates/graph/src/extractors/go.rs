//! Go AST extractor using tree-sitter

use crate::extractors::Extractor;
use crate::{Edge, EdgeType, Node, NodeType};
use anyhow::Result;
use tree_sitter::{Language, Node as TsNode, Parser};

pub struct GoExtractor {
    language: Language,
}

impl GoExtractor {
    pub fn new() -> Self {
        Self {
            language: Language::new(tree_sitter_go::LANGUAGE),
        }
    }
}

impl Default for GoExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl Extractor for GoExtractor {
    fn extract(&self, source: &str, file_path: &str) -> Result<(Vec<Node>, Vec<Edge>)> {
        let mut parser = Parser::new();
        parser.set_language(&self.language)?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse {file_path}"))?;

        let root = tree.root_node();
        let src = source.as_bytes();

        let mut nodes: Vec<Node> = Vec::new();
        let mut edges: Vec<Edge> = Vec::new();

        let module_id = module_id(file_path);
        nodes.push(Node {
            id: module_id.clone(),
            node_type: NodeType::Module,
            label: short_name(file_path),
            file_path: file_path.to_string(),
            line: 0,
            docstring: None,
        });

        let mut cur = root.walk();
        for child in root.children(&mut cur) {
            match child.kind() {
                "import_declaration" => extract_imports(&child, src, file_path, &module_id, &mut nodes, &mut edges),
                "type_declaration" => extract_types(&child, src, file_path, &module_id, &mut nodes, &mut edges),
                "function_declaration" => {
                    extract_function(&child, src, file_path, &module_id, NodeType::Function, &mut nodes, &mut edges)
                }
                "method_declaration" => {
                    extract_function(&child, src, file_path, &module_id, NodeType::Method, &mut nodes, &mut edges)
                }
                _ => {}
            }
        }

        Ok((nodes, edges))
    }
}

fn extract_imports(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    module_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        if child.kind() == "import_spec" {
            let path = child
                .child_by_field_name("path")
                .or_else(|| child.named_child(0))
                .map(|n| node_text(&n, src).trim_matches('"').to_string())
                .unwrap_or_default();
            if path.is_empty() {
                continue;
            }

            let import_id = format!("{}::import::{}", file_path, path);
            nodes.push(Node {
                id: import_id.clone(),
                node_type: NodeType::Import,
                label: path.clone(),
                file_path: file_path.to_string(),
                line: child.start_position().row + 1,
                docstring: None,
            });
            edges.push(Edge {
                source: module_id.to_string(),
                target: import_id,
                edge_type: EdgeType::Imports,
                confidence: 1.0,
                evidence: Some(format!("line {}", child.start_position().row + 1)),
            });
        }
    }
}

fn extract_types(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    module_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        if child.kind() != "type_spec" {
            continue;
        }
        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(&n, src))
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let id = format!("{}::{}", file_path, name);
        nodes.push(Node {
            id: id.clone(),
            node_type: NodeType::Class,
            label: name,
            file_path: file_path.to_string(),
            line: child.start_position().row + 1,
            docstring: None,
        });
        edges.push(Edge {
            source: module_id.to_string(),
            target: id,
            edge_type: EdgeType::Contains,
            confidence: 1.0,
            evidence: None,
        });
    }
}

fn extract_function(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    module_id: &str,
    kind: NodeType,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(&n, src))
        .unwrap_or_default();
    if name.is_empty() {
        return;
    }

    let fn_id = format!("{}::{}", module_id, name);
    nodes.push(Node {
        id: fn_id.clone(),
        node_type: kind,
        label: name,
        file_path: file_path.to_string(),
        line: node.start_position().row + 1,
        docstring: None,
    });
    edges.push(Edge {
        source: module_id.to_string(),
        target: fn_id.clone(),
        edge_type: EdgeType::Contains,
        confidence: 1.0,
        evidence: None,
    });

    if let Some(body) = node.child_by_field_name("body") {
        collect_calls(&body, src, &fn_id, edges);
    }
}

fn collect_calls(node: &TsNode, src: &[u8], caller_id: &str, edges: &mut Vec<Edge>) {
    if node.kind() == "call_expression" {
        let callee = node
            .child_by_field_name("function")
            .map(|n| node_text(&n, src))
            .unwrap_or_default();
        if !callee.is_empty() {
            edges.push(Edge {
                source: caller_id.to_string(),
                target: callee,
                edge_type: EdgeType::Calls,
                confidence: 1.0,
                evidence: Some(format!("line {}", node.start_position().row + 1)),
            });
        }
    }

    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        collect_calls(&child, src, caller_id, edges);
    }
}

fn module_id(file_path: &str) -> String {
    format!("module::{file_path}")
}

fn short_name(file_path: &str) -> String {
    std::path::Path::new(file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path)
        .to_string()
}

fn node_text(node: &TsNode, src: &[u8]) -> String {
    node.utf8_text(src).unwrap_or("").trim().to_string()
}
