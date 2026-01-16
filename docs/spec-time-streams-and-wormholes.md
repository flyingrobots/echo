<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# TimeStreams, Cursors, and Wormholes (Multi-Clock Time for Echo)
> **Background:** For a gentler introduction, see [WARP Primer](/guide/warp-primer).


Status: Draft Spec (Phase 0.75)  
Date: 2026-01-01  

This spec formalizes “time” in Echo as **multiple independent event streams** (“clocks”) with explicit **admission policies** into a **deterministic simulation worldline**.

It also clarifies how **wormholes** (reserved term: tick-range compression) enable “instant catch-up” and fast seeking without weakening provenance guarantees.

Related docs:
- `docs/spec-timecube.md` (Chronos × Kairos × Aion)
- `docs/spec-networking.md` (rollback/branching networking modes)
- `docs/spec-temporal-bridge.md` (cross-branch delivery lifecycle)
- `docs/testing-and-replay-plan.md` (replay verification contract)
- `docs/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md` (terminology law: wormhole is compression only)

---

## Problem Statement

Echo must support the following simultaneously:

1. **Wall-clock never stops**: network traffic arrives; humans click UI; OS timeouts fire.
2. **Simulation time can pause/rewind/fork**: time-travel debugging and multiverse gameplay require it.
3. **Tools must remain live** while the simulation is paused/rewound (authoring/debug UI cannot freeze).
4. **Distributed “game clocks” diverge**: a server can advance while a client pauses; messages can appear “from the future” relative to a rewound local simulation.

The naïve “one global game clock that drives everything” cannot model these requirements without paradoxical edge cases.

---

## Core Idea

Treat every external domain (network, input, UI, timers, rendering) as an **append-only event stream** with its own sequencing.

Then define the simulation as:

- a **worldline** (Chronos over a branch) that evolves only by **journaled graph rewrites**, and
- a set of **stream cursors** (per view / per worldline branch) that decide which stream events have been **admitted** into that worldline.

“Pausing a clock” becomes: *freeze a cursor and/or change admission policy*, not “stop the world.”

---

## Definitions

### HostTime vs HistoryTime (Rule of Thumb)

Echo benefits from a simple determinism discipline:

- **HostTime**: time sources outside the worldline (OS wall clock, monotonic clocks, NTP, hardware timers).
- **HistoryTime**: time derived as a fold over the committed history (worldline / event DAG / log).

Rule:

> Any operation that changes semantic state must be pure relative to HistoryTime.
> If an adapter consults HostTime, it must emit a decision record into history before the simulation consumes it.

For shorthand, “pure in HistoryTime” means: the semantic transition is a pure function of prior history + admitted inputs + pinned artifacts, and does not consult HostTime directly.

This aligns “time” with the existing Echo determinism contract:
replay must not require reading the host clock.

### Wall Clock (`W`)

The OS monotonic clock. It always moves forward and may be used for:
- pacing UI and rendering,
- timeouts/retries in adapters,
- metadata (“this observation was read at wall time W”).

Wall clock is **not authoritative** for deterministic simulation state.

### Chronos Tick (`T`)

Simulation time for a fixed worldline:
- `Tick(u64)` (discrete)
- optionally a derived `SimTime = Tick * dt_fixed`

Chronos is the index for replay, diffs, and deterministic stepping (`docs/spec-timecube.md`).

### TimeStream

An append-only log of events produced by some domain.

Minimum properties:
- Each event has a **monotonic** stream-local sequence number `seq` (no gaps in the produced ordering).
- Each event may include `wall_time` metadata (for UX and diagnostics).
- Each event’s payload is canonically encodable (for hashing/dedup where needed).

Examples:
- `NetworkRx`: inbound bytes/messages observed from sockets
- `NetworkTx`: outbound sends initiated by the simulation (often derivable from rewrites, but may still be useful as a stream for tooling)
- `GameInput`: “intended player actions” admitted into the sim
- `ToolInput`: keyboard/mouse/UI input used to control tools (should remain live during debugging)
- `RenderTicks`: rendering heartbeat events (purely in tool space)
- `Timer`: wall-clock-driven timer expirations (if allowed) or deterministic timers derived from Chronos

### Decision Records (Clock Decisions)

Whenever HostTime influences a semantic decision, the adapter must emit a canonical decision record that pins:

