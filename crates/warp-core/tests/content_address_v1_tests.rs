// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! ContentAddressV1 conformance: independent vectors and mutation postcards.
//!
//! Expected identities come from preimages assembled field by field in
//! `common/content_address_preimage.rs`, never from the production module, and
//! the main vectors are also frozen as hex. Each postcard is a deliberate
//! mutation of a vector with the outcome the law requires: the same identity,
//! a different identity, or a refusal.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
#[path = "common/content_address_preimage.rs"]
mod preimage;

use preimage::{attachment, content_edge, hex, node, slot_edge, Preimage};
use warp_core::content_address_v1::{
    attachment_digest, content_edge_id, content_node_id, slot_edge_id, ContentAddressError,
    OutgoingEdge,
};
use warp_core::{NodeId, TypeId, WarpId};

/// Outgoing edges in input order, the same edges sorted, and the frozen id.
type NodeCase<'a> = (&'a [OutgoingEdge], &'a [([u8; 32], [u8; 32])], &'a str);

const WARP: [u8; 32] = [3; 32];
const NODE_TYPE: [u8; 32] = [7; 32];
const ATOM_TYPE: [u8; 32] = [2; 32];
const EDGE_A: [u8; 32] = [6; 32];
const EDGE_B: [u8; 32] = [8; 32];
const TO_LOW: [u8; 32] = [4; 32];
const TO_HIGH: [u8; 32] = [5; 32];

fn edge(edge_type: [u8; 32], to: [u8; 32]) -> OutgoingEdge {
    OutgoingEdge {
        edge_type: TypeId(edge_type),
        to: NodeId(to),
    }
}

fn node_id(atom: &[u8], edges: &[OutgoingEdge]) -> Result<[u8; 32], ContentAddressError> {
    content_node_id(
        WarpId(WARP),
        TypeId(NODE_TYPE),
        TypeId(ATOM_TYPE),
        atom,
        edges,
    )
    .map(|id| id.0)
}

fn expected_node(atom: &[u8], sorted_edges: &[([u8; 32], [u8; 32])]) -> Preimage {
    node(
        WARP,
        NODE_TYPE,
        attachment(ATOM_TYPE, atom).hash(),
        sorted_edges,
    )
}

fn unhex(text: &str) -> [u8; 32] {
    let mut out = [0; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[2 * i..2 * i + 2], 16).expect("hex");
    }
    out
}

// Frozen vectors, derived from the field-by-field preimages above. The
// zero-edge node and the slot edge equal identities Echo admitted before the law
// moved into its own module (see the derivation regression test).
const ATTACHMENT_NEW: &str = "a658effc54ff1c915e9544c327aafaeb65d00a83e99d0c0c9848f3c2aa3bbafc";
const NODE_ZERO_EDGES: &str = "127c0d47351ee88620bec24603a1b8949610d792f06f913227308ee5c7e47f90";
const NODE_ONE_EDGE: &str = "322afec5e32a48d3ab373c0b7efbf614bda7cf391d01b726a72c64c753fc5e10";
const NODE_TWO_EDGES: &str = "b64b6a4157a7a819c217a9e39a927ce71d13aeeb6d161b4ad266e3489036c6af";
const CONTENT_EDGE: &str = "76c81ad538b8c6e18a6a93039137a13ad7dc6608d6ba3ce895634fc004787117";
const SLOT_EDGE: &str = "bd913d145bb0da61dc572cf56a5a4abd442bf295b7020c32aa1de85a3dd70aa2";

#[test]
fn attachment_digest_matches_its_independent_preimage_and_frozen_vector() {
    let expected = attachment(ATOM_TYPE, b"new");
    expected.assert_identity(attachment_digest(TypeId(ATOM_TYPE), b"new"), "attachment");
    assert_eq!(hex(&expected.hash()), ATTACHMENT_NEW);
    // The empty atom still frames its zero length.
    attachment(ATOM_TYPE, b"")
        .assert_identity(attachment_digest(TypeId(ATOM_TYPE), b""), "empty atom");
}

#[test]
fn node_ids_with_zero_one_and_two_edges_match_independent_preimages() {
    let cases: [NodeCase<'_>; 3] = [
        (&[], &[], NODE_ZERO_EDGES),
        (
            &[edge(EDGE_A, TO_HIGH)],
            &[(EDGE_A, TO_HIGH)],
            NODE_ONE_EDGE,
        ),
        (
            &[edge(EDGE_B, TO_LOW), edge(EDGE_A, TO_HIGH)],
            &[(EDGE_A, TO_HIGH), (EDGE_B, TO_LOW)],
            NODE_TWO_EDGES,
        ),
    ];
    for (edges, sorted, frozen) in cases {
        let expected = expected_node(b"new", sorted);
        expected.assert_identity(node_id(b"new", edges).expect("node"), "node");
        assert_eq!(hex(&expected.hash()), frozen, "{}", expected.describe());
    }
}

