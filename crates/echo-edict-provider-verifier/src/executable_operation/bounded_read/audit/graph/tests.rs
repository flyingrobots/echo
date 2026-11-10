// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Verifier laws for graph-read declarations and signatures.
use super::*;
use std::collections::BTreeMap;

fn word() -> Schema {
    Schema::Word(u64::MAX)
}

fn bytes(upper: u64) -> Schema {
    Schema::Bytes { lower: 0, upper }
}

fn table(fields: &[(&str, Schema)]) -> Schema {
    Schema::Row(
        fields
            .iter()
            .map(|(name, schema)| ((*name).to_owned(), schema.clone()))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn many(element: Schema, maximum: u64) -> Schema {
    Schema::Sequence {
        element: Some(Box::new(element)),
        maximum,
    }
}

fn range_input() -> Schema {
    table(&[
        ("warpId", identifier()),
        ("nodeId", identifier()),
        ("edgeType", identifier()),
    ])
}

fn range_output(maximum: u64) -> Schema {
    many(
        table(&[("edgeId", identifier()), ("toId", identifier())]),
        maximum,
    )
}

fn guide() -> Schema {
    table(&[
        ("left", word()),
        ("right", word()),
        ("live", Schema::Boolean),
    ])
}

fn node(atom: u64) -> Schema {
    table(&[
        ("nodeId", identifier()),
        ("nodeType", identifier()),
        ("atomType", bytes(32)),
        ("atomBytes", bytes(atom)),
    ])
}

fn walk_input(guide: Schema, edge_types: u64) -> Schema {
    table(&[
        ("warpId", identifier()),
        ("nodeId", identifier()),
        ("edgeTypes", many(identifier(), edge_types)),
        ("maxNodes", word()),
        ("maxDepth", word()),
        ("guide", guide),
    ])
}

fn walk_output(node: Schema, guide: Schema, capacity: [u64; 3]) -> Schema {
    let edge = table(&[
        ("from", word()),
        ("slot", word()),
        ("edgeId", identifier()),
        ("node", word()),
    ]);
    let occurrence = table(&[
        ("node", word()),
        ("edge", word()),
        ("depth", word()),
        ("guide", guide),
    ]);
    table(&[
        ("nodes", many(node, capacity[0])),
        ("edges", many(edge, capacity[1])),
        ("occurrences", many(occurrence, capacity[2])),
    ])
}

fn candidate(guide: Schema, node: Schema) -> Schema {
    table(&[
        ("guide", guide),
        ("depth", word()),
        ("slot", word()),
        ("parent", node.clone()),
        ("child", node),
    ])
}

fn decision(guide: Schema) -> Schema {
    table(&[
        ("expand", Schema::Boolean),
        ("guide", guide),
        ("wellFormed", Schema::Boolean),
        ("sufficient", Schema::Boolean),
    ])
}

fn text_value(value: &str) -> Value {
    Value::Text(value.to_owned())
}

fn map(fields: Vec<(&str, Value)>) -> Value {
    Value::Map(
        fields
            .into_iter()
            .map(|(key, value)| (text_value(key), value))
            .collect(),
    )
}

fn configuration(graph: Option<Value>) -> Value {
    let mut fields = vec![
        (
            "apiVersion",
            text_value("echo.operation-lowering-configuration/v1"),
        ),
        (
            "programKind",
            text_value("compiler-produced-bounded-read/v1"),
        ),
        ("maxReads", Value::Integer(4)),
        ("maxReadBytes", Value::Integer(64)),
    ];
    if let Some(graph) = graph {
        fields.push(("graphReads", graph));
    }
    map(fields)
}

fn ceilings(scans: i128, nodes: i128, depth: i128) -> Vec<(&'static str, Value)> {
    vec![
        ("maxEdgeScans", Value::Integer(scans)),
        ("maxClosureNodes", Value::Integer(nodes)),
        ("maxClosureDepth", Value::Integer(depth)),
    ]
}

#[test]
fn the_node_only_configuration_declares_no_graph_capability() {
    assert!(matches!(declaration(&configuration(None)), Ok(None)));
}

#[test]
fn a_graph_declaration_names_its_selector_only_when_present() {
    let plain = configuration(Some(map(ceilings(8, 0, 0))));
    assert!(matches!(
        declaration(&plain),
        Ok(Some(Declaration { selector: None }))
    ));
    let mut fields = ceilings(8, 4, 2);
    fields.push(("closureSelector", text_value("choose")));
    let guided = configuration(Some(map(fields)));
    let declared = declaration(&guided);
    assert!(
        matches!(declared, Ok(Some(Declaration { selector: Some(ref name) })) if name == "choose")
    );
}

#[test]
fn declared_ceilings_are_accepted_exactly_and_rejected_one_above() {
    for (scans, nodes, depth, accepted) in [
        (65_536, 4096, 256, true),
        (65_537, 4096, 256, false),
        (65_536, 4097, 256, false),
        (65_536, 4096, 257, false),
        (0, 1, 1, false),
        (1, 0, 0, true),
    ] {
        let value = configuration(Some(map(ceilings(scans, nodes, depth))));
        assert_eq!(
            declaration(&value).is_ok(),
            accepted,
            "{scans} {nodes} {depth}"
        );
    }
}

#[test]
fn a_declaration_with_an_unknown_or_missing_field_is_rejected() {
    let mut unknown = ceilings(1, 1, 1);
    unknown.push(("maxEdges", Value::Integer(1)));
    let mut blank = ceilings(1, 1, 1);
    blank.push(("closureSelector", text_value("")));
    let partial: Vec<_> = ceilings(1, 1, 1).into_iter().take(2).collect();
    for fields in [unknown, blank, partial] {
        assert!(declaration(&configuration(Some(map(fields)))).is_err());
    }
}

#[test]
fn an_edge_read_signature_is_the_typed_range_and_its_bounded_rows() {
    assert!(edge_signature(&range_input(), &range_output(4)).is_ok());
    for (maximum, accepted) in [(1, true), (65_536, true), (0, false), (65_537, false)] {
        assert_eq!(
            edge_signature(&range_input(), &range_output(maximum)).is_ok(),
            accepted,
            "{maximum}"
        );
    }
}

#[test]
fn an_edge_read_with_another_address_or_row_is_rejected() {
    let typed = table(&[
        ("warpId", identifier()),
        ("nodeId", identifier()),
        ("typeId", identifier()),
    ]);
    let loose = table(&[
        ("warpId", identifier()),
        ("nodeId", identifier()),
        ("edgeType", bytes(32)),
    ]);
    for input in [typed, loose] {
        assert!(edge_signature(&input, &range_output(4)).is_err());
    }
    let wide = many(
        table(&[
            ("edgeId", identifier()),
            ("toId", identifier()),
            ("fromId", identifier()),
        ]),
        4,
    );
    assert!(edge_signature(&range_input(), &wide).is_err());
    assert!(edge_signature(&range_input(), &identifier()).is_err());
}

#[test]
fn a_closure_signature_yields_the_node_and_guide_it_shares() {
    let guided = closure_signature(
        &walk_input(guide(), 2),
        &walk_output(node(64), guide(), [4, 8, 8]),
        64,
    );
    assert!(matches!(guided, Ok(ref guided) if guided.node == node(64) && guided.guide == guide()));
}

#[test]
fn a_closure_whose_occurrences_carry_another_guide_is_rejected() {
    let other = table(&[("left", word())]);
    let output = walk_output(node(64), other, [4, 8, 8]);
    assert!(closure_signature(&walk_input(guide(), 2), &output, 64).is_err());
}

#[test]
fn closure_capacities_are_accepted_exactly_and_rejected_one_above() {
    for (capacity, accepted) in [
        ([4096, 65_536, 65_536], true),
        ([4097, 1, 1], false),
        ([1, 65_537, 1], false),
        ([1, 1, 65_537], false),
        ([0, 1, 1], false),
        ([1, 0, 1], false),
        ([1, 1, 0], false),
    ] {
        let output = walk_output(node(64), guide(), capacity);
        assert_eq!(
            closure_signature(&walk_input(guide(), 2), &output, 64).is_ok(),
            accepted,
            "{capacity:?}"
        );
    }
    for (edge_types, accepted) in [(16, true), (17, false), (0, false)] {
        let output = walk_output(node(64), guide(), [1, 1, 1]);
        let checked = closure_signature(&walk_input(guide(), edge_types), &output, 64);
        assert_eq!(checked.is_ok(), accepted, "{edge_types}");
    }
}

#[test]
fn node_atom_bytes_cannot_exceed_the_configured_read_byte_ceiling() {
    for (declared, ceiling, accepted) in [(64, 64, true), (65, 64, false), (0, 64, false)] {
        let output = walk_output(node(declared), guide(), [1, 1, 1]);
        let checked = closure_signature(&walk_input(guide(), 2), &output, ceiling);
        assert_eq!(checked.is_ok(), accepted, "{declared}");
    }
}

#[test]
fn a_selector_is_accepted_only_over_the_closure_node_and_guide() {
    let guided = Guided {
        node: node(64),
        guide: guide(),
    };
    assert!(selector_signature(&candidate(guide(), node(64)), &decision(guide()), &guided).is_ok());
    let other_guide = table(&[("left", word())]);
    for (candidate, decision) in [
        (candidate(other_guide.clone(), node(64)), decision(guide())),
        (candidate(guide(), node(32)), decision(guide())),
        (candidate(guide(), node(64)), decision(other_guide)),
    ] {
        assert!(selector_signature(&candidate, &decision, &guided).is_err());
    }
}

#[test]
fn a_selector_missing_a_decision_flag_is_rejected() {
    let guided = Guided {
        node: node(64),
        guide: guide(),
    };
    let partial = table(&[
        ("expand", Schema::Boolean),
        ("guide", guide()),
        ("sufficient", Schema::Boolean),
    ]);
    assert!(selector_signature(&candidate(guide(), node(64)), &partial, &guided).is_err());
}

#[test]
fn resource_selector_and_storage_failures_carry_three_authority_classes() {
    let class = |tag: &str| {
        CLOSURE_OUTCOMES
            .iter()
            .find_map(|(outcome, class)| (*outcome == tag).then_some(*class))
    };
    assert_eq!(
        class("echo.closure-work-bound-exceeded/v1"),
        Some("resourceFault")
    );
    assert_eq!(
        class("echo.closure-selector-insufficient/v1"),
        Some("domainMappable")
    );
    assert_eq!(
        class("echo.closure-storage-malformed/v1"),
        Some("integrityFault")
    );
}
