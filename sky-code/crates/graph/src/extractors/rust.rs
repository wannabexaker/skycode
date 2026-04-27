//! Rust AST extractor using tree-sitter
//!
//! Extracts structs, enums, traits, functions, impl blocks, use declarations,
//! and call / macro-invocation edges. EXTRACTED confidence = 1.0.

use crate::{Edge, EdgeType, Node, NodeType};
use crate::extractors::Extractor;
use anyhow::Result;
use tree_sitter::{Language, Node as TsNode, Parser};

pub struct RustExtractor {
    language: Language,
}

impl RustExtractor {
    pub fn new() -> Self {
        Self {
            language: Language::new(tree_sitter_rust::LANGUAGE),
        }
    }
}

impl Default for RustExtractor {
    fn default() -> Self { Self::new() }
}

impl Extractor for RustExtractor {
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
        "use_declaration" => extract_use(node, src, file_path, module_id, nodes, edges),
        "struct_item" => extract_struct(node, src, file_path, module_id, nodes, edges),
        "enum_item" => extract_enum(node, src, file_path, module_id, nodes, edges),
        "trait_item" => extract_trait(node, src, file_path, module_id, nodes, edges),
        "function_item" => {
            extract_function(node, src, file_path, module_id, module_id, nodes, edges);
        }
        "impl_item" => extract_impl(node, src, file_path, module_id, nodes, edges),
        // `pub struct Foo` etc — unwrap visibility
        "visibility_modifier" => {}
        _ => {}
    }
}

// ── Struct / Enum / Trait ─────────────────────────────────────────────────

fn extract_struct(
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

    let id = format!("{}::{}", file_path, name);

    nodes.push(Node {
        id: id.clone(),
        node_type: NodeType::Class,
        label: name,
        file_path: file_path.to_string(),
        line: node.start_position().row + 1,
        docstring: extract_doc_comment(node, src),
    });

    edges.push(Edge {
        source: parent_id.to_string(),
        target: id,
        edge_type: EdgeType::Contains,
        confidence: 1.0,
        evidence: None,
    });
}

fn extract_enum(
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

    let id = format!("{}::{}", file_path, name);

    nodes.push(Node {
        id: id.clone(),
        node_type: NodeType::Class,
        label: format!("(enum) {}", name),
        file_path: file_path.to_string(),
        line: node.start_position().row + 1,
        docstring: extract_doc_comment(node, src),
    });

    edges.push(Edge {
        source: parent_id.to_string(),
        target: id,
        edge_type: EdgeType::Contains,
        confidence: 1.0,
        evidence: None,
    });
}

fn extract_trait(
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

    let id = format!("{}::{}", file_path, name);

    nodes.push(Node {
        id: id.clone(),
        node_type: NodeType::Class,
        label: format!("(trait) {}", name),
        file_path: file_path.to_string(),
        line: node.start_position().row + 1,
        docstring: extract_doc_comment(node, src),
    });

    edges.push(Edge {
        source: parent_id.to_string(),
        target: id,
        edge_type: EdgeType::Contains,
        confidence: 1.0,
        evidence: None,
    });
}

// ── Impl block ────────────────────────────────────────────────────────────

fn extract_impl(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    module_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    // The `type` field is the type being implemented
    let impl_type = node
        .child_by_field_name("type")
        .map(|n| node_text(&n, src))
        .unwrap_or_default();
    if impl_type.is_empty() { return; }

    let struct_id = format!("{}::{}", file_path, impl_type);

    // Optional trait being implemented: `impl Trait for Type`
    if let Some(trait_node) = node.child_by_field_name("trait") {
        let trait_name = node_text(&trait_node, src);
        if !trait_name.is_empty() {
            edges.push(Edge {
                source: struct_id.clone(),
                target: trait_name,
                edge_type: EdgeType::Inherits,   // closest semantic: implements a trait
                confidence: 1.0,
                evidence: Some(format!("line {}", node.start_position().row + 1)),
            });
        }
    }

    // Methods inside the impl body
    if let Some(body) = node.child_by_field_name("body") {
        let mut bcur = body.walk();
        for child in body.children(&mut bcur) {
            if child.kind() == "function_item" {
                extract_function(&child, src, file_path, &struct_id, &struct_id, nodes, edges);
            }
        }
    }
}

// ── Function extraction ───────────────────────────────────────────────────

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

    let is_method = scope_id.contains("::")
        && !scope_id.starts_with("module::");
    let fn_id = format!("{}::{}", parent_id, name);

    nodes.push(Node {
        id: fn_id.clone(),
        node_type: if is_method { NodeType::Method } else { NodeType::Function },
        label: name,
        file_path: file_path.to_string(),
        line: node.start_position().row + 1,
        docstring: extract_doc_comment(node, src),
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

// ── Use / import extraction ───────────────────────────────────────────────

fn extract_use(
    node: &TsNode,
    src: &[u8],
    file_path: &str,
    module_id: &str,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
) {
    // Capture the full use path text
    let full_text = node_text(node, src);
    // Strip `use ` prefix and trailing `;`
    let path = full_text
        .trim_start_matches("use ")
        .trim_end_matches(';')
        .trim()
        .to_string();
    if path.is_empty() { return; }

    let import_id = format!("import::{}", path);
    nodes.push(Node {
        id: import_id.clone(),
        node_type: NodeType::Import,
        label: path,
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

// ── Call / macro collection ───────────────────────────────────────────────

fn collect_calls(node: &TsNode, src: &[u8], caller_id: &str, edges: &mut Vec<Edge>) {
    match node.kind() {
        "call_expression" => {
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
        "macro_invocation" => {
            let macro_name = node
                .child_by_field_name("macro")
                .map(|n| node_text(&n, src))
                .unwrap_or_default();
            if !macro_name.is_empty() {
                edges.push(Edge {
                    source: caller_id.to_string(),
                    target: format!("{}!", macro_name),
                    edge_type: EdgeType::Calls,
                    confidence: 1.0,
                    evidence: Some(format!("line {}", node.start_position().row + 1)),
                });
            }
        }
        _ => {}
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

/// Extract a leading `/// ...` or `/** ... */` (doc comment block) attached to a node.
fn extract_doc_comment(node: &TsNode, src: &[u8]) -> Option<String> {
    let prev = node.prev_named_sibling()?;
    if prev.kind() == "line_comment" || prev.kind() == "block_comment" {
        let text = node_text(&prev, src);
        let cleaned = text
            .trim_start_matches("///")
            .trim_start_matches("//!")
            .trim_start_matches("//")
            .trim_start_matches("/**")
            .trim_end_matches("*/")
            .trim()
            .to_string();
        if !cleaned.is_empty() { return Some(cleaned); }
    }
    None
}
