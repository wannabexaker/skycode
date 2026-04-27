use graph::extractors::go::GoExtractor;
use graph::extractors::java::JavaExtractor;
use graph::extractors::python::PythonExtractor;
use graph::extractors::rust::RustExtractor;
use graph::extractors::typescript::TypeScriptExtractor;
use graph::extractors::Extractor;
use graph::{build_from_dir, Edge, EdgeType, Node, NodeType};

fn has_node(nodes: &[Node], label: &str, node_type: NodeType) -> bool {
    nodes
        .iter()
        .any(|n| n.label == label && n.node_type == node_type)
}

fn has_edge(edges: &[Edge], edge_type: EdgeType, target_substr: &str) -> bool {
    edges
        .iter()
        .any(|e| e.edge_type == edge_type && e.target.contains(target_substr))
}

#[test]
fn python_extractor_extracts_core_entities_and_calls() {
    let source = r#"
import os
from pkg import thing

class Greeter:
    def hello(self):
        helper()

def helper():
    print('ok')
"#;

    let extractor = PythonExtractor::new();
    let (nodes, edges) = extractor
        .extract(source, "sample.py")
        .expect("python extraction should succeed");

    assert!(has_node(&nodes, "sample", NodeType::Module));
    assert!(has_node(&nodes, "Greeter", NodeType::Class));
    assert!(has_node(&nodes, "hello", NodeType::Method));
    assert!(has_node(&nodes, "helper", NodeType::Function));

    assert!(has_edge(&edges, EdgeType::Imports, "os"));
    assert!(has_edge(&edges, EdgeType::Imports, "pkg"));
    assert!(has_edge(&edges, EdgeType::Calls, "helper"));
}

#[test]
fn typescript_extractor_extracts_imports_class_methods_and_calls() {
    let source = r#"
import { util } from './utils';

class Service {
  run() {
    execute();
  }
}

function execute() {
  return 1;
}
"#;

    let extractor = TypeScriptExtractor::new();
    let (nodes, edges) = extractor
        .extract(source, "service.ts")
        .expect("typescript extraction should succeed");

    assert!(has_node(&nodes, "service", NodeType::Module));
    assert!(has_node(&nodes, "Service", NodeType::Class));
    assert!(has_node(&nodes, "run", NodeType::Method));
    assert!(has_node(&nodes, "execute", NodeType::Function));

    assert!(has_edge(&edges, EdgeType::Imports, "./utils"));
    assert!(has_edge(&edges, EdgeType::Calls, "execute"));
}

#[test]
fn rust_extractor_extracts_struct_use_methods_and_calls() {
    let source = r#"
use std::fmt;

struct Engine;

impl Engine {
    fn start(&self) {
        helper();
        println!("ok");
    }
}

fn helper() {}
"#;

    let extractor = RustExtractor::new();
    let (nodes, edges) = extractor
        .extract(source, "engine.rs")
        .expect("rust extraction should succeed");

    assert!(has_node(&nodes, "engine", NodeType::Module));
    assert!(has_node(&nodes, "Engine", NodeType::Class));
    assert!(has_node(&nodes, "start", NodeType::Method));
    assert!(has_node(&nodes, "helper", NodeType::Function));

    assert!(has_edge(&edges, EdgeType::Imports, "std::fmt"));
    assert!(has_edge(&edges, EdgeType::Calls, "helper"));
    assert!(has_edge(&edges, EdgeType::Calls, "println!"));
}

#[test]
fn go_extractor_extracts_import_types_functions_and_calls() {
    let source = r#"
package main

import "fmt"

type Engine struct {}

func helper() {}

func run() {
    helper()
    fmt.Println("ok")
}
"#;

    let extractor = GoExtractor::new();
    let (nodes, edges) = extractor
        .extract(source, "engine.go")
        .expect("go extraction should succeed");

    assert!(has_node(&nodes, "engine.go", NodeType::Module));
    assert!(has_node(&nodes, "Engine", NodeType::Class));
    assert!(has_node(&nodes, "run", NodeType::Function));
    assert!(has_edge(&edges, EdgeType::Imports, "fmt"));
    assert!(has_edge(&edges, EdgeType::Calls, "helper"));
}

#[test]
fn java_extractor_extracts_import_class_methods_and_calls() {
    let source = r#"
import java.util.List;

class Service {
    void run() {
        execute();
    }

    void execute() {}
}
"#;

    let extractor = JavaExtractor::new();
    let (nodes, edges) = extractor
        .extract(source, "Service.java")
        .expect("java extraction should succeed");

    assert!(has_node(&nodes, "Service.java", NodeType::Module));
    assert!(has_node(&nodes, "Service", NodeType::Class));
    assert!(has_node(&nodes, "run", NodeType::Method));
    assert!(has_edge(&edges, EdgeType::Imports, "java.util.List"));
    assert!(has_edge(&edges, EdgeType::Calls, "execute"));
}

#[test]
fn build_from_dir_supports_mixed_languages() {
    let base = std::env::temp_dir().join(format!(
        "skycode-graph-mixed-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
    ));

    std::fs::create_dir_all(&base).expect("create temp graph dir");

    std::fs::write(
        base.join("main.py"),
        "def py_fn():\n    return 1\n",
    )
    .expect("write python fixture");

    std::fs::write(
        base.join("service.ts"),
        "export function tsFn(){ return py_fn(); }\n",
    )
    .expect("write typescript fixture");

    std::fs::write(
        base.join("client.js"),
        "function callJs(){ return tsFn(); }\n",
    )
    .expect("write javascript fixture");

    std::fs::write(
        base.join("lib.rs"),
        "fn rs_fn() { println!(\"hi\"); }\n",
    )
    .expect("write rust fixture");

    std::fs::write(
        base.join("worker.go"),
        "package main\nimport \"fmt\"\nfunc goFn(){ fmt.Println(\"ok\") }\n",
    )
    .expect("write go fixture");

    std::fs::write(
        base.join("Runner.java"),
        "import java.util.Map; class Runner { void run(){ execute(); } void execute(){} }\n",
    )
    .expect("write java fixture");

    let storage = build_from_dir(&base, None).expect("build_from_dir should succeed");

    assert!(storage.node_count() > 0, "expected extracted nodes");
    assert!(storage.edge_count() > 0, "expected extracted edges");

    let _ = std::fs::remove_dir_all(base);
}