- raw samples (e.g., monotonic `Instant`, wall clock timestamp, sync quality),
- a stable `policy_hash` (how samples were interpreted),
- the derived time/value that will be treated as HistoryTime by higher layers,
- optional context (source id, jitter/drift estimates, uncertainty).

Conceptually:

- The adapter reads HostTime and produces a `ClockDecision` event into a stream.
- Admission of that event inserts an observation fact (graph rewrite) that simulation rules can consume deterministically.

This makes “consult HostTime” replay-safe: replays consume the recorded decision record rather than touching the host clock.

### Decision Records (Stream Admission Decisions)

Clock decisions solve the “what time is it?” class of nondeterminism. Time travel and distributed sessions add a second class:

- the OS/network can produce events at unpredictable wall times, and
- the simulation must decide *when* those events become semantically real (admitted) for a given view/worldline.

To keep simulation rules pure and replayable, we treat “admission” itself as a **decision record** that is committed into history.

#### What this pins (minimum)

A `StreamAdmissionDecision` must pin enough information that replay can reconstruct **exactly**:

- which stream events were admitted,
- into which view/worldline and at which Chronos tick,
- under which policy/budgets/fairness settings.

Minimum fields:

- **Identity**
  - `decision_id`: derived stable id for cross-references (see below; not separately encoded)
  - `view_id`: which view/session this applies to (e.g., `ViewSim(A)` vs `ViewTool(A)`)
  - `stream_id`: which stream (e.g., `NetRx(A)`, `ToolInput`)
- **Where in HistoryTime**
  - `worldline_ref`: branch/worldline identifier (Kairos + Aion coordinates as needed; pinned by the snapshot header, not separately encoded in the admission digest)
  - `admit_at_tick`: Chronos tick at which admission occurred
- **What was admitted**
  - `admitted_range`: `[from_seq, to_seq]` (inclusive) or an explicit list for sparse admission
  - `admitted_digest`: canonical digest of the admitted event payloads (or their canonical encodings)
- **Why / how it was chosen**
  - `policy_hash`: stable hash of the admission policy version + parameters
  - `budget`: the configured admission budgets for this tick (msgs/bytes/work units)
  - `fairness_order_digest`: digest of the deterministic source ordering used for selection (e.g., connection ids)

#### Canonical schema (illustrative)

This is a conceptual schema; the on-wire encoding must use Echo’s canonical encoder and include all fields above in a stable order (or make them derivable/pinned, as with `decision_id` and `worldline_ref`).

```ts
interface StreamAdmissionDecision {
  // Derived ID for cross-references. If serialized redundantly, it must match
  // the deterministic derivation described below.
  decisionId?: string;
  viewId: string;
  streamId: string;

  // Where in the replayable history this admission takes effect.
  // Worldline reference (Kairos/Aion coordinates). This is pinned by the
  // snapshot header in commit ancestry and may be omitted from the admission
  // digest encoding.
  worldlineRef: {
    universe: string; // Aion id
    branch: string;   // Kairos id
  };
  admitAtTick: number; // Chronos tick

  // What was admitted from the stream.
  admittedRange?: { fromSeq: number; toSeq: number }; // inclusive
  admittedSeqs?: number[]; // for sparse admission
  admittedDigest: string; // digest(canonical(event_i)) over admitted events

  // Determinism pins.
  policyHash: string;
  budget: { maxEvents?: number; maxBytes?: number; maxWorkUnits?: number };
  fairnessOrderDigest?: string;
}
```

##### `decision_id` derivation (required for determinism)

`decision_id` is **derived**, not an independently-assigned identifier:

- Let `decision_record_bytes_v1` be the per-decision canonical bytes used inside
  the `admission_digest` encoding (see `docs/spec-merkle-commit.md`).
- Then:

  - `decision_id = blake3( "echo:stream_admission_decision_id:v1\0" || decision_record_bytes_v1 )`

Domain separation ensures `decision_id` cannot be confused with other digests.

#### How it composes with “observation as graph rewrite”

On a live run:

1. Adapters append events into streams (impure edge).
2. Admission selects a subset for this tick and emits a `StreamAdmissionDecision` into history.
3. The simulation inserts observation facts for the admitted stream events (graph rewrite), referencing the decision id and admitted seq(s).

On replay:

- Step (2) and (3) are reconstructed from the worldline history; no stream or HostTime access is required.

