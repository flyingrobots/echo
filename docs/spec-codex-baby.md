<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Codex’s Baby Specification (Phase 0.5)

Codex’s Baby (CB) is Echo’s deterministic event bus. It orchestrates simulation events, cross-branch messaging, and inspector telemetry while respecting causality, security, and the determinism invariants defined in Phase 0.5.

---

## Terminology
- **Event** – immutable envelope describing a mutation request or signal. Commands are a subtype of events.
- **Phase Lane** – per-scheduler phase queue storing pending events.
- **Priority Lane** – optional high-priority sub-queue for engine-critical events.

```ts
type EventKind = string; // e.g., "input/keyboard", "ai/proposal", "net/reliable"
```

---

## Event Envelope

```ts
interface EventEnvelope<TPayload = unknown> {
  readonly id: number;                 // monotonic per branch per tick
  readonly kind: EventKind;
  readonly chronos: ChronosTick;       // target tick (>= current for same-branch)
  readonly kairos: KairosBranchId;     // target branch
  readonly aionWeight?: number;
  readonly payload: TPayload;

  // Determinism & causality
  readonly prngSpan?: { seedStart: string; count: number };
  readonly readSet?: ReadKey[];
  readonly writeSet?: WriteKey[];
  readonly causeIds?: readonly string[]; // upstream event/diff hashes

  // Security & provenance
  readonly caps?: readonly string[];   // capability tokens required by handlers
  readonly envelopeHash?: string;      // BLAKE3(canonical bytes)
  readonly signature?: string;         // optional Ed25519 signature
  readonly signerId?: string;

  readonly metadata?: Record<string, unknown>; // inspector notes only
}
```

ID semantics:
- Reset to 0 each tick per branch.
- Cross-branch mail records `bridgeSeq`, but delivery order remains `(chronos, id, bridgeSeq)`.
- Canonical encoding: sorted keys for maps/arrays, little-endian numeric fields, no timestamps in hash.

---

## Queues & Lanes
Per scheduler phase, CB maintains a deterministic ring buffer with an optional priority lane.

```ts
interface EventQueue {
  readonly phase: SchedulerPhase;
  priorityLane: RingBuffer<EventEnvelope>;
  normalLane: RingBuffer<EventEnvelope>;
  size: number;
  capacity: number;
  highWater: number;
  immediateUses: number;
}
```

Dequeue order per phase: `priorityLane` FIFO, then `normalLane` FIFO. Capacities are configurable per lane; high-water marks tracked for inspector telemetry.

---

## Handler Contract

```ts
interface EventHandler {
  readonly kind: EventKind;
  readonly phase: SchedulerPhase;
  readonly priority?: number;
  readonly once?: boolean;
  readonly requiresCaps?: readonly string[];
  (evt: EventEnvelope, ctx: EventContext): void;
}
```

Registration:
- Deterministic order captured in `handlerTableHash = BLAKE3(sorted(phase, kind, priority, registrationIndex))`.
- Hash recorded once per run for replay audits.

Handlers may only run if `requiresCaps` ⊆ `evt.caps`; otherwise `ERR_CAPABILITY_DENIED` halts the tick deterministically.

---

## Event Context

```ts
interface EventContext {
  readonly timeline: TimelineFingerprint; // { chronos, kairos, aion }
  readonly rng: DeterministicRNG;         // obeys evt.prngSpan if present

  enqueue<T>(phase: SchedulerPhase, evt: EventEnvelope<T>): void;
  forkBranch(fromNode?: NodeId): BranchId;
  sendCross<T>(evt: EventEnvelope<T>): void; // wraps Temporal Bridge
}
```

Rules:
- `enqueue` targets same-branch events with `evt.chronos >= currentTick`.
- `sendCross` is the only sanctioned cross-timeline route.
- If `evt.prngSpan` provided, handler must consume exactly `count` draws; mismatch raises `ERR_PRNG_MISMATCH`.

---

## Temporal Bridge

Features:
- **Exactly-once toggle** via dedup set `seenEnvelopes: Set<hash>` on receiver.
- **Retro delivery**: if `evt.chronos < head(target).chronos`, spawn retro branch β′ from LCA, rewrite target, tag `evt.metadata.retro = true`.
- **Reroute on collapse**: if branch collapses before delivery, forward to merge target and record `evt.metadata.reroutedFrom`.
- **Paradox pre-check**: if `evt.writeSet` intersects reads applied since LCA, route to paradox handler/quarantine and increment entropy by `wM + wP`.

