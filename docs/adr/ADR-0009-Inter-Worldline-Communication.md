<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# ADR-0009: Inter-Worldline Communication, Frontier Transport, and Conflict Policy

- **Status:** Accepted
- **Date:** 2026-03-09
- **Depends on:** ADR-0008 (Worldline Runtime Model)
- **Theoretical basis:** WARP Paper Series (Papers I–V), unpublished.

## Context

ADR-0008 formalizes worldlines, writer/reader heads, and the SuperTick
scheduler for local execution. It deliberately stops at the boundary of a
single scheduler. This ADR addresses the next question: what happens when
worldlines need to communicate — whether across heads on the same machine,
across warps, or eventually across machines?

Three scenarios drive this:

1. **Multi-warp operations.** Portal and instance ops in Echo's graph model
   span multiple warps. Replay of these operations requires a communication
   model between the worldlines hosting those warps.
2. **Gameplay mechanics.** Ghost actors, branch-and-compare puzzles, and
   speculative execution all involve worldlines that diverge and later need
   to exchange information or compare outcomes.
3. **Future distributed execution.** If worldlines eventually span machines
   (Continuum-style systems), the communication model must scale from local
   message passing to network replication without changing the semantic
   contract.

The WARP paper series develops the formal machinery for this problem:
footprint-based commutation, frontier-relative patches, suffix transport,
and observer geometry. This ADR captures the architectural decisions derived
from that work, without reproducing the formal treatment.

### The replication problem in one paragraph

Suppose worldline A is at local tick 500, and worldline B sends a patch
saying "I performed an operation at my local tick 423." The naive
interpretation is historical insertion: rewind to a common point, insert the
remote action, replay forward. This is pathological — it invalidates
downstream hashes, forces resynchronisation from old checkpoints, and turns
latency into replay storms. The right abstraction is a _frontier-relative
patch_: "here is a patch based on frontier F; decide whether it commutes
with your unseen suffix since F." If it commutes, transport it to the tip
and append. No rewind. No rebase.

## Decision

### 1) Worldlines communicate by message passing only

Worldlines interact exclusively through intents and messages admitted via
deterministic ingress. There is no shared mutable state across worldline
boundaries.

This preserves:

- **Causal isolation.** Each worldline's provenance is self-contained.
- **Replay integrity.** Replaying a worldline requires only its own
  provenance log plus the messages it received.
- **Debugging clarity.** Cross-worldline interactions are visible as
  discrete events in the provenance DAG, not hidden shared-state mutations.

Cross-worldline messages are themselves intents: content-addressed,
capability-checked, and admitted through the receiver's deterministic
ingress. The receiver's provenance records the message as a causal
dependency.

### 2) Chronos is local — network patches are frontier-relative

A sender's local tick number is not a network insertion point. It is
meaningful only within that sender's Chronos line. The network-level causal
datum is the sender's **frontier** (or version vector), not a tick index.

A network patch carries:

- **Operation identity** — deterministic, content-addressed.
- **Base frontier / version vector** — the sender's causal context at
  authoring time.
- **Payload** — the replayable patch body.
- **Footprint** — reads, writes, deletes, and preserved anchors.
- **Precondition witness** — digest of read versions or anchor versions,
  protecting against stale-read application.
- **Optional audit metadata** — receipt hash, transport proof, state root
  hint, signature.

The receiver uses the base frontier to compute a common frontier and
determine what local work the sender has not yet seen.

### 3) Suffix transport is the replication primitive, not rebase

When a receiver gets a frontier-relative patch, it asks:

> Given my current state and my unseen local suffix since the sender's
> frontier, does this remote patch commute with that suffix?

If the patch is **independent** of every element in the unseen suffix
(no footprint interference), it can be **transported** to the current tip
and appended as a merge tick. No replay from the common frontier is needed.
Accepted history is never rewritten.

Rebase survives only as:

- A **debugging tool** — answering counterfactual "what if" questions.
- A **branch constructor** — producing derived histories for inspection.
- A **compression/explanation mechanism** — normalising a provenance DAG
  into a linear narrative.

It is not the live communication primitive.

### 4) Four-dimensional footprint interference

The interference test for cross-worldline patches considers four footprint
components:

