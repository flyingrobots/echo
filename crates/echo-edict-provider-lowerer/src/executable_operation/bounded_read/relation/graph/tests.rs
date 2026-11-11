// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! Signature laws of the two graph reads, over hand-built structural types.
use super::*;
use std::collections::BTreeMap;

fn fields(entries: &[(&str, ReadType)]) -> ReadType {
    ReadType::Record(
        entries
            .iter()
            .map(|(name, ty)| ((*name).to_owned(), ty.clone()))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn list_of(item: ReadType, max: u64) -> ReadType {
    ReadType::List(Some(Box::new(item)), max)
}

fn edge_input() -> ReadType {
    fields(&[("warpId", ID), ("nodeId", ID), ("edgeType", ID)])
}

fn edge_output(max: u64) -> ReadType {
    list_of(fields(&[("edgeId", ID), ("toId", ID)]), max)
}

fn guide() -> ReadType {
    fields(&[("offset", WORD), ("follow", ReadType::Bool)])
}

fn node(max: u64) -> ReadType {
    fields(&[
        ("nodeId", ID),
        ("nodeType", ID),
        ("atomType", ReadType::Bytes(0, 32)),
        ("atomBytes", ReadType::Bytes(0, max)),
    ])
}

fn closure_input(guide: ReadType, edge_types: u64) -> ReadType {
    fields(&[
        ("warpId", ID),
        ("nodeId", ID),
        ("edgeTypes", list_of(ID, edge_types)),
        ("maxNodes", WORD),
        ("maxDepth", WORD),
        ("guide", guide),
    ])
}

fn closure_output(node: ReadType, guide: ReadType, capacity: [u64; 3]) -> ReadType {
    fields(&[
        ("nodes", list_of(node, capacity[0])),
        (
            "edges",
            list_of(
                fields(&[
                    ("from", WORD),
                    ("slot", WORD),
                    ("edgeId", ID),
                    ("node", WORD),
                ]),
                capacity[1],
            ),
        ),
        (
            "occurrences",
            list_of(
                fields(&[
                    ("node", WORD),
                    ("edge", WORD),
                    ("depth", WORD),
                    ("guide", guide),
                ]),
                capacity[2],
            ),
        ),
    ])
}

fn candidate(guide: ReadType, node: ReadType) -> ReadType {
    fields(&[
        ("guide", guide),
        ("depth", WORD),
        ("slot", WORD),
        ("parent", node.clone()),
        ("child", node),
    ])
}

fn decision(guide: ReadType) -> ReadType {
    fields(&[
        ("expand", ReadType::Bool),
        ("guide", guide),
        ("wellFormed", ReadType::Bool),
        ("sufficient", ReadType::Bool),
    ])
}

#[test]
fn an_edge_read_has_exactly_the_typed_range_signature() {
    assert!(edge_read(&edge_input(), &edge_output(8)).is_ok());
    // A nominal identifier is still a 32-byte identifier.
    let nominal = ReadType::Nominal("app@1.Edge".to_owned(), Box::new(ID));
    let named = fields(&[("warpId", ID), ("nodeId", ID), ("edgeType", nominal)]);
    assert!(edge_read(&named, &edge_output(8)).is_ok());
}

#[test]
fn an_edge_read_refuses_a_changed_address_or_row() {
    let extra = fields(&[
        ("warpId", ID),
        ("nodeId", ID),
        ("edgeType", ID),
        ("typeId", ID),
    ]);
    let short = fields(&[("warpId", ID), ("nodeId", ID)]);
    let narrow = fields(&[
        ("warpId", ID),
        ("nodeId", ID),
        ("edgeType", ReadType::Bytes(0, 32)),
    ]);
    for input in [extra, short, narrow] {
        assert!(edge_read(&input, &edge_output(8)).is_err());
    }
    let row = list_of(fields(&[("edgeId", ID), ("to", ID)]), 8);
    let untyped = ReadType::List(None, 8);
    for output in [row, untyped, ID] {
        assert!(edge_read(&edge_input(), &output).is_err());
    }
}

#[test]
fn an_edge_list_is_admitted_at_its_ceiling_and_refused_empty_or_above() {
    for (max, admitted) in [(1, true), (65_536, true), (0, false), (65_537, false)] {
        assert_eq!(
            edge_read(&edge_input(), &edge_output(max)).is_ok(),
            admitted,
            "{max}"
        );
    }
}

#[test]
fn a_closure_read_shares_one_node_and_guide_type_with_its_occurrences() {
    let input = closure_input(guide(), 2);
    let output = closure_output(node(64), guide(), [8, 16, 16]);
    let (node, guide) = (node(64), guide());
    // Eight nodes, plus two edge types for each of sixteen occurrences.
    assert_eq!(
        closure_read(&input, &output, 64),
        Ok(ClosureTypes {
            node: &node,
            guide: &guide,
            attempts: 8 + 16 * 2,
        })
    );
}

#[test]
fn a_closure_read_refuses_a_guide_its_occurrences_do_not_carry() {
    let other = fields(&[("offset", WORD)]);
    let output = closure_output(node(64), other, [8, 16, 16]);
    assert!(closure_read(&closure_input(guide(), 2), &output, 64).is_err());
}

#[test]
fn closure_capacities_are_admitted_at_each_ceiling_and_refused_one_above() {
    let input = closure_input(guide(), 2);
    for (capacity, admitted) in [
        ([4096, 65_536, 65_536], true),
        ([4097, 65_536, 65_536], false),
        ([4096, 65_537, 65_536], false),
        ([4096, 65_536, 65_537], false),
        ([0, 1, 1], false),
        ([1, 0, 1], false),
        ([1, 1, 0], false),
    ] {
        let output = closure_output(node(64), guide(), capacity);
        assert_eq!(
            closure_read(&input, &output, 64).is_ok(),
            admitted,
            "{capacity:?}"
        );
    }
    for (edge_types, admitted) in [(16, true), (17, false), (0, false)] {
        let input = closure_input(guide(), edge_types);
        let output = closure_output(node(64), guide(), [1, 1, 1]);
        assert_eq!(closure_read(&input, &output, 64).is_ok(), admitted);
    }
}

#[test]
fn a_node_row_cannot_declare_more_atom_bytes_than_the_read_byte_ceiling() {
    let input = closure_input(guide(), 2);
    for (declared, ceiling, admitted) in [(64, 64, true), (65, 64, false), (0, 64, false)] {
        let output = closure_output(node(declared), guide(), [1, 1, 1]);
        assert_eq!(
            closure_read(&input, &output, ceiling).is_ok(),
            admitted,
            "{declared}"
        );
    }
}

#[test]
fn a_closure_read_refuses_changed_row_shapes() {
    let input = closure_input(guide(), 2);
    let renamed = fields(&[
        ("nodeId", ID),
        ("nodeType", ID),
        ("atomType", ReadType::Bytes(0, 32)),
        ("atom", ReadType::Bytes(0, 64)),
    ]);
    let wide_type = fields(&[
        ("nodeId", ID),
        ("nodeType", ID),
        ("atomType", ID),
        ("atomBytes", ReadType::Bytes(0, 64)),
    ]);
    for node in [renamed, wide_type] {
        let output = closure_output(node, guide(), [1, 1, 1]);
        assert!(closure_read(&input, &output, 64).is_err());
    }
    let no_bounds = fields(&[
        ("warpId", ID),
        ("nodeId", ID),
        ("edgeTypes", list_of(ID, 2)),
        ("guide", guide()),
    ]);
    let output = closure_output(node(64), guide(), [1, 1, 1]);
    assert!(closure_read(&no_bounds, &output, 64).is_err());
}

#[test]
fn the_selector_decides_over_the_closure_node_and_guide_types() {
    let (node, guide) = (node(64), guide());
    let closure = ClosureTypes {
        node: &node,
        guide: &guide,
        attempts: 0,
    };
    let accepted = selector_conforms(
        &candidate(guide.clone(), node.clone()),
        &decision(guide.clone()),
        &closure,
    );
    assert!(accepted.is_ok());
}

#[test]
fn a_selector_over_another_node_or_guide_type_is_refused() {
    let (node, guide) = (node(64), guide());
    let closure = ClosureTypes {
        node: &node,
        guide: &guide,
        attempts: 0,
    };
    let foreign_guide = fields(&[("offset", WORD)]);
    let foreign_node = self::node(32);
    let cases = [
        (
            candidate(foreign_guide.clone(), node.clone()),
            decision(guide.clone()),
        ),
        (
            candidate(guide.clone(), foreign_node),
            decision(guide.clone()),
        ),
        (
            candidate(guide.clone(), node.clone()),
            decision(foreign_guide),
        ),
    ];
    for (candidate, decision) in cases {
        assert!(selector_conforms(&candidate, &decision, &closure).is_err());
    }
}

#[test]
fn a_selector_must_return_every_decision_flag() {
    let (node, guide) = (node(64), guide());
    let closure = ClosureTypes {
        node: &node,
        guide: &guide,
        attempts: 0,
    };
    let silent = fields(&[
        ("expand", ReadType::Bool),
        ("guide", guide.clone()),
        ("wellFormed", ReadType::Bool),
    ]);
    let numeric = fields(&[
        ("expand", WORD),
        ("guide", guide.clone()),
        ("wellFormed", ReadType::Bool),
        ("sufficient", ReadType::Bool),
    ]);
    for decision in [silent, numeric] {
        let candidate = candidate(guide.clone(), node.clone());
        assert!(selector_conforms(&candidate, &decision, &closure).is_err());
    }
}

#[test]
fn the_three_failure_kinds_of_a_closure_keep_distinct_authority_classes() {
    let class = |tag: &str| {
        CLOSURE_FAILURES
            .iter()
            .find_map(|(failure, class)| (*failure == tag).then_some(*class))
    };
    assert_eq!(
        class("echo.closure-node-bound-exceeded/v1"),
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
