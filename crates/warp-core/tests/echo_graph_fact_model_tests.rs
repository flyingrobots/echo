// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Regression tests for Echo's native graph fact digest law.

use warp_core::{
    CausalObjectId, EntityRef, ExternalEntityRef, FactDigest, GraphEdgeKind, GraphFact,
    GraphObjectKind, GraphValueRef, RuntimeLocalHandle, TransactionLocalObjectId,
};

fn object_id(id: &str) -> CausalObjectId {
    CausalObjectId::new(id)
}

fn object_kind() -> GraphObjectKind {
    GraphObjectKind::with_version("echo", "object", "v0")
}

fn edge_kind() -> GraphEdgeKind {
    GraphEdgeKind::with_version("echo", "contains", "v0")
}

fn value_ref(codec: &str) -> GraphValueRef {
    GraphValueRef::new([7_u8; 32], codec)
}

fn object_exists_fact() -> GraphFact {
    GraphFact::ObjectExists {
        object_id: object_id("object:1"),
        object_kind: object_kind(),
    }
}

#[test]
fn echo_graph_fact_digest_is_deterministic() {
    let fact = object_exists_fact();

    assert_eq!(fact.digest(), fact.digest());
    assert_eq!(fact.build_digest_input_bytes(), fact.build_digest_input_bytes());
}

#[test]
fn echo_graph_fact_digest_distinguishes_fact_kind() {
    let exists = object_exists_fact();
    let retired = GraphFact::ObjectRetired {
        object_id: object_id("object:1"),
    };

    assert_ne!(exists.digest(), retired.digest());
}

#[test]
fn echo_graph_fact_digest_distinguishes_absent_and_empty_fields() {
    let absent = GraphFact::ObjectExists {
        object_id: object_id("object:1"),
        object_kind: GraphObjectKind::new("echo", "object"),
    };
    let empty = GraphFact::ObjectExists {
        object_id: object_id("object:1"),
        object_kind: GraphObjectKind::with_version("echo", "object", ""),
    };

    assert_ne!(absent.digest(), empty.digest());
}

#[test]
fn echo_graph_fact_digest_includes_property_codec() {
    let cbor = GraphFact::PropertySet {
        entity: EntityRef::Causal(object_id("object:1")),
        key: "title".to_owned(),
        value: value_ref("cbor/v1"),
    };
    let bytes = GraphFact::PropertySet {
        entity: EntityRef::Causal(object_id("object:1")),
        key: "title".to_owned(),
        value: value_ref("bytes/v1"),
    };

    assert_ne!(cbor.digest(), bytes.digest());
}

#[test]
fn transaction_local_refs_are_usable_but_not_causal_identity() {
    let staged_ref = EntityRef::TransactionLocal(TransactionLocalObjectId::new(
        "tx:1",
        "step:1",
        "new-object",
    ));
    let fact = GraphFact::PropertyCleared {
        entity: staged_ref.clone(),
        key: "draft".to_owned(),
    };

    assert!(staged_ref.as_causal_object_id().is_none());
    assert_ne!(fact.digest(), FactDigest([0_u8; 32]));
}

#[test]
fn runtime_local_handles_are_usable_but_not_causal_identity() {
    let runtime_ref = EntityRef::RuntimeLocal(RuntimeLocalHandle::new(
        "optic-artifact-handle",
        "runtime:1",
    ));
    let fact = GraphFact::PropertyCleared {
        entity: runtime_ref.clone(),
        key: "cached".to_owned(),
    };

    assert!(runtime_ref.as_causal_object_id().is_none());
    assert_ne!(fact.digest(), FactDigest([0_u8; 32]));
}

#[test]
fn causal_refs_remain_causal_identity() -> Result<(), String> {
    let object_id = object_id("object:1");
    let causal_ref = EntityRef::Causal(object_id.clone());
    let resolved = causal_ref
        .as_causal_object_id()
        .ok_or_else(|| "causal ref should expose durable object id".to_owned())?;

    assert_eq!(resolved, &object_id);
    Ok(())
}

#[test]
fn external_refs_are_usable_but_not_causal_identity() {
    let external_ref = EntityRef::External(ExternalEntityRef::new("jedit", "buffer:demo"));
    let fact = GraphFact::EdgeExists {
        edge_id: object_id("edge:1"),
        from: external_ref.clone(),
        to: EntityRef::Causal(object_id("object:1")),
        edge_kind: edge_kind(),
    };

    assert!(external_ref.as_causal_object_id().is_none());
    assert_ne!(fact.digest(), FactDigest([0_u8; 32]));
}