- **Reads** — objects or fields whose values were consulted.
- **Writes** — objects or fields modified by the patch.
- **Deletes** — identities or structures removed.
- **Anchors** — preserved identities the patch assumes continue to exist,
  even if not written.

Two patches **interfere** if any of the following holds:

- Either patch deletes something the other uses (reads, writes, or anchors).
- Either patch writes something the other reads or writes.

Write-write disjointness alone is insufficient. A patch that writes
`node:7.color` may be invalidated by a concurrent write to `node:7.type`
if the first patch read `type` as a guard. The read-set clauses make this
explicit.

This extends Echo's existing `in_slots` / `out_slots` footprint model to
the network with anchors and precondition witnesses.

### 5) State convergence and history convergence are separate concerns

Two worldlines (or replicas) may reach **isomorphic current states** via
different serialisations of commuting concurrent imports. If one hashes
linear log order, the history roots may differ even when the state roots
agree.

Design consequence: treat **state convergence** as primary and **history
convergence** as a separate problem addressed by canonical batching or
DAG hashing. Do not conflate "same state" with "same log."

When history convergence is also required (audit, legal provenance,
blame), canonical batching — quotienting commuting concurrent imports into
a deterministic batch sorted by a common total key — restores it.

### 6) Explicit conflict surfacing over silent last-write-wins

When footprint interference blocks transport, the system MUST NOT silently
discard one side's intent. The default is explicit conflict handling.

The receiver's conflict policy stack, in order of preference:

1. **Datatype-specific join** — if the application datatype has a
   semantically justified algebraic join (CRDT-style), invoke it.
2. **Explicit conflict object** — surface a conflict carrying both sides'
   intent and witnesses. The conflict is a first-class provenance event.
3. **Retry** — reject the patch with a newer frontier, requesting the
   sender to recompute.
4. **Branch-replay** — construct a derived branch from the common frontier
   for offline or collaborative resolution.

CRDTs are appropriate exactly where the application datatype already
provides a semantically justified join. Outside those domains, the correct
default is explicit conflict policy, not silent blanket merging.

### 7) No general merge after fork

Fork is a runtime primitive (ADR-0008). Merge is a domain decision.

The system promises:

- **Compare** — inspect diverged worldlines side by side.
- **Discard** — abandon a speculative branch.
- **Selective typed collapse** — merge specific fields or subgraphs under
  explicit application-defined policy.

The system does NOT promise general merge. Arbitrary diverged worldlines
cannot be automatically reconciled without domain-specific merge semantics.
Attempting general merge leads to the same impossibility that plagues
distributed systems: you cannot simultaneously preserve both sides' intent,
maintain append-only history, and converge deterministically without
application-specific semantics.

### 8) Receiver-side cost management

For large unseen suffixes, literal scan of every local patch since the
common frontier is too expensive. The receiver SHOULD maintain hierarchical
footprint summaries — a balanced tree of range synopses over the suffix.

Each internal node stores aggregate footprint information (union of reads,
writes, deletes, anchors for all patches in the range). The receiver
descends only into ranges that _may_ interfere with the incoming patch,
pruning obviously disjoint ranges.

This gives O(log n + k) cost for finding k actual conflicts in a suffix of
length n. In the worst case of dense interference, cost remains O(n).

Cascading imports (a new merge tick extending the suffix while other
imports are pending) require only one additional transport check against the
new tick, not a full rescan.

## Required Invariants

### Communication

1. Cross-worldline state mutation occurs only through admitted intents.
2. No shared mutable state across worldline boundaries.
3. Cross-worldline messages are recorded as causal dependencies in the
   receiver's provenance DAG.
4. The receiver's import decision is deterministic given the same state,
   history, and incoming patch.

### Transport

1. Suffix transport is defined only when the incoming patch is independent
   of every element in the unseen suffix.
2. A transported patch appended at the tip produces the same committed state
   as replay from the common frontier (up to canonical isomorphism).
3. Transport is directional and witness-carrying. The receiver can verify
   the commutation without replaying from the common frontier.

### Conflict

1. Footprint interference blocks transport. The system MUST invoke an
   explicit conflict policy — never silently drop intent.
2. Conflict objects are first-class provenance events, not error conditions.
3. CRDT joins are used only where the datatype has a semantically justified
   join. They are not a universal fallback.

