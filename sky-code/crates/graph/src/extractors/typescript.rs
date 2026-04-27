//! TypeScript / TSX AST extractor using tree-sitter
//!
//! Extracts classes, functions, methods, interfaces, imports and call edges.
//! EXTRACTED confidence = 1.0  (AST-derived, not inferred).

use crate::{Edge, EdgeType, Node, NodeType};
use crate::extractors::Extractor;
use anyhow::Result;
use tree_sitter::{Language, Node as TsNode, Parser};

pub struct TypeScriptExtractor {
    language: Language,
}

impl TypeScriptExtractor {
    pub fn new() -> Self {
        Self {
            language: Language::new(tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
        }
    }

    pub fn new_tsx() -> Self {
        Self {
            language: Language::new(tree_sitter_typescript::LANGUAGE_TSX),
        }
    }
}

impl Default for TypeScriptExtractor {
    fn default() -> Self { Self::new() }
}

impl Extractor for TypeScriptExtractor {
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
            dispatch_top_level(&child, src, file_path, &module_id, &mut nodes, &mut edges);
        }

        Ok((nodes, edges))
    }
}

// ── Top-level dispatcher ──────────────────────────────────────────────────

fn dispatch_top_level(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    module_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    match node.kind() {
        "import_statement" => extract_import(node, src, file_path, module_id, nodes, edges),
        "class_declaration" | "abstract_class_declaration" => {
            extract_class(node, src, file_path, module_id, nodes, edges);
        }
        "function_declaration" | "generator_function_declaration" => {
            extract_function(node, src, file_path, module_id, nodes, edges);
        }
        "interface_declaration" => {
            extract_interface(node, src, file_path, module_id, nodes, edges);
        }
        // export default class / export function / export class
        "export_statement" => {
            if let Some(inner) = node.child_by_field_name("declaration") {
                dispatch_top_level(&inner, src, file_path, module_id, nodes, edges);
            }
        }
        // const Foo = () => {} / const Foo = function() {}
        "lexical_declaration" | "variable_declaration" => {
            extract_variable_fn(node, src, file_path, module_id, nodes, edges);
        }
        _ => {}
    }
}

// ── Class extraction ──────────────────────────────────────────────────────

fn extract_class(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    parent_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(&n, src))
        .unwrap_or_default();
    if name.is_empty() { return; }

    let class_id = format!("{}::{}", file_path, name);

    nodes.push(Node {
        id: class_id.clone(),
        node_type: NodeType::Class,
        label: name.clone(),
        file_path: file_path.to_string(),
        line: node.start_position().row + 1,
        docstring: extract_leading_comment(node, src),
    });

    edges.push(Edge {
        source: parent_id.to_string(),
        target: class_id.clone(),
        edge_type: EdgeType::Contains,
        confidence: 1.0,
        evidence: None,
    });

    // Extends clause
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        if child.kind() == "class_heritage" {
            let mut hcur = child.walk();
            for hchild in child.children(&mut hcur) {
                if hchild.kind() == "extends_clause" {
                    if let Some(val) = hchild.child_by_field_name("value") {
                        let base = node_text(&val, src);
                        if !base.is_empty() {
                            edges.push(Edge {
                                source: class_id.clone(),
                                target: base,
                                edge_type: EdgeType::Inherits,
                                confidence: 1.0,
                                evidence: Some(format!("line {}", hchild.start_position().row + 1)),
                            });
                        }
                    }
                }
            }
        }
    }

    // Methods inside class body
    if let Some(body) = node.child_by_field_name("body") {
        let mut bcur = body.walk();
        for child in body.children(&mut bcur) {
            match child.kind() {
                "method_definition" | "public_field_definition" => {
                    if child.kind() == "method_definition" {
                        extract_method(&child, src, file_path, &class_id, nodes, edges);
                    }
                }
                _ => {}
            }
        }
    }
}

// ── Interface extraction ──────────────────────────────────────────────────

fn extract_interface(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    parent_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(&n, src))
        .unwrap_or_default();
    if name.is_empty() { return; }

    let iface_id = format!("{}::{}", file_path, name);

    // Treat interfaces as Class nodes (structural type)
    nodes.push(Node {
        id: iface_id.clone(),
        node_type: NodeType::Class,
        label: format!("(interface) {}", name),
        file_path: file_path.to_string(),
        line: node.start_position().row + 1,
        docstring: None,
    });

    edges.push(Edge {
        source: parent_id.to_string(),
        target: iface_id,
        edge_type: EdgeType::Contains,
        confidence: 1.0,
        evidence: None,
    });
}

