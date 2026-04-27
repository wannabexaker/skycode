//! Python AST extractor using tree-sitter
//!
//! Extracts classes, functions, imports and call edges — EXTRACTED confidence = 1.0.

use crate::{Edge, EdgeType, Node, NodeType};
use crate::extractors::Extractor;
use anyhow::Result;
use tree_sitter::{Language, Node as TsNode, Parser};

pub struct PythonExtractor {
    language: Language,
}

impl PythonExtractor {
    pub fn new() -> Self {
        Self {
            language: Language::new(tree_sitter_python::LANGUAGE),
        }
    }
}

impl Default for PythonExtractor {
    fn default() -> Self { Self::new() }
}

impl Extractor for PythonExtractor {
    fn extract(&self, source: &str, file_path: &str) -> Result<(Vec<Node>, Vec<Edge>)> {
        let mut parser = Parser::new();
        parser.set_language(&self.language)?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse {file_path}"))?;

        let root = tree.root_node();
        let src_bytes = source.as_bytes();

        let mut nodes: Vec<Node> = Vec::new();
        let mut edges: Vec<Edge> = Vec::new();

        // Module node — top-level container
        let module_id = module_id(file_path);
        nodes.push(Node {
            id: module_id.clone(),
            node_type: NodeType::Module,
            label: short_module_name(file_path),
            file_path: file_path.to_string(),
            line: 0,
            docstring: None,
        });

        // Walk top-level children
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            match child.kind() {
                "class_definition" => {
                    extract_class(&child, src_bytes, file_path, &module_id, &mut nodes, &mut edges);
                }
                "function_definition" | "decorated_definition" => {
                    let fn_node = if child.kind() == "decorated_definition" {
                        child.child_by_field_name("definition").unwrap_or(child)
                    } else {
                        child
                    };
                    if fn_node.kind() == "function_definition" {
                        extract_function(
                            &fn_node, src_bytes, file_path, &module_id, &module_id,
                            &mut nodes, &mut edges,
                        );
                    }
                }
                "import_statement" => {
                    extract_import(&child, src_bytes, file_path, &module_id, &mut nodes, &mut edges);
                }
                "import_from_statement" => {
                    extract_from_import(&child, src_bytes, file_path, &module_id, &mut nodes, &mut edges);
                }
                _ => {}
            }
        }

        Ok((nodes, edges))
    }
}

// ── Extraction helpers ──────────────────────────────────────────────────────

fn extract_class(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    module_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(&n, src))
        .unwrap_or_default();
    if name.is_empty() { return; }

    let class_id = format!("{}::{}", file_path, name);

    // Inherit edges
    let mut inherit_edges = Vec::new();
    if let Some(bases) = node.child_by_field_name("superclasses") {
        let mut bcur = bases.walk();
        for base in bases.children(&mut bcur) {
            let base_name = node_text(&base, src);
            if !base_name.is_empty() {
                inherit_edges.push(Edge {
                    source: class_id.clone(),
                    target: base_name,
                    edge_type: EdgeType::Inherits,
                    confidence: 1.0,
                    evidence: Some(format!("line {}", node.start_position().row + 1)),
                });
            }
        }
    }

    // Docstring
    let docstring = extract_docstring(node, src);

    nodes.push(Node {
        id: class_id.clone(),
        node_type: NodeType::Class,
        label: name.clone(),
        file_path: file_path.to_string(),
        line: node.start_position().row + 1,
        docstring,
    });

    edges.push(Edge {
        source: module_id.to_string(),
        target: class_id.clone(),
        edge_type: EdgeType::Contains,
        confidence: 1.0,
        evidence: None,
    });
    edges.extend(inherit_edges);

    // Extract methods from class body
    if let Some(body) = node.child_by_field_name("body") {
        let mut bcur = body.walk();
        for child in body.children(&mut bcur) {
            let fn_node = if child.kind() == "decorated_definition" {
                child.child_by_field_name("definition").unwrap_or(child)
            } else {
                child
            };
            if fn_node.kind() == "function_definition" {
                extract_function(&fn_node, src, file_path, &class_id, &class_id, nodes, edges);
            }
        }
    }
}

fn extract_function(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    parent_id: &str,
    scope_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(&n, src))
        .unwrap_or_default();
    if name.is_empty() { return; }

    let kind = if parent_id.starts_with("module::") {
        NodeType::Function
    } else {
        NodeType::Method
    };
    let fn_id = format!("{}::{}", parent_id, name);
    let docstring = extract_docstring(node, src);

    nodes.push(Node {
        id: fn_id.clone(),
        node_type: kind,
        label: name.clone(),
        file_path: file_path.to_string(),
        line: node.start_position().row + 1,
        docstring,
    });

    edges.push(Edge {
        source: parent_id.to_string(),
        target: fn_id.clone(),
        edge_type: EdgeType::Contains,
        confidence: 1.0,
        evidence: None,
    });

    // Scan function body for call expressions
    if let Some(body) = node.child_by_field_name("body") {
        collect_calls(&body, src, &fn_id, edges);
    }
}

fn collect_calls(node: &TsNode, src: &[u8], caller_id: &str, edges: &mut Vec<Edge>) {
    if node.kind() == "call" {
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
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_calls(&child, src, caller_id, edges);
    }
}

fn extract_import(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    module_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "dotted_name" || child.kind() == "aliased_import" {
            let import_name = node_text(&child, src);
            if import_name.is_empty() { continue; }
            let import_id = format!("import::{}", import_name);
            nodes.push(Node {
                id: import_id.clone(),
                node_type: NodeType::Import,
                label: import_name,
                file_path: file_path.to_string(),
                line: node.start_position().row + 1,
                docstring: None,
            });
            edges.push(Edge {
                source: module_id.to_string(),
                target: import_id,
                edge_type: EdgeType::Imports,
                confidence: 1.0,
                evidence: None,
            });
        }
    }
}

fn extract_from_import(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    module_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let module_name = node
        .child_by_field_name("module_name")
        .map(|n| node_text(&n, src))
        .unwrap_or_default();
    if module_name.is_empty() { return; }

    let import_id = format!("import::{}", module_name);
    nodes.push(Node {
        id: import_id.clone(),
        node_type: NodeType::Import,
        label: module_name,
        file_path: file_path.to_string(),
        line: node.start_position().row + 1,
        docstring: None,
    });
    edges.push(Edge {
        source: module_id.to_string(),
        target: import_id,
        edge_type: EdgeType::Imports,
        confidence: 1.0,
        evidence: None,
    });
}

// ── Utility helpers ─────────────────────────────────────────────────────────

fn node_text(node: &TsNode, src: &[u8]) -> String {
    node.utf8_text(src).unwrap_or("").to_string()
}

fn module_id(file_path: &str) -> String {
    format!("module::{}", file_path)
}

fn short_module_name(file_path: &str) -> String {
    std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path)
        .to_string()
}

fn extract_docstring(node: &TsNode, src: &[u8]) -> Option<String> {
    let body = node.child_by_field_name("body")?;
    let mut cursor = body.walk();
    let first = body.children(&mut cursor).next()?;
    if first.kind() == "expression_statement" {
        let expr = first.child(0)?;
        if matches!(expr.kind(), "string" | "concatenated_string") {
            let text = node_text(&expr, src);
            return Some(text.trim_matches(|c| c == '"' || c == '\'').to_string());
        }
    }
    None
}