#### Why `policy_hash` matters

Without a `policy_hash`, replay can still reconstruct the same state if the admitted seq set is explicit,
but you lose two critical properties:

- **Auditability**: you can’t explain why those events were admitted (budgets, fairness, window Δ).
- **Stability under evolution**: policy changes can silently alter behavior if admission is ever recomputed.

Echo’s standard should be: if admission depends on policy, the policy is pinned (hash) in the decision record.

#### Where this lives (locking it in)

`StreamAdmissionDecision` is **HistoryTime**, not optional telemetry.

However, Echo also distinguishes between:

- **replay boundary artifacts** (prescriptive deltas required to reconstruct state), and
- **diagnostic artifacts** (descriptive traces/receipts required to explain and audit decisions).

To keep commit hash v2 stable (patch-digest-only) while still making time travel auditable and replayable:

- The *semantic effect* of admitting stream events is captured by the worldline (via observation rewrites / tick patch ops).
- The *admission decision record* itself must be persisted as a deterministic **diagnostic lane** alongside the tick patch/receipt.

Concretely:

1. Each committed tick produces (at minimum) a tick patch `μ` (`docs/spec-warp-tick-patch.md`) and a tick receipt digest (`decision_digest`) (`docs/spec-merkle-commit.md`).
2. When stream admission is in play, the engine/tooling layer must also emit a deterministic `StreamAdmissionDecision` log for that tick.
3. The snapshot includes an `admission_digest` (deterministic digest of those admission decision records) so tools can:
   - verify they are present and untampered,
   - fetch/index them for time-travel debugging,
   - reproduce “why/how was this admitted?” narratives without consulting HostTime.

This makes admission decisions part of “the history you can fold over” (HistoryTime), without forcing them into the core state delta format.
See `docs/spec-merkle-commit.md` for `admission_digest`.

### Cursor

A `(stream_id, seq)` position indicating “up to which event has been admitted/seen” for a given **view**.

Important: the stream keeps growing even if a cursor is frozen.

### Admission Policy

Rules that map:
- “current view state” + “stream backlog” + “budgets”
to:
- “which events become admitted into the simulation at this tick / step.”

Admission is where “messages from the future” are handled lawfully: events can exist in a stream backlog without being admitted into the current simulation branch.

### Worldline

A worldline is a deterministic history of commits/diffs:
- A Chronos sequence along a branch (Kairos dimension) with recorded rewrite decisions.
- Replay is defined by applying the journal in order (`docs/testing-and-replay-plan.md`).

Worldlines may fork and merge (Kairos) and may coexist (Aion).

### View (Shadow World Session / SWS View)

This spec uses “SWS” informally as “a world-view instance”:

`View = { worldline_ref, tick_cursor, stream_cursors, admission_policy }`

Multiple views may be:
- **entangled** (share the same worldline + cursors), or
- **diverged** (different worldlines and/or different stream cursors).

Tooling is implemented as one or more views that remain responsive regardless of what the simulation view is doing.

---

## The Determinism Contract

Echo’s deterministic promise is:

> Given a starting snapshot and a recorded worldline (including all admitted observation facts), replay reconstructs the same state and produces identical hashes.

Corollaries:
- The OS/network is allowed to be nondeterministic as a *producer of stream events*.
- The simulation is deterministic because only **admitted** events affect it, and admission is journaled as part of the tick patches / receipts.
- On replay, live streams are ignored; the replay consumes the journaled admissions and rewrites.

This fits the “stimuli are not necessarily stored raw, but their processed effects are recorded” approach, while making the observation/admission boundary explicit and toolable.

---

## Observation as Graph Rewrite (“for free” capture)

To align with the “rewrites are recorded by default” discipline:

- External adapters produce observation events into a `TimeStream` (impure edge).
- Admission turns selected stream events into **observation facts** by proposing graph rewrites that insert those facts.
- Those rewrites are recorded in the worldline journal.

This makes “record mode” unnecessary: observation capture is just normal operation.

Practical guidance:
- Record at least the **semantic observation** (e.g., decoded `EventEnvelope` or message payload).
- If debugging adapter/parsing correctness matters, also record the **raw bytes/chunks** as the “truth” and derive parsed facts deterministically inside the sim.

### Timers and “sleep” (HostTime-safe pattern)