// ── Function / method extraction ─────────────────────────────────────────

fn extract_function(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    parent_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(&n, src))
        .unwrap_or_default();
    if name.is_empty() { return; }

    let fn_id = format!("{}::{}", parent_id, name);

    nodes.push(Node {
        id: fn_id.clone(),
        node_type: NodeType::Function,
        label: name,
        file_path: file_path.to_string(),
        line: node.start_position().row + 1,
        docstring: None,
    });

    edges.push(Edge {
        source: parent_id.to_string(),
        target: fn_id.clone(),
        edge_type: EdgeType::Contains,
        confidence: 1.0,
        evidence: None,
    });

    if let Some(body) = node.child_by_field_name("body") {
        collect_calls(&body, src, &fn_id, edges);
    }
}

fn extract_method(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    class_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let name = node
        .child_by_field_name("name")
        .map(|n| node_text(&n, src))
        .unwrap_or_default();
    if name.is_empty() { return; }

    let method_id = format!("{}::{}", class_id, name);

    nodes.push(Node {
        id: method_id.clone(),
        node_type: NodeType::Method,
        label: name,
        file_path: file_path.to_string(),
        line: node.start_position().row + 1,
        docstring: None,
    });

    edges.push(Edge {
        source: class_id.to_string(),
        target: method_id.clone(),
        edge_type: EdgeType::Contains,
        confidence: 1.0,
        evidence: None,
    });

    if let Some(body) = node.child_by_field_name("body") {
        collect_calls(&body, src, &method_id, edges);
    }
}

// ── Arrow / assigned function extraction ─────────────────────────────────

fn extract_variable_fn(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    parent_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    let mut cur = node.walk();
    for child in node.children(&mut cur) {
        if child.kind() != "variable_declarator" { continue; }

        let name = child
            .child_by_field_name("name")
            .map(|n| node_text(&n, src))
            .unwrap_or_default();
        if name.is_empty() { continue; }

        let value = match child.child_by_field_name("value") {
            Some(v) => v,
            None => continue,
        };

        let is_fn = matches!(
            value.kind(),
            "arrow_function" | "function" | "generator_function"
        );
        if !is_fn { continue; }

        let fn_id = format!("{}::{}", parent_id, name);

        nodes.push(Node {
            id: fn_id.clone(),
            node_type: NodeType::Function,
            label: name,
            file_path: file_path.to_string(),
            line: child.start_position().row + 1,
            docstring: None,
        });

        edges.push(Edge {
            source: parent_id.to_string(),
            target: fn_id.clone(),
            edge_type: EdgeType::Contains,
            confidence: 1.0,
            evidence: None,
        });

        if let Some(body) = value.child_by_field_name("body") {
            collect_calls(&body, src, &fn_id, edges);
        }
    }
}

// ── Import extraction ─────────────────────────────────────────────────────

fn extract_import(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    module_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    // The module path is the `source` field — a string literal
    let source_str = node
        .child_by_field_name("source")
        .map(|n| node_text(&n, src).trim_matches(|c| c == '"' || c == '\'').to_string())
        .unwrap_or_default();
    if source_str.is_empty() { return; }

    let import_id = format!("import::{}", source_str);
    nodes.push(Node {
        id: import_id.clone(),
        node_type: NodeType::Import,
        label: source_str,
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

// ── Call collection ───────────────────────────────────────────────────────

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

// ── Utilities ─────────────────────────────────────────────────────────────

fn node_text(node: &TsNode, src: &[u8]) -> String {
    node.utf8_text(src).unwrap_or("").to_string()
}

fn module_id(file_path: &str) -> String {
    format!("module::{}", file_path)
}

fn short_name(file_path: &str) -> String {
    std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path)
        .to_string()
}

/// Extract a leading `/** ... */` or `// ...` comment above a node as a docstring.
fn extract_leading_comment(node: &TsNode, src: &[u8]) -> Option<String> {
    let prev = node.prev_named_sibling()?;
    if matches!(prev.kind(), "comment") {
        let text = node_text(&prev, src);
        let cleaned = text
            .trim_start_matches("//")
            .trim_start_matches("/**")
            .trim_end_matches("*/")
            .trim()
            .to_string();
        if !cleaned.is_empty() { return Some(cleaned); }
    }
    None
}
