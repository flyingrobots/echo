// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Executable graph-schema relations for the authored-effect program boundary.
#![cfg(feature = "trusted_runtime")]
#![allow(clippy::expect_used, clippy::panic)]
use echo_edict_canonical::{encode_canonical_cbor_v1 as encode, CanonicalValueV1 as Value};
use warp_core::edict_effects::{EffectGraphSchema, EffectGraphSchemaError, EffectNodeAddressing};
use warp_core::TypeId;
fn map(fields: impl IntoIterator<Item = (&'static str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (Value::Text(key.into()), value))
            .collect(),
    )
}
fn id(byte: u8) -> Value {
    Value::Bytes(vec![byte; 32])
}
fn schema() -> Value {
    map([
        (
            "node_types",
            Value::Map(vec![
                (
                    id(1),
                    map([
                        ("addressing", Value::Integer(0)),
                        ("atom_types", Value::Array(vec![id(10), id(11)])),
                    ]),
                ),
                (
                    id(2),
                    map([
                        ("addressing", Value::Integer(1)),
                        ("atom_types", Value::Array(vec![id(12)])),
                    ]),
                ),
            ]),
        ),
        (
            "edge_types",
            Value::Map(vec![(
                id(3),
                Value::Array(vec![
                    Value::Array(vec![id(1), id(2)]),
                    Value::Array(vec![id(2), id(1)]),
                ]),
            )]),
        ),
    ])
}
fn field_mut<'a>(value: &'a mut Value, name: &str) -> &'a mut Value {
    let Value::Map(fields) = value else {
        panic!("map")
    };
    &mut fields
        .iter_mut()
        .find(|(key, _)| key == &Value::Text(name.into()))
        .expect("field")
        .1
}
fn node_mut(value: &mut Value) -> &mut Value {
    let Value::Map(nodes) = field_mut(value, "node_types") else {
        panic!("nodes")
    };
    &mut nodes[0].1
}
fn edges_mut(value: &mut Value) -> &mut Vec<Value> {
    let Value::Map(edges) = field_mut(value, "edge_types") else {
        panic!("edges")
    };
    let Value::Array(pairs) = &mut edges[0].1 else {
        panic!("pairs")
    };
    pairs
}
fn refused(value: Value) {
    assert_eq!(
        EffectGraphSchema::from_canonical_bytes(&encode(&value).expect("canonical fixture")),
        Err(EffectGraphSchemaError::InvalidSchema)
    );
}
#[test]
fn canonical_graph_schema_executes_distinct_node_atom_and_edge_relations() {
    let schema = EffectGraphSchema::from_canonical_bytes(&encode(&schema()).expect("schema bytes"))
        .expect("valid graph schema");
    assert_eq!(
        schema.node_addressing(TypeId([1; 32])),
        Some(EffectNodeAddressing::Content)
    );
    assert_eq!(
        schema.node_addressing(TypeId([2; 32])),
        Some(EffectNodeAddressing::Mutable)
    );
    assert_eq!(schema.node_addressing(TypeId([9; 32])), None);
    assert!(schema.allows_atom(TypeId([1; 32]), TypeId([10; 32])));
    assert!(!schema.allows_atom(TypeId([1; 32]), TypeId([12; 32])));
    assert!(!schema.allows_atom(TypeId([10; 32]), TypeId([1; 32])));
    assert!(schema.allows_edge(TypeId([3; 32]), TypeId([1; 32]), TypeId([2; 32])));
    assert!(!schema.allows_edge(TypeId([3; 32]), TypeId([1; 32]), TypeId([1; 32])));
    assert!(!schema.allows_edge(TypeId([4; 32]), TypeId([1; 32]), TypeId([2; 32])));
}
#[test]
fn graph_schema_refuses_wrong_fields_addressing_identifiers_and_pair_shapes() {
    for mode in 0..7 {
        let mut value = schema();
        match mode {
            0 => {
                let Value::Map(fields) = &mut value else {
                    panic!("root")
                };
                fields.push((Value::Text("extra".into()), Value::Integer(0)));
            }
            1 => {
                let Value::Map(fields) = &mut value else {
                    panic!("root")
                };
                fields.pop();
            }
            2 => *field_mut(node_mut(&mut value), "addressing") = Value::Integer(2),
            3 => {
                let Value::Map(fields) = node_mut(&mut value) else {
                    panic!("node")
                };
                fields.push((Value::Text("extra".into()), Value::Integer(0)));
            }
            4 => {
                *field_mut(node_mut(&mut value), "atom_types") =
                    Value::Array(vec![Value::Bytes(vec![10; 31])]);
            }
            5 => {
                let Value::Map(nodes) = field_mut(&mut value, "node_types") else {
                    panic!("nodes")
                };
                nodes[0].0 = Value::Text("not-an-id".into());
            }
            _ => edges_mut(&mut value)[0] = Value::Array(vec![id(1)]),
        }
        refused(value);
    }
}
#[test]
fn graph_schema_set_order_and_duplicates_are_refused_without_normalizing() {
    for mode in 0..4 {
        let mut value = schema();
        match mode {
            0 => {
                *field_mut(node_mut(&mut value), "atom_types") = Value::Array(vec![id(11), id(10)]);
            }
            1 => {
                *field_mut(node_mut(&mut value), "atom_types") = Value::Array(vec![id(10), id(10)]);
            }
            2 => edges_mut(&mut value).reverse(),
            _ => {
                let first = edges_mut(&mut value)[0].clone();
                edges_mut(&mut value).push(first);
            }
        }
        refused(value);
    }
}

#[test]
fn empty_schema_relations_deny_and_endpoint_pairs_order_by_both_types() {
    let empty = map([
        ("node_types", Value::Map(vec![])),
        ("edge_types", Value::Map(vec![])),
    ]);
    let parsed = EffectGraphSchema::from_canonical_bytes(&encode(&empty).expect("empty bytes"))
        .expect("empty sets are valid");
    assert_eq!(parsed.node_addressing(TypeId([1; 32])), None);
    assert!(!parsed.allows_atom(TypeId([1; 32]), TypeId([10; 32])));
    assert!(!parsed.allows_edge(TypeId([3; 32]), TypeId([1; 32]), TypeId([2; 32])));
    let mut value = schema();
    *edges_mut(&mut value) = vec![
        Value::Array(vec![id(1), id(1)]),
        Value::Array(vec![id(1), id(2)]),
        Value::Array(vec![id(2), id(1)]),
    ];
    let parsed = EffectGraphSchema::from_canonical_bytes(&encode(&value).expect("ordered pairs"))
        .expect("same-source pairs sort by target");
    assert!(parsed.allows_edge(TypeId([3; 32]), TypeId([1; 32]), TypeId([1; 32])));
    edges_mut(&mut value).swap(0, 1);
    refused(value);
}

#[test]
fn graph_schema_refuses_noncanonical_root_order_and_trailing_bytes() {
    let mut value = schema();
    let mut reversed = vec![0xa2]; // Two-entry CBOR map, node_types before edge_types.
    for name in ["node_types", "edge_types"] {
        reversed.extend(encode(&Value::Text(name.into())).expect("key"));
        reversed.extend(encode(field_mut(&mut value, name)).expect("field"));
    }
    assert_eq!(
        EffectGraphSchema::from_canonical_bytes(&reversed),
        Err(EffectGraphSchemaError::InvalidSchema)
    );
    let mut trailing = encode(&schema()).expect("valid schema");
    trailing.push(0);
    assert_eq!(
        EffectGraphSchema::from_canonical_bytes(&trailing),
        Err(EffectGraphSchemaError::InvalidSchema)
    );
}