Wall-clock timers are a common source of accidental nondeterminism.
To keep reducers pure:

1. A system requests a timer by emitting a deterministic event into history (HistoryTime-only), e.g. “wake me +5s in derived time,”
   including the intended `policy_hash`.
2. An adapter consults HostTime and emits a `TimerFired` / `ClockDecision` record when the host indicates the timer has elapsed.
3. A simulation rule consumes the recorded firing decision and applies the semantic transition.

This prevents `now()` from silently entering semantic state transitions.

---

## Policies (Per Stream)

Policies are per-stream and per-view. A single “Pause” UI action can therefore be defined as a policy bundle, not a global stop.

### Live (bounded)

- Admit events each tick up to a budget (`msgs_per_tick`, `bytes_per_tick`, or `work_units_per_tick`).
- Enforce a deterministic fairness order across sources (e.g., round-robin by connection id).

### Pause + Drop

- Cursor is frozen.
- Incoming stream events are discarded (or never produced, if the adapter is stopped).

Use when the stream should not influence the view while paused (e.g., gamepad input during debugging).

### Pause + Buffer

- Cursor is frozen.
- Stream continues to grow; events accumulate in a spool/backlog.

Use when you must remain connected (network) or keep tool interaction live.

### Rewind / Fork

Rewind is a view operation, not a stream operation:
- Fork a new worldline at tick `T-k` (Kairos fork).
- Set the simulation view to the forked worldline.
- Stream cursors may remain where they were (so future events remain buffered) or may be reset (detached analysis).

### Catch-Up

Catch-up is “advance cursors (and simulation ticks) quickly until a target predicate holds.”

Targets (choose explicitly):
- **Backlog target**: drain buffered events until backlog < threshold.
- **Remote tick target**: run until reaching a stamped `server_tick` (common for client/server).
- **Budget target**: run until a fixed compute/time budget is consumed, then stop (tool remains responsive).

Catch-up must still respect deterministic ordering and footprint-based conflict rules.

---

## Wormholes (Instant Catch-Up and Fast Seeking)

Terminology law: “wormhole” is reserved for **history/payload tick-range compression** and is not a synonym for portals/instances.
See `docs/architecture/TERMS_WARP_STATE_INSTANCES_PORTALS_WORMHOLES.md`.

### Definition (Operational)

A wormhole is a compressed representation of a multi-tick segment:

- It replaces a contiguous tick range `[i, k]` with a single edge whose label carries a **sub-payload** `P_{i:k}` sufficient to reconstruct the interior segment.
- Wormholes are provenance-preserving: expansion yields the original committed ticks and receipts.

### Why Wormholes Enable “Instant” Catch-Up

Catch-up is expensive when it requires replaying thousands of ticks to reach a known state.
Wormholes provide two acceleration paths:

1. **Checkpoint carriers**: store a materialized snapshot at the wormhole output boundary (or periodic checkpoints inside the wormhole).
   - Fast-forward becomes: load the checkpoint and continue.
   - Verification becomes: optionally expand/verify the wormhole offline or lazily.

2. **Verified skip**: if a wormhole boundary includes hashes that pin the input/output state roots and the sub-payload digest, a tool can “skip” the interior while maintaining tamper evidence.

The key insight is that “instant” is always a choice between:
- *compute now* (expand and replay) vs
- *trust boundary* (skip using checkpoint/hashes) vs
- *verify later* (lazy expansion / sampling).

Echo should treat this as a policy surface, not an implicit behavior.

### Tooling Implications

Time-travel tooling should use wormholes for:
- rapid seeking across long histories,
- rendering timelines without loading full interior traces,
- producing “replay cost”/“distance” metrics (wormhole expansion cost is measurable and budgetable).

---

## Distributed Scenario: “Messages from the Future”

In a connected session:
- The server continues advancing its worldline.
- The client can fork/rewind its local worldline for debugging or gameplay.
- Network stream continues spooling incoming messages (buffer policy).

Those messages are not paradoxical; they are simply **not admitted** into the rewound branch yet.

Two lawful resolution strategies:

1. **Catch-up**: resume admission and advance until the client view rejoins the synchronized regime.
2. **Merge**: treat divergent worldlines as Kairos branches and reconcile via explicit merge semantics (authority policy, conflict rules, paradox quarantine).