#[test]
fn edge_ids_match_independent_preimages_and_frozen_vectors() {
    let content = content_edge(WARP, TO_LOW, EDGE_A, TO_HIGH);
    content.assert_identity(
        content_edge_id(
            WarpId(WARP),
            NodeId(TO_LOW),
            TypeId(EDGE_A),
            NodeId(TO_HIGH),
        )
        .0,
        "content edge",
    );
    assert_eq!(hex(&content.hash()), CONTENT_EDGE);
    let slot = slot_edge(WARP, TO_LOW, EDGE_A);
    slot.assert_identity(
        slot_edge_id(WarpId(WARP), NodeId(TO_LOW), TypeId(EDGE_A)).0,
        "slot edge",
    );
    assert_eq!(hex(&slot.hash()), SLOT_EDGE);
}

#[test]
fn retargeting_a_slot_keeps_its_id_while_a_content_edge_id_follows_its_target() {
    let slot = |to: [u8; 32]| {
        let _ = to; // A slot edge's id never reads its target.
        slot_edge_id(WarpId(WARP), NodeId(TO_LOW), TypeId(EDGE_A))
    };
    assert_eq!(slot(TO_HIGH), slot([9; 32]));
    let content = |to| content_edge_id(WarpId(WARP), NodeId(TO_LOW), TypeId(EDGE_A), NodeId(to));
    assert_ne!(content(TO_HIGH), content([9; 32]));
    // The two laws never coincide for the same source and type.
    assert_ne!(slot(TO_HIGH).0, content(TO_HIGH).0);
}

/// Postcards for one node vector: each mutation and the outcome it must have.
#[test]
fn node_identity_postcards() {
    let edges = [edge(EDGE_A, TO_HIGH), edge(EDGE_B, TO_LOW)];
    let sorted = [(EDGE_A, TO_HIGH), (EDGE_B, TO_LOW)];
    let base = node_id(b"new", &edges).expect("base");
    expected_node(b"new", &sorted).assert_identity(base, "base");

    // Same identity: edges are canonicalized, so input order does not matter.
    assert_eq!(
        node_id(b"new", &[edge(EDGE_B, TO_LOW), edge(EDGE_A, TO_HIGH)]).expect("reordered"),
        base,
        "reordering outgoing edges before canonicalization keeps the identity"
    );

    // Different identity: each preimage mutation below must not reproduce it.
    let mut no_terminator = expected_node(b"new", &sorted);
    no_terminator.fields[0].bytes = b"echo.bridge.node-id.v1".to_vec();
    let mut big_endian_count = expected_node(b"new", &sorted);
    big_endian_count.fields[4].bytes = 2u64.to_be_bytes().to_vec();
    let unsorted = expected_node(b"new", &[(EDGE_B, TO_LOW), (EDGE_A, TO_HIGH)]);
    let mut attachment_no_terminator = attachment(ATOM_TYPE, b"new");
    attachment_no_terminator.fields[0].bytes = b"echo.bridge.attachment-value.v1".to_vec();
    let mut attachment_descend_tag = attachment(ATOM_TYPE, b"new");
    attachment_descend_tag.fields[1].bytes = vec![1];
    let mut attachment_big_endian = attachment(ATOM_TYPE, b"new");
    attachment_big_endian.fields[3].bytes = 3u64.to_be_bytes().to_vec();
    let mutated = [
        (
            "node domain without its NUL terminator",
            no_terminator.hash(),
        ),
        ("edge count big-endian", big_endian_count.hash()),
        ("edges hashed in input order", unsorted.hash()),
        (
            "attachment domain without its NUL terminator",
            node(WARP, NODE_TYPE, attachment_no_terminator.hash(), &sorted).hash(),
        ),
        (
            "attachment tag 1 (descend) instead of 0 (atom)",
            node(WARP, NODE_TYPE, attachment_descend_tag.hash(), &sorted).hash(),
        ),
        (
            "atom length big-endian",
            node(WARP, NODE_TYPE, attachment_big_endian.hash(), &sorted).hash(),
        ),
        (
            "node type and atom type swapped",
            content_node_id(
                WarpId(WARP),
                TypeId(ATOM_TYPE),
                TypeId(NODE_TYPE),
                b"new",
                &edges,
            )
            .expect("swapped")
            .0,
        ),
        (
            "another warp",
            content_node_id(
                WarpId([9; 32]),
                TypeId(NODE_TYPE),
                TypeId(ATOM_TYPE),
                b"new",
                &edges,
            )
            .expect("warp")
            .0,
        ),
        (
            "one more atom byte",
            node_id(b"new\0", &edges).expect("longer"),
        ),
        (
            "an edge dropped",
            node_id(b"new", &edges[..1]).expect("fewer"),
        ),
        (
            "an edge retargeted",
            node_id(b"new", &[edge(EDGE_A, [9; 32]), edge(EDGE_B, TO_LOW)]).expect("retarget"),
        ),
        (
            "edge type and target roles swapped",
            node_id(b"new", &[edge(TO_HIGH, EDGE_A), edge(EDGE_B, TO_LOW)]).expect("roles"),
        ),
    ];
    for (mutation, id) in mutated {
        assert_ne!(id, base, "{mutation} must change the identity");
    }

    // Refused: the same (edge type, to) twice. Admission can never accept a
    // content node with a duplicate outgoing tuple, so the constructor refuses
    // instead of deduplicating into an identity that names another edge set.
    assert_eq!(
        node_id(b"new", &[edge(EDGE_A, TO_HIGH), edge(EDGE_A, TO_HIGH)]),
        Err(ContentAddressError::DuplicateOutgoingEdge {
            edge_type: TypeId(EDGE_A),
            to: NodeId(TO_HIGH),
        })
    );
    // Not duplicates: one type to two targets, or two types to one target.
    assert!(node_id(b"new", &[edge(EDGE_A, TO_HIGH), edge(EDGE_A, TO_LOW)]).is_ok());
    assert!(node_id(b"new", &[edge(EDGE_A, TO_HIGH), edge(EDGE_B, TO_HIGH)]).is_ok());
}

