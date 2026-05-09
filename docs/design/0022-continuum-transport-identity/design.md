<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0022 - Continuum transport identity and import idempotence

_Lock the M027 import/loop-law decision: Echo's witnessed suffix model is the
source shape for Continuum transport, not a local adapter around a thinner
Continuum shell._

Legend: PLATFORM

Depends on:

- [0009 - Witnessed causal suffix export and import](../0009-witnessed-causal-suffix-sync/design.md)
- [0012 - Witnessed suffix posture canonicalization](../0012-witnessed-suffix-posture-canonicalization/design.md)
- [0018 - Echo Optics API Design](../0018-echo-optics-api-design/design.md)

## Decision

The runtime-boundary transport family is being shaped from Echo outward.

Recorded decisions for this task:

1. Continuum is free to change. Echo is the first serious consumer of this
   boundary, so the shared family should be corrected now instead of preserving
   an underspecified schema.
2. Echo's witnessed suffix nouns are promoted into Continuum's runtime-boundary
   family. The shared schema should name `WitnessedSuffixShell`,
   `CausalSuffixBundle`, `WitnessedSuffixAdmissionResponse`, and the
   `ImportOutcome` that wraps them.
3. Echo should consume the Continuum family explicitly once that schema matches
   Echo's evidence model.
4. If the current Continuum runtime-boundary shape conflicts with Echo's
   witnessed suffix/admission model, Continuum changes. Echo's causal evidence
   shape wins.
5. Transport identity and import idempotence are facts carried by typed
   evidence, not by summary strings, final-state hashes, host-time arrival
   order, or runtime-local Lamport clocks.

## Core Rule

Echo imports witnessed causal suffix bundles, not state.

Idempotence is shell equivalence under the retained causal evidence:

```text
same bundle identity
+ same source shell identity
+ same base and target frontiers
+ same witness basis
=> same import question
```

That rule is narrower than visible-state equality and wider than a local receipt
hash. A local receipt proves what this runtime did with a bundle. It is not the
portable identity of the bundle itself.

## Why This Exists

The older Continuum runtime-boundary schema had the right intent but a thinner
shape:

- `SuffixShell` carried frontier digests and counts.
- `ImportOutcome` carried a shared outcome kind and optional receipt reference.

That is not enough for M027. Echo needs to decide whether a repeated import is:

- the same bundle arriving again
- self-history returning through another runtime
- a support supplement for already-known evidence
- state-equivalent but witness-distinct history
- a lawful plurality
- a conflict
- an obstruction

Those cases cannot be collapsed into "same final state" or "same frontier
digest." OG-II's operational point applies directly here: state convergence does
not imply observer convergence. Two imports that materialize the same state can
still differ in provenance, intent, support path, or replayability.

## Canonical Echo Shape

Echo already names the minimum runtime shape in code:

```text
WitnessedSuffixShell {
    source_worldline_id,
    source_suffix_start_tick,
    source_suffix_end_tick,
    source_entries,
    boundary_witness,
    witness_digest,
    basis_report,
}

CausalSuffixBundle {
    base_frontier,
    target_frontier,
    source_suffix,
    bundle_digest,
}

ImportSuffixResult {
    bundle_digest,
    admission: WitnessedSuffixAdmissionResponse,
}

WitnessedSuffixAdmissionOutcome =
    Admitted { target_worldline_id, admitted_refs, basis_report }
  | Staged { staged_refs, basis_report }
  | Plural { candidate_refs, residual_posture, basis_report }
  | Conflict { reason, source_ref, evidence_digest, overlap_revalidation }
  | Obstructed { source_ref, residual_posture, evidence_digest }
```

The Continuum family should expose this meaning directly. It may add runtime
metadata around it, such as source runtime, target runtime, history family, or
lane labels, but it must not replace the core suffix evidence with a weaker
state/hash/count summary.

## Transport Identity

Transport identity has layers. They must not be confused:

| Layer               | Meaning                                      | May identify duplicates?    | Must not be used as         |
| ------------------- | -------------------------------------------- | --------------------------- | --------------------------- |
| Content hash        | Bytes in CAS or payload storage              | Exact byte reuse            | Causal-history identity     |
| Reading identity    | The question a retained reading answers      | Cached read reuse           | Import identity             |
| Bundle digest       | The witnessed suffix bundle question         | Exact import-shell reuse    | Local admission receipt     |
| Source shell digest | Compact source suffix evidence               | Shell equivalence           | Target-local tick identity  |
| Local receipt       | What this runtime admitted/staged/conflicted | Local audit and witness     | Portable source identity    |
| Local Lamport/tick  | Runtime-local ordering coordinate            | Local replay/order evidence | Cross-runtime duplicate key |

Lamport clocks and local ticks can affect local hashes. They therefore cannot be
the universal duplicate key for a transported suffix. They are scoped evidence,
not the shared import identity.

## Import Idempotence Law

When Echo receives a `CausalSuffixBundle`, the runtime must:

1. Verify bundle identity and source shell identity.
2. Resolve the target basis explicitly.
3. Compare the bundle against retained prior import outcomes.
4. Classify the import with typed posture.
5. Return a witnessed result without silently mutating current state.

Re-import of the exact same bundle should be idempotent. It may return the
prior import outcome or a new local receipt that points at the prior outcome,
but it must not create fake novelty.

Self-history returning through another runtime is not fresh remote work. It must
be classified as a loop or already-adjudicated import path, with evidence.

State-equivalent but witness-distinct imports are not duplicates. They may be
support supplements, alternate support paths, lawful plural history, conflicts,
or obstructions depending on evidence and policy.

## Continuum Runtime-Boundary Cut

Continuum's `continuum-runtime-boundary-family.graphql` should use Echo's
witnessed suffix model as the shared family cut:

- `ProvenanceRef`
- `SettlementBasisReport`
- `SettlementOverlapRevalidation`
- `WitnessedSuffixShell`
- `CausalSuffixBundle`
- `WitnessedSuffixAdmissionOutcome`
- `WitnessedSuffixAdmissionResponse`
- `ImportOutcome`

The old `SuffixShell` name is too vague for this boundary. If a compatibility
alias is needed later, it should be an adapter term. The family itself should
name the witnessed suffix shell and causal suffix bundle explicitly.

## M027 RED Targets

The next implementation slice should add RED tests before changing runtime
behavior:

1. Exact bundle re-import returns an idempotent import outcome, not fresh
   admission.
2. Self-history returning through a peer is classified as a loop or
   already-adjudicated import path.
3. Same visible state with different witness/source shell identity is not
   deduped by state hash alone.
4. Local tick or Lamport-like order fields do not participate in portable
   duplicate detection.
5. `ImportOutcome` preserves the nested
   `WitnessedSuffixAdmissionResponse.outcome` variant.
6. Obstruction and conflict evidence remain typed and deterministic.
7. Continuum-compiled runtime-boundary artifacts expose the same suffix nouns
   that Echo consumes.

## Non-Goals

This decision does not add:

- a sync daemon
- direct peer mutation
- last-writer-wins import
- host-time ordering
- materialized state exchange as transport truth
- a git-warp-first schema
- a GraphQL-first Echo runtime API

GraphQL is the authored family surface that Wesley compiles. Echo's runtime
truth remains witnessed causal admission and observation.
