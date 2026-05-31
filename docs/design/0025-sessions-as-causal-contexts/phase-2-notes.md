<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Phase 2 pre-RED notes — pinned implementation decisions

**Phase 2 RED cannot start until each decision below is signed off.**

The Phase 1 design (`design.md`) deliberately stopped short of
implementation-level choices. The handoff (`phase-2-handoff.md`)
names the five "pre-RED decisions still required." This file pins
those decisions so that RED tests do not invent them accidentally
and Phase 2 GREEN does not litigate them again from scratch.

Each decision below has a **proposed value** and a **status**.
`proposed` means the agent has selected a default with rationale;
the human reviewer signs off (or pushes back) before Phase 2 RED
begins. `accepted` means signed off and binding.

---

## D1 — Abortive close v1 scope

**Question:** Confirm B (the weaker variant) — `close(SessionId)`
applies only to future intents; in-flight intents already past the
session gate continue to a definite outcome. No cancel-by-session-id
for pending head-inbox traffic in v1.

**Proposed:** Confirm B. Phase 2 RED must include a test that
asserts pending head-inbox cancellation is **not** available in v1
(so the deferral is honest and a future cycle has to consciously
reopen it).

**Rationale:** Matches the design.md commitment. A in v1 means
`HeadInbox` learns cancel-by-key, which is a separate refactor with
its own correctness story (idempotency, ordering, observer
attribution); not Phase 2's scope. Keep the smaller surface.

**Status:** `proposed`

---

## D2 — `MissingSession` representation at the decode boundary

**Question:** When an EINT envelope arrives with no `session_id`
header for a session-aware operation, does the decode layer raise
`IntentRejected(stage=Decode, reason=MissingSession)`, or a separate
hard-reject type at the decode boundary that never enters the
intent event stream?

**Proposed:** **Separate hard-reject** at the decode boundary, not
an `IntentRejected` event. A missing header means the envelope is
structurally malformed for the operation it claims to be; there is
no intent to record because there is no admitted intent. Returning
an `IntentRejected` would imply admission machinery saw it, which
is the wrong attribution.

**Rationale:**

- The phase-2-handoff invariants state "`MissingSession` and
  `UnknownSession` do not collapse" and "no `SessionEventLog`
  entry" for `MissingSession`. A hard-reject type at decode satisfies
  both naturally; an `IntentRejected` variant has to remember to
  skip event-log emission.
- The wire boundary is the right enforcement layer. Anything past
  the boundary deals in well-formed envelopes; the hard-reject
  type makes that contract explicit at the type level.
- `UnknownSession` (header present but resolves to nothing) stays
  as `IntentRejected(stage=SessionGate)` because it represents an
  envelope that **did** pass the decode contract; the rejection
  is then a normal admission decision.

**Status:** `proposed`

---

## D3 — `IngressAddress` type home

**Question:** Where does the protocol-shaped `IngressAddress` type
live, and how is it encoded?

**Proposed:** New module **`echo-wasm-abi::ingress_address`** for the
wire-facing protocol value. `warp-core::head_inbox::IngressTarget`
remains the runtime routing target. Encoded under the 0024 universal
LE binary codec **via Wesley emission**, not by hand, once the schema
is declared.

**Rationale:**

- The pair `IngressAddress (wire) <-> IngressTarget (runtime)` is
  conceptually one routing surface, but the current crate graph is
  directional: `warp-core` depends on `echo-wasm-abi`, and
  `echo-wasm-abi` does not depend on `warp-core`. Keeping the
  wire-facing type in `echo-wasm-abi` preserves that direction and
  avoids an ABI -> core dependency cycle.
- Wesley-emitting the codec keeps the wire format in lockstep with
  the protocol type. Hand-rolled codecs accrue the same hand-edit
  bugs we just carded in
  `jedit/docs/method/backlog/bad-code/generated-rope-codec-manual-fixes.md`.
  Don't reintroduce the smell.
- `echo-wasm-abi` is the wire / kernel-port crate; `warp-core`
  imports ABI protocol types and maps them into runtime-owned
  structures. Putting `IngressAddress` in `echo-wasm-abi` matches
  that direction.

**Status:** `proposed`

---

## D4 — `Session` node storage module

**Question:** Which crate / module owns the durable `Session` node
and its lifecycle facts?

**Proposed:** New module **`warp-core::session`**, sibling to
`warp-core::head_inbox` and `warp-core::optic_artifact`. Session
nodes are causal-graph entities and their lifecycle facts
(`SessionOpened`, `SessionClosed`) are causal-WAL events; attaching
them to existing `warp-core::graph` would mix session concerns into
generic graph machinery.

