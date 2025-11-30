<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Temporal Bridge Specification (Phase 0.5)

The Temporal Bridge (TB) is the service that moves events between branches in Echo’s timeline tree. It guarantees deterministic delivery, retro-branch creation, paradox prevention, and entropy bookkeeping.

---

## Overview

1. Sender enqueues a cross-branch `EventEnvelope` via Codex’s Baby (`emitCross`).
2. TB validates chronology, resolves target branch, and queues for delivery.
3. At commit or `timeline_flush`, TB delivers events to their target branch, creating retro branches if needed.
4. Paradox guard runs pre-checks using read/write sets; paradoxes are quarantined.
5. Delivery results update entropy metrics and causality graphs.

---

## Data Structures

```ts
interface BridgeQueueEntry {
  envelope: EventEnvelope;
  sourceBranch: KairosBranchId;
  bridgeSeq: number;            // monotonic per source branch
  dedupHash: string;            // envelopeHash or hash(envelope)
}

interface BridgeState {
  pending: Map<KairosBranchId, BridgeQueueEntry[]>;
  seen: Set<string>;            // dedup for exactly-once mode
  retroLog: RetroRecord[];
}

interface RetroRecord {
  eventId: string;
  fromBranch: KairosBranchId;
  retroBranch: KairosBranchId;
  lcaNodeId: NodeId;
}
```

---

## Event Lifecycle

### 1. Enqueue
- `emitCross(evt)` pushes `BridgeQueueEntry` into `pending[targetBranch]`.
- `bridgeSeq` increments per source branch.
- `dedupHash` computed using canonical event encoding.

### 2. Validation
For each entry:
1. Resolve target branch head `Hβ`.
2. If `evt.chronos < Hβ.chronos` → retro flow.
3. If `evt.chronos > Hβ.chronos` → queue for future tick.
4. Optional uniqueness: if `dedupHash` already in `seen`, drop (exactly-once mode).

### 3. Retro Delivery
If `evt.chronos < Hβ.chronos`:
1. Find lowest common ancestor node `L` between `Hβ` and tick `evt.chronos`.
2. Fork retro branch β′ from β at node `L`.
3. Rewrite `evt.kairos = β′`; record `RetroRecord` and `evt.metadata.retro = true`.
4. Queue event in `pending[β′]`.

### 4. Paradox Pre-check
Before delivery, TB compares event write set against read set of diffs applied since `L`:
- `writes(evt) ∩ reads(appliedSinceL) ≠ ∅` → send to paradox queue, increment entropy `wM + wP`, emit `ParadoxEvent` for inspector.
- Otherwise, proceed to delivery.

### 5. Deliver
During `timeline_flush`:
- Dequeue in order: `(chronos, evt.id, bridgeSeq)`.
- Inject into Codex’s Baby queue for target branch and phase.
- Mark `seen.add(dedupHash)` if exactly-once.
- Accumulate entropy: `entropy += wM` per delivered cross-branch event; double if retro.
- Append causal edges `causeIds → envelopeHash` to branch node’s `CausalityGraph`.

### 6. Collapse Handling
If branch β collapses before delivery:
- Determine merge target branch α and node `mergeNode`.
- Re-route event to α with `chronos = max(evt.chronos, mergeNode.chronos)`.
- Tag `evt.metadata.reroutedFrom = β`.

---

## APIs

```ts
interface TemporalBridge {
  enqueue(entry: BridgeQueueEntry): void;
  deliver(context: BridgeContext): void; // invoked during timeline_flush
  rerouteCollapsed(branch: KairosBranchId, mergeTarget: BranchId, mergeNode: NodeId): void;
  stats(): BridgeInspectorFrame;
}

interface BridgeContext {
  readonly getBranchHead: (branch: BranchId) => NodeId;
  readonly forkRetro: (baseNode: NodeId, targetBranch: BranchId) => BranchId;
  readonly applyParadoxQuarantine: (evt: EventEnvelope, branch: BranchId) => void;
  readonly pushToCodex: (evt: EventEnvelope, branch: BranchId) => void;
}

interface BridgeInspectorFrame {
  readonly tick: ChronosTick;
  readonly pendingPerBranch: Record<KairosBranchId, number>;
  readonly retroEvents: RetroRecord[];
  readonly paradoxes: number;
  readonly rerouted: number;
}
```

---

## Determinism Hooks
- Delivery order deterministic: `(chronos, evt.id, bridgeSeq)`.
- Retro forks create β′ deterministically (branch ID = hash(baseNodeId || sourceBranch || evt.id)).
- Dedup hash ensures identical events dropped identically on replay.
- Paradox detection uses normalized read/write sets; same inputs → same quarantine set.

---

## Error Codes
- `ERR_BRIDGE_DUPLICATE` – event dropped due to dedupHash collision in exactly-once mode.
- `ERR_BRIDGE_CAPABILITY` – insufficient capabilities for cross-branch delivery.
- `ERR_BRIDGE_PARADOX` – event quarantined; require manual intervention.

---

## Test Matrix
1. Retro Branch – cross-branch event to past tick creates β′ with LCA recorded.
2. Paradox Quarantine – artificial read/write overlap triggers quarantine and entropy increment.
3. Dedup – identical events emitted twice drop deterministically.
4. Collapse Reroute – branch collapse reroutes pending events without duplication.

---

The Temporal Bridge spec links Codex’s Baby and the branch tree, enforcing causal integrity for cross-branch events.
