# Codex’s Baby Specification (Phase 0)

Codex’s Baby is Echo’s event/command bus. It routes gameplay commands, system events, and inter-branch communication while preserving determinism. This spec documents the data structures, lifecycle, and integration points for the initial implementation.

---

## Goals
- Deterministic buffering and delivery of commands within the fixed-timestep loop.
- Segregated queues per phase to control when handlers execute.
- Capacity planning and backpressure to prevent runaway memory usage.
- Inter-branch bridge to deliver messages across timelines (including retro branches).
- Instrumentation hooks for profiling and debugging (timeline inspector).

---

## Data Model

### Command Envelope
```ts
interface CommandEnvelope<TPayload = unknown> {
  readonly id: number;                   // stable, per-branch incrementing
  readonly kind: string;                 // semantic tag (e.g., "input/keyboard")
  readonly chronos: ChronosTick;         // target tick (>= current for current branch)
  readonly kairos: KairosBranchId;       // branch recipient
  readonly aionWeight?: number;          // optional significance
  readonly payload: TPayload;
  readonly metadata?: Record<string, unknown>; // instrumentation, debugging
}
```

`id` is deterministic within a branch: start at 0 each tick, increment per enqueue. For cross-branch messages, the bridge assigns a composite ID `(sourceBranchId, sequence)` so messages can be traced.

### Queues

Codex’s Baby maintains multiple buffers:

| Buffer | Description | Delivery Phase |
| ------ | ----------- | -------------- |
| `immediateQueue` | Rare “execute-now” commands (discouraged, telemetry heavy). | Direct dispatch (same frame) |
| `preUpdateQueue` | Input assimilation, external adapter commands. | `pre_update` |
| `updateQueue` | Simulation events generated during `update`. | `update` (next system) |
| `postUpdateQueue` | Deferred cleanup, physics callbacks. | `post_update` |
| `timelineQueue` | Branch management, timeline merges. | `timeline_flush` |

Each queue is a ring buffer with fixed capacity configured via engine options (growable via doubling if allowed). Structure:
```ts
interface CommandQueue {
  readonly phase: SchedulerPhase;
  buffer: CommandEnvelope[];
  head: number;  // dequeue index
  tail: number;  // enqueue index
  size: number;
  capacity: number;
}
```

### Handler Registry
```ts
interface CommandHandler {
  readonly kind: string;
  readonly phase: SchedulerPhase;
  readonly priority?: number;
  readonly once?: boolean;
  (envelope: CommandEnvelope, context: CommandContext): void;
}
```
Handlers register per `kind` and phase. Multiple handlers per kind allowed; executed in deterministic order (priority desc, registration order).

### Command Context
Provides limited capabilities to handler:
```ts
interface CommandContext {
  readonly timeline: TimelineFingerprint;
  enqueue: <T>(phase: SchedulerPhase, envelope: CommandEnvelope<T>) => void;
  branchFork: (options) => BranchHandle;
  // Additional utilities: deterministic PRNG, inspector hooks, etc.
}
```

---

## Lifecycle

### Enqueue
```ts
function enqueue(phase: SchedulerPhase, envelope: CommandEnvelope): void {
  const queue = queues[phase];
  if (queue.size === queue.capacity) {
    if (!allowGrowth) throw new BackpressureError(phase);
    growQueue(queue);
  }
  queue.buffer[queue.tail] = envelope;
  queue.tail = (queue.tail + 1) % queue.capacity;
  queue.size += 1;
  metrics.enqueued[phase] += 1;
}
```
- `growQueue` doubles capacity by allocating new array and copying existing elements in order.
- Backpressure: if queue full and growth disabled, scheduler logs and drops oldest (configurable) or throws. Defaults to throw in development, drop-with-warning in production with metrics.

### Flush (per phase)
During scheduler phases, Codex’s Baby flushes relevant queue:
```ts
function flushPhase(phase: SchedulerPhase, context: CommandContext) {
  const queue = queues[phase];
  while (queue.size > 0) {
    const envelope = queue.buffer[queue.head];
    queue.buffer[queue.head] = undefined!;
    queue.head = (queue.head + 1) % queue.capacity;
    queue.size -= 1;

    dispatch(envelope, context);
  }
}
```

### Dispatch
```ts
function dispatch(envelope: CommandEnvelope, context: CommandContext) {
  const handlers = handlerRegistry.get(envelope.kind, envelope.phase);
  if (!handlers?.length) return;
  for (const handler of handlers) {
    handler(envelope, context);
    metrics.dispatched[envelope.phase] += 1;
    if (handler.once) unregister(handler);
  }
}
```
- Exceptions from handlers bubble up; engine decides whether to halt tick or continue (likely halt to preserve determinism).

---

## Inter-Branch Bridge

### Sender Workflow
1. System enqueues `CommandEnvelope` targeting branch `β`.
2. If `β` equals current branch, standard enqueue.
3. Else, pass to `TemporalBridge`:
   - Validate chronology (`envelope.chronos >= currentChronos`).
   - If `envelope.chronos < currentChronos`, create retro branch:
     - Fork from timeline node at `chronos`.
     - Assign new branch ID `β'`.
     - Rewrite envelope with `kairos = β'`.
4. Bridge stores message in per-target buffer keyed by `(branchId, chronos)`.

### Delivery
During `timeline_flush` phase:
```ts
function deliverCrossBranch(context: TimelineContext) {
  const pending = bridge.popForBranch(context.branchId, context.chronos);
  for (const envelope of pending) {
    enqueue(envelope.phase, envelope);
  }
}
```
- If branch collapsed before delivery, bridge retries by rerouting to parent branch or logging orphaned message (tooling hook).

### Entropy & Paradox Checks
- Bridge consults paradox guard: if message would violate invariant (e.g., removing object that already merged), mark envelope with `metadata.paradox` and route to special handler.
- Entropy meter increments based on cross-branch message volume and paradox flags.

---

## Immediate Channel
- Small queue processed immediately upon enqueue (outside scheduler).
- Heavy instrumentation: track usage count per frame; exceed threshold triggers warning.
- Only for engine-critical signals (e.g., emergency halt). Gameplay systems should avoid.

---

## Instrumentation
- Metrics per phase: enqueued, dispatched, dropped, queue high water mark.
- Profiling: average handler duration, percent time spent in Codex’s Baby per phase.
- Timeline inspector: ability to snapshot queue contents, replay at dev time.
- Logging: optional event trace of envelopes (with sampling to limit volume).

---

## Determinism Considerations
- Queue iteration order is stable (FIFO).
- Handler registration deterministic: stored in array by registration order, with stable priority sorting.
- Cross-branch delivery uses deterministic branch IDs; message ordering within target branch sorted by `(chronos, id)`.
- Avoid asynchronous handlers; enforce synchronous execution.

---

## Backpressure Policy
- Default capacities (configurable):
  - `pre_update`: 2048
  - `update`: 4096
  - `post_update`: 2048
  - `timeline_flush`: 1024
- Options:
  - `throw`: halt tick when queue full (development).
  - `dropOldest`: remove oldest envelope (production fallback).
  - `dropNewest`: discard new envelope (if safe).
- Each queue tracks drop stats for telemetry.

---

## Open Questions
- Should envelope payloads be strongly typed via generics per handler registration?
- How to serialize envelopes for network replication (shared format with Persistence port)?
- Do we need priority queues for certain kinds (e.g., high-priority AI commands)?
- What’s the best representation for retro-branch acknowledgments so designers can detect timeline edits?

Updates to this spec should be reflected in `execution-plan.md` and the decision log once implemented. Subsequent work: branch tree diff spec and deterministic math module.