Delivery policy defaults to at-least-once; exactly-once enables dedup.

---

## Immediate Channel
- Whitelist event kinds (`engine/halt`, `engine/diagnostic`, etc.).
- Per-tick budget; exceeding emits `ERR_IMMEDIATE_BUDGET_EXCEEDED` and halts deterministically.

---

## Backpressure Policies

```ts
type BackpressureMode = "throw" | "dropOldest" | "dropNewest";
```

- Development default: `throw` (abort tick with `ERR_QUEUE_OVERFLOW`).
- Production defaults: `dropNewest` for `pre_update`, `dropOldest` for `update`/`post_update`.
- Each drop records `DropRecord { phase, kind, id, chronos }` added to the run manifest; replay reproduces drop order.

---

## Inspector Packet

```ts
interface CBInspectorFrame {
  tick: ChronosTick;
  branch: KairosBranchId;
  queues: {
    [phase in SchedulerPhase]?: {
      size: number;
      capacity: number;
      highWater: number;
      enqueued: number;
      dispatched: number;
      dropped: number;
      immediateUses?: number;
      p50Latency: number; // enqueue→dispatch ticks
      p95Latency: number;
      kindsTopN: Array<{ kind: EventKind; count: number }>;
    };
  };
}
```

Emitted after `timeline_flush` so metrics do not perturb simulation.

---

## Determinism Hooks
- Dispatch order per phase: FIFO by `(chronos, id, bridgeSeq)` with deterministic tie-break by registration order and priority.
- PRNG spans enforced; mismatches halt tick.
- `readSet`/`writeSet` recorded for causality graph and paradox detection.
- Security envelopes verified before handler invocation; tampering emits `ERR_ENVELOPE_TAMPERED`.

---

## Capability & Security
- `requiresCaps` enforced at dispatch.
- If `signature` present, verify `Ed25519(signature, envelopeHash)`; failure halts deterministically.
- Capability violations and tampering log deterministic error nodes.

---

## Public API Surface

```ts
interface CodexBaby {
  on(handler: EventHandler): void;
  off(handler: EventHandler): void;

  emit<T>(phase: SchedulerPhase, evt: EventEnvelope<T>): void;   // same branch
  emitCross<T>(evt: EventEnvelope<T>): void;                     // via bridge

  flush(phase: SchedulerPhase, ctx: EventContext): void;         // scheduler hook
  stats(): CBInspectorFrame;                                     // inspector packet
}
```

All mutations route through CB; external systems observe only inspector packets and deterministic manifests.

---

## Implementation Checklist
1. **Rename & Canonicalize** – adopt `EventEnvelope`, implement canonical encoder + BLAKE3 hash.
2. **Handler Table Hash** – compute once at startup, record in run manifest.
3. **Backpressure & Drop Records** – per-queue policies with deterministic drop manifests.
4. **PRNG Span Enforcement** – wrap handlers to track draws when `prngSpan` present.
5. **Temporal Bridge Enhancements** – dedup, retro branch creation, reroute on collapse, paradox pre-check.
6. **Capability Gate** – enforce `requiresCaps`, emit errors on violation.
7. **Inspector Packet** – produce `CBInspectorFrame` with latency/top-N metrics.
8. **Immediate Channel Budget** – whitelist + counter enforcement.
9. **Error Codes** – define deterministic errors: `ERR_QUEUE_OVERFLOW`, `ERR_CAPABILITY_DENIED`, `ERR_ENVELOPE_TAMPERED`, `ERR_IMMEDIATE_BUDGET_EXCEEDED`, `ERR_PRNG_MISMATCH`.
10. **Docs & Samples** – provide example flow (input event → system → diff) with read/write sets and zero PRNG span.

---

## Test Matrix
- **Determinism:** Same event set (with PRNG spans) across Node/Chromium/WebKit ⇒ identical `worldHash` and handlerTableHash.
- **Backpressure:** Force overflow for each mode; replay reproduces drop manifest.
- **Temporal Bridge:** Cross-branch retro delivery creates β′ correctly and respects paradox quarantine.
- **Security:** Capability mismatch raises deterministic error; tampered signatures rejected.
- **Immediate Channel:** Budget exceed halts deterministically; under budget yields identical final state.
- **Inspector Metrics:** Latencies stable; inspector calls have no side effects.

---

Adhering to this spec aligns Codex’s Baby with the causality layer and determinism guarantees established for the branch tree.