#[test]
fn diagnostics_name_fields_and_hide_atom_bytes_unless_asked() {
    let secret = b"private document text";
    let summary = attachment(ATOM_TYPE, secret).describe();
    assert!(summary.contains("atom bytes [21 bytes] blake3 "));
    assert!(!summary.contains(&hex(secret)));
    assert!(summary.contains("atom type [32 bytes] 0202"));
    let shown = attachment(ATOM_TYPE, secret).show_atom().describe();
    assert!(shown.contains(&hex(secret)));
    let report = std::panic::catch_unwind(|| {
        expected_node(secret, &[]).assert_identity([0; 32], "deliberate mismatch");
    })
    .expect_err("mismatch panics");
    let message = report.downcast_ref::<String>().expect("message");
    for name in [
        "node domain",
        "warp",
        "node type",
        "attachment digest",
        "edge count (u64 LE)",
    ] {
        assert!(message.contains(name), "{name} in {message}");
    }
}

#[test]
fn frozen_vectors_are_well_formed() {
    for frozen in [
        ATTACHMENT_NEW,
        NODE_ZERO_EDGES,
        NODE_ONE_EDGE,
        NODE_TWO_EDGES,
        CONTENT_EDGE,
        SLOT_EDGE,
    ] {
        assert_eq!(hex(&unhex(frozen)), frozen);
    }
}

#[test]
fn edge_input_order_never_changes_the_id_and_duplicates_refuse_in_any_order() {
    // Ascending input is hashed in place; any other order is sorted first.
    let a = edge(EDGE_A, TO_LOW);
    let b = edge(EDGE_A, TO_HIGH);
    let c = edge(EDGE_B, TO_LOW);
    let expected = expected_node(
        b"new",
        &[(EDGE_A, TO_LOW), (EDGE_A, TO_HIGH), (EDGE_B, TO_LOW)],
    )
    .hash();
    for order in [[a, b, c], [c, b, a], [b, a, c], [a, c, b]] {
        assert_eq!(node_id(b"new", &order).expect("distinct edges"), expected);
    }
    for duplicated in [vec![a, a], vec![a, a, b], vec![a, b, a], vec![b, a, a]] {
        assert_eq!(
            node_id(b"new", &duplicated),
            Err(ContentAddressError::DuplicateOutgoingEdge {
                edge_type: TypeId(EDGE_A),
                to: NodeId(TO_LOW),
            })
        );
    }
}
