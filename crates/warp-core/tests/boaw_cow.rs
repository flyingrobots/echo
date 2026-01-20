// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! BOAW COW Overlay Semantics Tests (ADR-0007 §5)
//!
//! Tests for Copy-on-Write semantics:
//! - Delete is unlink (view-only)
//! - Overlay precedence
//! - Structural sharing

mod common;

// =============================================================================
// T2: COW Overlay Semantics
// =============================================================================

// TODO(COW-001): Implement once Snapshot + Overlay view resolution exists.
#[test]
#[ignore = "COW-001: BOAW COW overlay not yet implemented"]
fn t2_1_delete_is_unlink_not_physical_delete() {
    // Given: base snapshot has node X; overlay deletes X
    // Expect: reads from overlay show X absent; base snapshot still contains X
    //
    // This test asserts the ADR contract:
    // - base snapshot remains unchanged
    // - overlay removes visibility in the next snapshot/view
    //
    // Wire this once you have Snapshot + Overlay view resolution.
    todo!(
        "COW-001: build base with node X, apply overlay delete, \
         assert view hides X but base still has X"
    );
}

// TODO(COW-002): Implement once DeltaView wiring exists.
#[test]
#[ignore = "COW-002: BOAW COW overlay not yet implemented"]
fn t2_2_overlay_wins_over_base_reads() {
    // Given: base has attachment A; overlay sets A to new value
    // Expect: read yields overlay value; commit includes overlay value
    todo!(
        "COW-002: base has attachment A; overlay sets A; \
         reads return overlay; commit includes overlay"
    );
}

// TODO(COW-003): Implement once segment-level COW optimization exists.
#[test]
#[ignore = "COW-003: BOAW COW overlay not yet implemented"]
fn t2_3_structural_sharing_reuses_unchanged_segments() {
    // Given: commit changes only one node attachment
    // Expect: only the affected segments differ; unchanged segments reused
    //
    // - Build commit C0
    // - Build commit C1 with one small change
    // - Compare segment manifests: most hashes identical
    //
    // NOTE: This test is for future segment-level COW optimization.
    // Per ADR-0007 migration plan, skip until god test passes.
    todo!(
        "COW-003: commit C0 then C1 with tiny change; \
         verify segment manifest reuses most segments"
    );
}

// =============================================================================
// Reachable-only semantics (§5.4 - COW view)
// NOTE: This test uses COW naming (t2_4) to stay with COW tests.
// If boaw_snapshot.rs is added later, consider moving this test there as t1_3.
// =============================================================================

// TODO(COW-004): Implement once reachability harness is wired.
#[test]
#[ignore = "COW-004: BOAW reachability not yet wired to harness"]
fn t2_4_reachable_only_semantics() {
    // Given: unreachable node/edge exists in object store but not reachable from root
    // Expect: snapshot excludes it; state_root unchanged if only unreachable changes
    //
    // - Add unreachable objects to CAS (or builder inputs)
    // - Ensure snapshot excludes them
    // - Ensure state_root ignores them
    todo!(
        "COW-004: add unreachable node, verify snapshot excludes it, \
         verify state_root unchanged"
    );
}

// =============================================================================
// View resolution during execution
// =============================================================================

// TODO(COW-005): Implement once DeltaView is implemented.
#[test]
#[ignore = "COW-005: BOAW DeltaView not yet implemented"]
fn delta_view_resolves_overlay_then_base() {
    // Given: base snapshot + in-progress TickDelta with some writes
    // Expect: DeltaView::get() returns overlay value if present, else base value
    //
    // This validates the read path during execution before commit.
    todo!(
        "COW-005: create DeltaView over snapshot + delta, \
         verify reads resolve correctly"
    );
}

// TODO(COW-006): Implement once DeltaView tombstone handling exists.
#[test]
#[ignore = "COW-006: BOAW DeltaView not yet implemented"]
fn delta_view_handles_tombstones() {
    // Given: base has node X; delta contains DeleteNode(X)
    // Expect: DeltaView::get_node(X) returns None
    todo!(
        "COW-006: create DeltaView with delete tombstone, \
         verify node appears absent"
    );
}