### Convergence

1. State convergence (identical state roots) is the primary correctness
   criterion for commuting imports.
2. History convergence (identical history roots) requires additional
   mechanism (canonical batching) and is not automatic.
3. State roots MUST agree after both sides import all commuting concurrent
   work. History roots MAY differ unless canonical batching is enabled.

## Implementation Considerations

### Near-term (local multi-worldline)

- Extend `WorldlineTickPatchV1` with explicit read footprint (`in_slots`
  already exists), anchor set, and precondition witness.
- Implement frontier-relative patch construction for multi-warp operations.
- Add conflict policy trait with `Accept`, `Join`, `Conflict`, `Retry`,
  and `Branch` variants.
- Wire inter-worldline intent delivery through the existing ingress path.

### Mid-term (formalized transport)

- Implement suffix transport as a library operation over the provenance
  store.
- Add hierarchical footprint summaries for suffix range pruning.
- Define canonical batching for history convergence where required.
- Extend the `ProvenanceStore` with merge tick and conflict object types.

### Later (distributed)

- Worldline ownership and authority records.
- Signed provenance exchange (receipt hashes, transport proofs).
- Causal readiness checks (request missing dependencies before import).
- Remote frontier advertisement and subscription.
- Cross-node causal tracing via `global_tick` correlation metadata.

## Relationship to the WARP Paper Series

This ADR derives its design principles from the WARP paper series
(Papers I–V). The formal proofs — network tick confluence, transport
squares, observer geometry, and rulial distance — live in those papers.
This ADR captures the _architectural decisions_ for Echo's implementation
without reproducing the formal treatment.

Key correspondences:

| ADR Concept                             | Paper Series Origin                                           |
| --------------------------------------- | ------------------------------------------------------------- |
| Frontier-relative patches               | Paper V: network patch definition                             |
| Four-dimensional footprint interference | Paper V: generalised interference relation                    |
| Suffix transport                        | Paper V: directional binary transport and suffix composition  |
| State vs. history convergence           | Paper V: state root vs. history root separation               |
| Explicit conflict surfacing             | Paper V: conflict inevitability and observer distance theorem |
| Observer geometry connection            | Paper IV: observers as functors, rulial distance              |
| Local tick confluence                   | Paper II: within-tick commuting conversions                   |
| Footprint discipline                    | Paper III: patch boundaries and causal cones                  |

## Test Requirements

| Category               | What to verify                                                                  |
| ---------------------- | ------------------------------------------------------------------------------- |
| Message isolation      | Cross-worldline mutation only through admitted intents; no shared state leakage |
| Transport correctness  | Transported patch at tip produces same state as replay from common frontier     |
| Interference detection | All four footprint dimensions checked; stale-read conflicts caught              |
| Conflict policy        | Interfering imports invoke explicit policy; no silent intent loss               |
| Convergence            | Commuting imports produce identical state roots regardless of arrival order     |
| Cascading imports      | New merge tick requires only incremental transport check for pending imports    |

## Consequences

- Inter-worldline communication has a clean, testable contract: intents
  in, receipts out, no shared mutation.
- The transport primitive eliminates replay storms for the common case of
  non-overlapping work across worldlines.
- Conflict handling is honest: when work interferes, both sides' intent is
  preserved in a first-class conflict object, not silently discarded.
- The architecture scales from local multi-warp to distributed replication
  without changing the semantic contract — only the transport medium
  changes.
- The separation of state convergence from history convergence gives
  implementors a clear choice: converge state cheaply, or pay for history
  convergence with canonical batching when audit/provenance demands it.

## Non-Goals

- This ADR does not specify wire encoding formats.
- This ADR does not prescribe specific CRDT implementations.
- This ADR does not require distributed execution in any near-term
  milestone.
- This ADR does not reproduce formal proofs from the WARP paper series.

## Document Governance

- Any change to the communication or transport invariants requires a
  dedicated design amendment PR.
- PRs introducing cross-worldline state sharing must reference this ADR
  and justify the exception.
- Conflict policy implementations must satisfy the explicit-surfacing
  invariant: no silent intent loss.

---

_Quod hodie facimus in aeternitate resonat._
