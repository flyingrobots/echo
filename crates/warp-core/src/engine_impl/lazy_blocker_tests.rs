// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
#![allow(clippy::expect_used)] // Test assertions may abort the witness.
//! On-demand blocker attribution matches the scan a tick used to run.
use super::*;
use crate::{EdgeRecord, Footprint, PatternGraph, TickDelta};

const WIDE: &str = "test/blame-wide";
const NARROW: &str = "test/blame-narrow";
const WRITES: &str = "test/blame-writes";
const READS: &str = "test/blame-reads";

/// Writes its scope, and writes or reads each node its scope points at.
fn wide_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut footprint = Footprint::default();
    footprint.n_write.insert_with_warp(warp_id, *scope);
    for edge in view.edges_from(scope) {
        if edge.ty == make_type_id(WRITES) {
            footprint.n_write.insert_with_warp(warp_id, edge.to);
        } else if edge.ty == make_type_id(READS) {
            footprint.n_read.insert_with_warp(warp_id, edge.to);
        }
    }
    footprint
}

/// Reads its scope and every node its scope points at.
fn narrow_footprint(view: GraphView<'_>, scope: &NodeId) -> Footprint {
    let warp_id = view.warp_id();
    let mut footprint = Footprint::default();
    footprint.n_read.insert_with_warp(warp_id, *scope);
    for edge in view.edges_from(scope) {
        footprint.n_read.insert_with_warp(warp_id, edge.to);
    }
    footprint
}

fn nothing(_: GraphView<'_>, _: &NodeId, _: &mut TickDelta) {}

fn rule(name: &'static str, footprint: fn(GraphView<'_>, &NodeId) -> Footprint) -> RewriteRule {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"rule:");
    hasher.update(name.as_bytes());
    RewriteRule {
        id: hasher.finalize().into(),
        name,
        left: PatternGraph { nodes: vec![] },
        matcher: |_, _| true,
        executor: RuleExecutor::legacy(nothing),
        compute_footprint: footprint,
        factor_mask: 0,
        conflict_policy: ConflictPolicy::Abort,
        join_fn: None,
    }
}

/// A small deterministic generator, so a failing seed can be replayed.
struct Draw(u64);

impl Draw {
    fn below(&mut self, bound: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % bound
    }
}

/// Builds a random contended tick, commits it and returns its rejection count
/// after checking that on-demand attribution equals the removed eager scan.
fn check_seed(seed: u64) -> usize {
    let mut draw = Draw(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let ty = make_type_id("test/blame-node");
    let root = make_node_id("test/blame-root");
    let mut store = GraphStore::default();
    store.insert_node(root, NodeRecord { ty });
    let groups: Vec<NodeId> = (0..=draw.below(5))
        .map(|group| make_node_id(&format!("test/blame-group-{group}")))
        .collect();
    for group in &groups {
        store.insert_node(*group, NodeRecord { ty });
    }
    let entities: Vec<NodeId> = (0..=draw.below(40))
        .map(|entity| make_node_id(&format!("test/blame-entity-{entity}")))
        .collect();
    for (entity_index, entity) in entities.iter().enumerate() {
        store.insert_node(*entity, NodeRecord { ty });
        for edge_index in 0..draw.below(4) {
            let group = groups[usize::try_from(draw.below(groups.len() as u64)).expect("index")];
            let kind = if draw.below(2) == 0 { WRITES } else { READS };
            store.insert_edge(
                *entity,
                EdgeRecord {
                    id: make_edge_id(&format!("test/blame-edge-{entity_index}-{edge_index}")),
                    from: *entity,
                    to: group,
                    ty: make_type_id(kind),
                },
            );
        }
    }
    let mut engine = Engine::new(store, root);
    engine
        .register_rule(rule(WIDE, wide_footprint))
        .expect("wide");
    engine
        .register_rule(rule(NARROW, narrow_footprint))
        .expect("narrow");
    let tx = engine.begin();
    for entity in &entities {
        if draw.below(3) != 0 {
            assert!(matches!(
                engine.apply(tx, WIDE, entity),
                Ok(ApplyResult::Applied)
            ));
        }
        if draw.below(3) == 0 {
            assert!(matches!(
                engine.apply(tx, NARROW, entity),
                Ok(ApplyResult::Applied)
            ));
        }
    }
    let state_before = engine.state.clone();
    let (_, receipt, _) = engine.commit_with_receipt(tx).expect("the tick commits");
    let eager = engine.eager_blockers_for_test.clone();
    assert_eq!(eager.len(), receipt.entries().len(), "seed {seed}");
    let lazy = engine
        .footprint_blockers(&state_before, &receipt)
        .expect("attribution is derivable");
    assert_eq!(lazy, eager, "seed {seed}: on-demand blockers differ");
    for (entry, blockers) in receipt.entries().iter().zip(&lazy) {
        if entry.disposition != TickReceiptDisposition::Applied {
            assert!(
                !blockers.is_empty(),
                "seed {seed}: a rejection has no blocker"
            );
        }
    }
    for index in 0..receipt.entries().len() {
        assert!(
            receipt.blocked_by(index).is_empty(),
            "seed {seed}: a tick no longer records blockers"
        );
    }
    receipt
        .entries()
        .iter()
        .filter(|entry| entry.disposition != TickReceiptDisposition::Applied)
        .count()
}

#[test]
fn on_demand_blockers_equal_the_removed_eager_scan() {
    let rejected: usize = (0..400).map(check_seed).sum();
    // The generator must actually produce contention, or the check is empty.
    assert!(
        rejected > 400,
        "only {rejected} rejections across all seeds"
    );
}
