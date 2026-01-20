// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
//! BOAW Collapse/Merge Tests (ADR-0007 §9)
//!
//! Tests for multi-parent merge semantics:
//! - Commutative merge parent order invariance
//! - Canonical parent ordering for order-dependent merges
//! - Conflict artifact determinism

mod common;

// =============================================================================
// T6: Merge / Collapse
// =============================================================================

#[test]
#[ignore = "BOAW collapse/merge not yet implemented"]
fn t6_1_commutative_merge_is_parent_order_invariant() {
    // Given: same set of parents in different orders
    // Expect: identical merge commit hash for mergeable types
    //
    // For mergeable types (commutative+associative), parent order must not matter.
    unimplemented!(
        "Implement: create two parent commits with commutative attachment merges; \
         merge in both orders; commit_hash equal"
    );
}

#[test]
#[ignore = "BOAW collapse/merge not yet implemented"]
fn t6_2_order_dependent_merge_uses_canonical_parent_order() {
    // Given: order-dependent merge type
    // Expect: merge uses parent commit hash sort order and is stable
    //
    // If you support order-dependent merges, the engine must canonicalize
    // parent ordering by sorting parents by commit_hash, then folding.
    unimplemented!(
        "Implement: make an order-dependent merge type; \
         verify parent ordering by commit hash yields stable result"
    );
}

#[test]
#[ignore = "BOAW collapse/merge not yet implemented"]
fn t6_3_irreconcilable_conflicts_produce_deterministic_conflict_artifact() {
    // Given: two parents write different non-mergeable values to same key
    // Expect: merge yields conflict artifact (bytes + hash stable)
    //
    // When merge cannot resolve:
    // - Emit a conflict artifact attachment/object containing only:
    //   - parent commit hashes
    //   - statement/value hashes
    //   - type ids
    //   - policy hashes
    // No secrets, no raw sensitive bytes.
    unimplemented!(
        "Implement: two parents write different non-mergeable values; \
         merge yields deterministic conflict artifact"
    );
}

// =============================================================================
// Merge regimes (§9.2)
// =============================================================================

#[test]
#[ignore = "BOAW typed merge registry not yet implemented"]
fn merge_regime_crdt_like_is_preferred() {
    // Preferred: commutative + associative merges (CRDT-like) for mergeable types.
    // This test validates that types declaring MergeBehavior::Mergeable
    // can be merged without conflicts.
    unimplemented!(
        "Implement: create mergeable type with CRDT merge function; \
         verify multi-parent merge succeeds"
    );
}

#[test]
#[ignore = "BOAW typed merge registry not yet implemented"]
fn merge_regime_lww_with_canonical_order() {
    // Allowed: order-dependent merges only with canonical parent order.
    // LWW (Last Writer Wins) uses deterministic winner by canonical ordering key.
    unimplemented!(
        "Implement: create LWW type; \
         verify canonical ordering determines winner"
    );
}

// =============================================================================
// Presence vs Value (§9.3)
// =============================================================================

#[test]
#[ignore = "BOAW presence policy not yet implemented"]
fn presence_policy_delete_wins() {
    // For each key:
    // - Presence policy: delete-wins | add-wins | LWW
    // - Default: delete-wins for reachability
    //
    // Given: one parent deletes key, another keeps it
    // Expect: merged view shows key deleted (default policy)
    unimplemented!(
        "Implement: one parent deletes node, one keeps; \
         verify delete-wins policy applies"
    );
}

#[test]
#[ignore = "BOAW presence policy not yet implemented"]
fn presence_policy_add_wins() {
    // Given: policy set to add-wins for a specific attachment type
    // Expect: if any parent has the key, merged view has it
    unimplemented!(
        "Implement: configure add-wins policy; \
         verify key survives if any parent has it"
    );
}

// =============================================================================
// Conflict artifacts (§9.4)
// =============================================================================

#[test]
#[ignore = "BOAW conflict artifacts not yet implemented"]
fn conflict_artifact_is_first_class_and_deterministic() {
    // Conflict artifacts are first-class, deterministic, safe.
    // They contain only:
    // - parent commit hashes
    // - statement/value hashes
    // - type ids
    // - policy hashes
    //
    // Multiple runs with same inputs must produce identical artifact bytes.
    unimplemented!(
        "Implement: create conflict scenario; \
         verify artifact is deterministic across multiple runs"
    );
}

#[test]
#[ignore = "BOAW conflict artifacts not yet implemented"]
fn conflict_artifact_contains_no_secrets() {
    // Verify that conflict artifacts never contain raw sensitive bytes.
    // Only hashes and metadata.
    unimplemented!(
        "Implement: create conflict with sensitive data; \
         verify artifact contains only hashes, no raw bytes"
    );
}