This is compatible with cooperative gameplay where peers exist “out of sync” intentionally; reconciliation becomes an explicit game mechanic instead of a networking bug.

---

## Worked Example: Network Latency + Connected Rewind + Wormhole Catch-Up

This section walks through your “M1/A1” latency diagram using the TimeStreams/Cursors model, then extends it with a connected time-travel rewind.

### Actors and Streams

We model three things:

- **Worldline**: the committed simulation history along a branch (Chronos ticks).
- **Streams**:
  - `NetRx(A)`: inbound network observations at client A (append-only)
  - `NetRx(B)`: inbound network observations at client B (append-only)
  - (Optional) `NetTx(A)` / `NetTx(B)`: outbound send intents (often derivable from world rewrites, but useful for tooling lanes)
- **Cursors** (per view):
  - `cursor(NetRx(A))`, `cursor(NetRx(B))`

We assume fixed-timestep ticks for simplicity; the same reasoning works if `dt` is itself a stream.

### Baseline (No Time Travel)

Narrative:

```text
 [client A]         [client B]  tick
    |                  |         0
   [M1]                |         1  A sends "M1" to B
 .  |\____________     |         2
    |             \___[M1]       3  B reads "M1" from A
    |                 [A1]       4  B sends response "A1" to A
 .  |        _________/|         5
    | _____ /          |         6
    |/                 |         7
   [A1]                |         8  A reads "A1" from B
    |                  |         9
```

Operationally, the important distinction is:

- “send” is a simulation decision (a rewrite; journaled in the worldline),
- “read/observe” is admission of `NetRx` stream events (also journaled via observation rewrites).

One plausible event/cursor story (illustrative):

1) Tick 1 — A sends M1
   - A simulation rule decides to send:
     - worldline records a rewrite like `NetSendIntent { to=B, payload=M1 }`
   - (Optional) also append `NetTx(A)` stream event for tooling lanes (not required for determinism).

2) Tick 3 — B admits observation M1
   - The adapter reads OS/network and appends a new stream event:
     - `NetRx(B)[seq=42] = bytes("M1")` (plus metadata like wall time)
   - Admission policy for `NetRx(B)` is Live; at tick 3 it admits seq 42:
     - cursor advances: `cursor(NetRx(B)) = 42`
     - admission produces an observation rewrite inserting a fact:
       - `ObservedNetMessage { from=A, msg="M1", rx_seq=42 }`
   - Simulation rules consume `ObservedNetMessage(M1)` and produce deterministic rewrites (game logic).

3) Tick 4 — B sends A1
   - Deterministic send intent rewrite is recorded: `NetSendIntent { to=A, payload=A1 }`.

4) Tick 8 — A observes A1
   - Adapter appends `NetRx(A)[seq=99] = bytes("A1")`.
   - Admission at tick 8 admits seq 99:
     - `cursor(NetRx(A)) = 99`
     - inserts `ObservedNetMessage { from=B, msg="A1", rx_seq=99 }`
   - Simulation rules consume it.

Key point: the worldline captures *the ticks where observations were admitted*, regardless of what wall clock did.
Replay re-applies those observation rewrites; it does not consult the network.

### Connected Time Travel: Rewind Locally While Staying Connected

Now assume A is connected to a server/session and chooses to time travel.
We introduce two views at A:

- `ViewSim(A)`: the simulation view you can rewind (the “game world” view)
- `ViewTool(A)`: the debugger/authoring UI view (must remain live)

At tick 9 (on A), the user triggers rewind “100 ticks ago” (here we’ll use 7 ticks for the toy example):

1) Fork a new worldline branch
   - `W` is the current synchronized branch.
   - Fork at tick 2 (Kairos): create `W' = Fork(W @ tick=2)`.
   - Set `ViewSim(A).worldline = W'` and `ViewSim(A).tick = 2`.

2) Apply a pause/buffer policy bundle
   - `ToolInput`: Live (tools stay interactive)
   - `GameInput`: PauseDrop (avoid accidental player actions while rewound)
   - `NetRx(A)`: PauseBuffer (stay connected; keep spooling inbound data; admit nothing into `W'` for now)

While A is rewound, the network continues producing stream events:

- `NetRx(A)[seq=100]`, `NetRx(A)[seq=101]`, ...

But because `NetRx(A)` is PauseBuffer for `ViewSim(A)`, the cursor does not move:

