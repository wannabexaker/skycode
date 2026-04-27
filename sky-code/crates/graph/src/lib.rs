//! SkyCode Knowledge Graph
//!
//! AST-based code graph extraction with caching and query support.
//! Inspired by graphify — 100% offline, no LLM needed for structural extraction.

pub mod cache;
pub mod extractors;
pub mod query;
pub mod semantic;
pub mod storage;

use std::path::Path;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A code entity node
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Node {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub file_path: String,
    pub line: usize,
    pub docstring: Option<String>,
}

/// A directed relationship between two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub edge_type: EdgeType,
    /// 1.0 = EXTRACTED (AST), <1.0 = INFERRED (LLM)
    pub confidence: f32,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Class,
    Function,
    Method,
    Import,
    Module,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    Calls,
    Imports,
    Inherits,
    Contains,
    SemanticallySimilarTo,
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeType::Calls => write!(f, "calls"),
            EdgeType::Imports => write!(f, "imports"),
            EdgeType::Inherits => write!(f, "inherits"),
            EdgeType::Contains => write!(f, "contains"),
            EdgeType::SemanticallySimilarTo => write!(f, "similar_to"),
        }
    }
}

/// Build a graph from a source directory
pub fn build_from_dir(dir: &Path, cache_db: Option<&str>) -> Result<storage::GraphStorage> {
    use crate::extractors::go::GoExtractor;
    use crate::extractors::java::JavaExtractor;
    use crate::extractors::python::PythonExtractor;
    use crate::extractors::rust::RustExtractor;
    use crate::extractors::typescript::TypeScriptExtractor;
    use crate::extractors::Extractor;

    let mut storage = storage::GraphStorage::new();
    let cache = cache_db
        .map(|p| cache::ExtractionCache::open(p))
        .transpose()?;

    let py_ext = PythonExtractor::new();
    let rs_ext = RustExtractor::new();
    let ts_ext = TypeScriptExtractor::new();
    let tsx_ext = TypeScriptExtractor::new_tsx();
    let go_ext = GoExtractor::new();
    let java_ext = JavaExtractor::new();

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let extractor: &dyn Extractor = match ext {
            "py" => &py_ext,
            "rs" => &rs_ext,
            "ts" | "js" => &ts_ext,
            "tsx" | "jsx" => &tsx_ext,
            "go" => &go_ext,
            "java" => &java_ext,
            _ => continue,
        };

        let src = std::fs::read_to_string(path)?;
        let fp = path.to_string_lossy().to_string();

        let (nodes, edges) = if let Some(ref c) = cache {
            let hash = cache::sha256_hash(&src);
            if let Some((n, e)) = c.get(&fp, &hash)? {
                let nodes: Vec<Node> = serde_json::from_str(&n)?;
                let edges: Vec<Edge> = serde_json::from_str(&e)?;
                (nodes, edges)
            } else {
                let result = extractor.extract(&src, &fp)?;
                c.set(&fp, &hash,
                    &serde_json::to_string(&result.0)?,
                    &serde_json::to_string(&result.1)?)?;
                result
            }
        } else {
            extractor.extract(&src, &fp)?
        };

        for node in nodes { storage.add_node(node); }
        for edge in edges { storage.add_edge(edge); }
    }

    Ok(storage)
}
