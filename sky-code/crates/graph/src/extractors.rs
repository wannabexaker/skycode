//! AST extractors for different languages

pub mod go;
pub mod java;
pub mod python;
pub mod rust;
pub mod typescript;

use crate::{Node, Edge};
use anyhow::Result;

pub trait Extractor {
    fn extract(&self, source: &str, file_path: &str) -> Result<(Vec<Node>, Vec<Edge>)>;
}