- `cursor(NetRx(A))` remains at whatever value it had at tick 2 on this branch (e.g., 0 or 98 depending on history).

These buffered events are “from the future” relative to `W'`’s Chronos tick=2, but they are not paradoxes:
they exist in the stream backlog and are simply not admitted.

### Returning to Sync: Catch-Up via Wormhole/Checkpoint

There are multiple lawful “return to sync” strategies; Echo should make the choice explicit.

#### Strategy A: Discard the diverged branch and reattach to the canonical head (resync)

If A wants to reattach to the shared session truth quickly:

- Abandon `W'` (or keep it as an analysis branch).
- Switch `ViewSim(A)` back to `W @ head`.
- Use wormholes/checkpoints to seek instantly to the head state without replaying every tick locally.

This is the pragmatic “snap back to the server timeline” behavior.

#### Strategy B: Fast-forward the diverged branch using a wormhole segment (catch-up on the same branch)

If `W'` is a strict prefix fork (no divergent commits after the fork point) and is meant to rejoin by advancing:

1) Pick a target tick `k` (e.g., tick 9 on `W`).
2) Use a wormhole segment `P_{2:k}` (or periodic checkpoints) to skip replay work:
   - load a checkpoint at tick `k` (wormhole output boundary),
   - optionally verify the wormhole boundary hashes (now or later).
3) Extend `W'` from tick 2 → `k` by applying the wormhole segment (equivalent to replaying that tick range), then set `ViewSim(A).tick = k`.
4) Decide what to do with buffered stream backlog:
   - **Admit and process** (may create divergence if those events were not part of the original worldline),
   - **Drop** (if they are invalid for the rejoined regime),
   - **Reconcile via merge** (if you want “future you sent me data while I was in the past” as gameplay).

The important bit is that wormholes accelerate “move the simulation view to a known tick” without changing the admission rules.
They do not magically resolve semantic conflicts; they just make seeking/catch-up computationally tractable.

---

## Worked Example: `server_tick` Stamps, Catch-Up Targets, and When Merge Is Required

The previous example shows “messages arrive late in Chronos ticks.” In distributed sessions, we also need a stable notion of:

- “what tick did the sender believe this corresponds to?” and
- “how do I catch up without wall-clock coupling?”

This example introduces a stamped `server_tick` (or more generally `sender_tick`) and shows:

- a concrete catch-up predicate (“advance until `local_tick >= server_tick - window`”), and
- the line between **catch-up** (advance on a compatible history) and **merge/resync** (reconcile divergent histories).

### Minimal message shape

Assume inbound network observations decode to a canonical “envelope” that includes:

- `sender_id`
- `sender_tick` (the sender’s Chronos tick when it emitted the message)
- `payload`
- optional `policy_hash` / schema versioning

The key requirement is that `sender_tick` is part of the admitted observation fact (HistoryTime), not derived from HostTime.

### Scenario setup

- Server `S` advances its canonical worldline `W_S` from tick 1000 onward.
- Client `A` is currently viewing/playing on a compatible branch `W_A` and is behind due to latency.

At wall time `W`, A receives a message `MsgX` that was emitted by `S` at `sender_tick = 1008`,
but A’s simulation view is currently at tick 1003.

We model this as:

- Adapter appends a stream event:
  - `NetRx(A)[seq=500] = bytes(MsgX)`
- Admission at client tick 1003 inserts an observation fact:
  - `ObservedNetEnvelope { sender=S, sender_tick=1008, rx_seq=500, payload=... }`

Important: in this model, “A learned about sender_tick=1008” at tick 1003.
That is not a contradiction; it just means A’s local worldline is now holding a fact about a remote timeline.

### Catch-up target (remote-tick chase)

Define a catch-up policy for the simulation view:

- Track `max_seen_server_tick` as a fold over admitted `ObservedNetEnvelope` facts.
- Define a “safety window” `Δ` (ticks) to bound speculative prediction.

Catch-up predicate:

> Keep ticking forward (admitting buffered network events) until `local_tick >= max_seen_server_tick - Δ`.

Notes:
- If `Δ = 0`, this is strict chase.
- If `Δ > 0`, this resembles rollback/prediction windows: the client runs close to the leading edge but expects corrections.
- The predicate depends only on HistoryTime facts (admitted envelopes), not HostTime.

### Two cases: catch-up vs merge/resync