**Rationale:**

- A dedicated module keeps session concerns isolated from the rest
  of the graph schema; future "what does a session own?" questions
  do not ripple through unrelated graph code.
- The causal-WAL attribution stays clean: session events flow
  through the same WAL as other engine events but are typed
  separately, making `SessionEventLog` projection
  (D5) a straightforward filter rather than a join.
- Matches the precedent for `head_inbox` (also a focused module
  owning a specific causal concern).

**Status:** `proposed`

---

## D5 — `SessionEventLog` strategy

**Question:** Is `SessionEventLog(session_id)` computed on read or
materialized?

**Proposed:** **Compute on read** for v1.

**Rationale:**

- The phase-2-handoff explicitly recommends compute-on-read as the
  v1 default "unless benchmark data forces materialization." No
  such benchmark data exists yet.
- The design framing is "derived projection." Materialization is a
  perf optimization with attendant cache-invalidation costs;
  compute-on-read is correctness-first.
- Phase 2 RED can include a marker test (`#[ignore]` or
  `// PERF-FOLLOWUP`) declaring the benchmark trigger for a future
  materialization cycle: "if computing `SessionEventLog(genesis)`
  exceeds N ms over M events, reopen this decision."

**Status:** `proposed`

---

## D6 — `system/genesis` `PrincipalRef` construction

**Question:** How is the `PrincipalRef::system("genesis")`-shaped
principal constructed today, and what (if anything) does
`warp-core::optic_artifact::PrincipalRef` need to grow to support
it?

**Current state:** `PrincipalRef` is `{ id: String }` with no
`system()` or `genesis()` constructor; today you would write
`PrincipalRef { id: "system/genesis".to_string() }` directly. The
string-literal-init path is fine for tests but spreads "what is the
genesis principal called?" across call sites.

**Proposed:** Add **`PrincipalRef::system(label: &str) -> Self`**
and **`PrincipalRef::genesis() -> Self`** (the latter as a tiny
sugar for the canonical genesis principal). The string format
(`"system/<label>"`) is enforced inside the constructor so call
sites cannot drift on the exact representation.

```rust
impl PrincipalRef {
    pub fn system(label: &str) -> Self {
        Self { id: format!("system/{label}") }
    }
    pub fn genesis() -> Self {
        Self::system("genesis")
    }
}
```

**Rationale:**

- The genesis principal name is a load-bearing identity. Letting
  every call site spell it as a string literal is the same class
  of bug as a stringly-typed event reason: one typo and you have
  two genesis principals that don't compare equal.
- `system(label)` generalizes — every future engine-owned actor
  ("system/scheduler", "system/recovery") gets the same
  constructor.
- This is additive; no existing call site changes shape (they can
  migrate to the constructor incrementally), so it does not block
  Phase 2 RED — it just enables the first RED test
  (`system/genesis has a concrete PrincipalRef`) to be one line of
  setup.

**Status:** `proposed`

---

## Phase 2 RED sequence (informational, not pinned)

This is the user-proposed sequence from the pre-flight review;
recorded here so RED authors do not re-derive it under time
pressure. Each row may become its own RED batch.

| Step | Scope                                                                 |
| ---- | --------------------------------------------------------------------- |
| R1   | Session identity + `system/genesis` exists                            |
| R2   | Session lifecycle events (`SessionOpened`, `SessionClosed`)           |
| R3   | Admission gate: `MissingSession` / `UnknownSession` / `ClosedSession` |
| R4   | Intent attribution + `SessionEventLog` projection                     |
| R5   | Receipt vs quiescence; one-way quiescence gate                        |
| R6   | `IngressAddress` decode boundary → `IngressTarget`                    |
| R7   | jedit migration trigger / leash fires                                 |

When R7 closes, the leash file
`jedit/docs/method/backlog/leash/jedit-session-port.md` transitions
through `triggered → deleted` and the jedit-side scaffold is
removed.

---

## Sign-off

The Phase 2 agent must not begin RED until every decision above is
in `accepted` status. Update the status fields in this file (and
record any pushback / counter-proposal inline under the relevant
section) before opening the `cycle/0025-sessions-as-causal-contexts-red`
branch.

If a decision needs to be reopened mid-Phase-2 (RED reveals an
invariant the proposal can't satisfy), reopen via the same file:
flip the status to `reopened`, capture the discovery, and update.
Do not silently drift.
