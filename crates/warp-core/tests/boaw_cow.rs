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

#[test]
#[ignore = "BOAW COW overlay not yet implemented"]
fn t2_1_delete_is_unlink_not_physical_delete() {
    // Given: base snapshot has node X; overlay deletes X
    // Expect: reads from overlay show X absent; base snapshot still contains X
    //
    // This test asserts the ADR contract:
    // - base snapshot remains unchanged
    // - overlay removes visibility in the next snapshot/view
    //
    // Wire this once you have Snapshot + Overlay view resolution.
    unimplemented!(
        "Implement: build base with node X, apply overlay delete, \
         assert view hides X but base still has X"
    );
}

#[test]
#[ignore = "BOAW COW overlay not yet implemented"]
fn t2_2_overlay_wins_over_base_reads() {
    // Given: base has attachment A; overlay sets A to new value
    // Expect: read yields overlay value; commit includes overlay value
    unimplemented!(
        "Implement: base has attachment A; overlay sets A; \
         reads return overlay; commit includes overlay"
    );
}

#[test]
#[ignore = "BOAW COW overlay not yet implemented"]
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
    unimplemented!(
        "Implement: commit C0 then C1 with tiny change; \
         verify segment manifest reuses most segments"
    );
}

// =============================================================================
// Reachable-only semantics (§4.1)
// =============================================================================

#[test]
#[ignore = "BOAW reachability not yet wired to harness"]
fn t1_3_reachable_only_semantics() {
    // Given: unreachable node/edge exists in object store but not reachable from root
    // Expect: snapshot excludes it; state_root unchanged if only unreachable changes
    //
    // - Add unreachable objects to CAS (or builder inputs)
    // - Ensure snapshot excludes them
    // - Ensure state_root ignores them
    unimplemented!(
        "Implement: add unreachable node, verify snapshot excludes it, \
         verify state_root unchanged"
    );
}

// =============================================================================
// View resolution during execution
// =============================================================================

#[test]
#[ignore = "BOAW DeltaView not yet implemented"]
fn delta_view_resolves_overlay_then_base() {
    // Given: base snapshot + in-progress TickDelta with some writes
    // Expect: DeltaView::get() returns overlay value if present, else base value
    //
    // This validates the read path during execution before commit.
    unimplemented!(
        "Implement: create DeltaView over snapshot + delta, \
         verify reads resolve correctly"
    );
}

#[test]
#[ignore = "BOAW DeltaView not yet implemented"]
fn delta_view_handles_tombstones() {
    // Given: base has node X; delta contains DeleteNode(X)
    // Expect: DeltaView::get_node(X) returns None
    unimplemented!(
        "Implement: create DeltaView with delete tombstone, \
         verify node appears absent"
    );
}