#### Case 1: History-compatible (catch-up works)

Assume `W_A` is a strict prefix of the canonical server history projection relevant to A (no local divergence that affects authoritative merges).

Then catch-up can be implemented as:

1) Switch `NetRx(A)` admission to Live (bounded) or Catch-Up (raised budgets).
2) Tick forward quickly until the predicate holds.
3) Use wormholes/checkpoints to avoid replaying every intermediate tick:
   - if A has a checkpoint at tick 1007, it can jump there and then simulate 1007→1008 explicitly,
   - or it can apply a wormhole segment `P_{1003:1007}` and then continue.

The key property is that A is not attempting to preserve a divergent local past as “equally real” in this case; it is chasing the compatible line.

#### Case 2: Diverged local history (catch-up alone is insufficient)

Assume A time-travelled and made divergent semantic commits on `W_A'`:

- A rewound to tick 1000, explored an alternative outcome, and committed new rewrites (not just inspection).
- Meanwhile, `S` continued; messages arriving now are stamped with `sender_tick >= 1008`.

Now A cannot “catch up” to `W_S` by fast-forwarding alone, because there is no longer a single compatible future:

- Either A wants to remain in its diverged branch (keep the alternate past), or
- A wants to rejoin the server’s canonical worldline.

So A must choose explicitly:

1) **Resync (discard and reattach)**
   - Keep `W_A'` as an analysis/puzzle branch if desired.
   - Switch `ViewSim(A)` back to the canonical attachment point (`W_A := projection(W_S)`).
   - Seek to a recent tick using a checkpoint/wormhole snapshot provided by the server (fast, practical).

2) **Merge (reconcile histories)**
   - Treat “server canonical” and “client diverged” as Kairos branches and create an explicit merge commit.
   - Apply merge semantics (authority policy / deterministic join rules).
   - If admitted server events conflict with reads/writes in the diverged branch, paradox quarantine rules apply (see `docs/spec-entropy-and-paradox.md`).

This is where your “co-op time travel puzzle” idea becomes real:
merge is a first-class gameplay mechanic rather than an error.

### Tooling implications

The inspector/debugger should be able to show, at minimum:

- `max_seen_server_tick` (from admitted envelopes),
- `local_tick`,
- `lag = max_seen_server_tick - local_tick`,
- current `Δ` (prediction/catch-up window),
- stream backlog for `NetRx(A)` during pause-buffer or rewind,
- whether the current view is “compatible prefix”, “diverged branch”, or “merge candidate.”

This gives the user a concrete mental model: “I am at tick 1003, I have evidence the server is at 1008, and I am choosing whether to chase, resync, or merge.”

---

## Tooling Hooks (Minimum)

Inspector surfaces should be able to observe:
- per-stream backlog size (events, bytes, age in wall time),
- per-view cursor positions,
- current admission policy + budgets,
- wormhole/checkpoint density (expected seek/replay cost),
- “lag in ticks” metrics relative to a stamped remote tick, when applicable.

These are read-only frames; mutation remains capability-guarded and explicit.

---

## Open Questions (Explicitly Deferred)

1. **Security/capabilities** *(high; #246)*: who is allowed to fork/rewind/merge in multiplayer; how provenance sovereignty constrains tool access.
2. **Cross-worldline merge semantics for stream facts** *(high; #245)*: when buffered “future” events are admitted into a forked branch, are they still considered valid, or must they be revalidated/reinterpreted?
3. **`dt` policy** *(medium; #243)*: fixed timestep vs variable dt as an admitted stream. (Fixed is simpler; variable dt should be treated as a stream if allowed.)
4. **Stream retention** *(medium; #244)*: how long spools persist; how they are compacted; how they relate to durability/WAL epochs.

Tracking: deferred questions are tracked in issues #243, #244, #245, and #246; they do not block TT0 spec lock but are required before the time-travel tooling MVP (#171).

---

## Minimal Next Steps (Concrete)

1. Add a small “TimeStreams” section to the networking/debugger docs: define admission cursor + backlog behavior in client pause/rewind.
2. Extend the inspector protocol with a `streams` frame type (backlog + cursor + policy) once the runtime has a concrete stream abstraction.
3. Specify a wormhole/checkpoint storage format aligned with `spec-merkle-commit.md` and the session diff stream (so tools can seek instantly).
