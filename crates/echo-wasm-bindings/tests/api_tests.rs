// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! API surface tests for the DemoKernel WASM bindings shim.
use echo_wasm_bindings::{DemoKernel, SemanticOp, Value};

#[test]
fn api_add_set_connect_delete_roundtrip() {
    let mut k = DemoKernel::new();
    k.add_node("A".into());
    k.add_node("B".into());
    k.set_field("A".into(), "name".into(), Value::Str("Server".into()));
    k.connect("A".into(), "B".into());
    k.delete_node("B".into());

    let graph = k.graph();
    assert!(graph.nodes.contains_key("A"));
    assert!(!graph.nodes.contains_key("B"));
    assert_eq!(graph.edges.len(), 0); // edge removed with B deletion

    let history = k.history();
    assert_eq!(history.len(), 5);
    assert_eq!(history[0].op, SemanticOp::AddNode);
}

#[test]
fn set_field_missing_node_is_noop_and_not_logged() {
    let mut k = DemoKernel::new();
    k.set_field("missing".into(), "name".into(), Value::Str("Nope".into()));

    let graph = k.graph();
    assert!(!graph.nodes.contains_key("missing"));

    let history = k.history();
    assert_eq!(history.len(), 0);
}

#[test]
fn connect_requires_existing_nodes() {
    let mut k = DemoKernel::new();
    k.add_node("A".into());

    k.connect("A".into(), "B".into());

    let graph = k.graph();
    assert_eq!(graph.edges.len(), 0);

    let history = k.history();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].op, SemanticOp::AddNode);
}

#[test]
fn delete_missing_node_is_noop_and_not_logged() {
    let mut k = DemoKernel::new();
    k.delete_node("missing".into());

    let graph = k.graph();
    assert!(!graph.nodes.contains_key("missing"));

    let history = k.history();
    assert_eq!(history.len(), 0);
}
