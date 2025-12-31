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
fn connect_self_loop_is_allowed_and_logged() {
    let mut k = DemoKernel::new();
    k.add_node("A".into());

    k.connect("A".into(), "A".into());

    let graph = k.graph();
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].from, "A");
    assert_eq!(graph.edges[0].to, "A");

    let history = k.history();
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].op, SemanticOp::Connect);
    assert_eq!(history[1].target, "A");
    assert_eq!(history[1].new_value, Some(Value::Str("A".into())));
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

#[test]
fn add_node_duplicate_id_is_noop_and_does_not_clobber_fields() {
    let mut k = DemoKernel::new();
    k.add_node("A".into());
    k.set_field("A".into(), "name".into(), Value::Str("Server".into()));
    k.add_node("A".into());

    let graph = k.graph();
    let a = graph.nodes.get("A").expect("node A missing");
    assert_eq!(a.fields.get("name"), Some(&Value::Str("Server".into())));

    let history = k.history();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].op, SemanticOp::AddNode);
    assert_eq!(history[1].op, SemanticOp::Set);
}

#[test]
fn set_field_logs_subject_and_prior_value() {
    let mut k = DemoKernel::new();
    k.add_node("A".into());

    k.set_field("A".into(), "name".into(), Value::Str("Server".into()));
    k.set_field("A".into(), "name".into(), Value::Str("Client".into()));

    let history = k.history();
    assert_eq!(history.len(), 3);

    let first_set = &history[1];
    assert_eq!(first_set.op, SemanticOp::Set);
    assert_eq!(first_set.target, "A");
    assert_eq!(first_set.subject.as_deref(), Some("name"));
    assert_eq!(first_set.old_value, None);
    assert_eq!(first_set.new_value, Some(Value::Str("Server".into())));

    let second_set = &history[2];
    assert_eq!(second_set.op, SemanticOp::Set);
    assert_eq!(second_set.target, "A");
    assert_eq!(second_set.subject.as_deref(), Some("name"));
    assert_eq!(second_set.old_value, Some(Value::Str("Server".into())));
    assert_eq!(second_set.new_value, Some(Value::Str("Client".into())));
}
