<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0025 — Phase 2 Handoff

**Phase 1 complete. Phase 2 (RED + GREEN) not started.** No Rust tests,
stubs, modules, or API surface have been added in this cycle. Phase 2 is
substantial engine work and starts only with explicit implementation
greenlight.

---

## Branch and history

Branch: `cycle/0025-sessions-as-causal-contexts`, forked from
`stack/echo-le-binary-codec` (per the 0024 coordination bend; see
`design.md`).

Phase 1 commits, in order:

| SHA        | Subject                                                                   |
| ---------- | ------------------------------------------------------------------------- |
| `5a73a3f`  | docs(design): 0025 sessions as causal contexts (Phase 1 design)           |
| `8a21f045` | docs(design): revise 0025 to integrate with existing warp-core primitives |
| `a53cac95` | docs(design): 0025 pre-RED cleanup patch                                  |
| `34e4984b` | docs(design): 0025 pre-RED clarifications                                 |

The full design is in `design.md`. The seed backlog stub is in
`request.md`. The retired `echo-session-proto-split` card lives at
`docs/method/graveyard/PLATFORM_echo-session-proto-split.md` with a
tombstone note.

The companion jedit migration plan is at
`jedit/docs/method/backlog/asap/sessions-migration.md` on the jedit
branch `stack/jedit-rope-rename` (jedit commit `e6abe45`).

---

## What is locked

The design has stabilized through three rounds of human review. The
following decisions are binding on Phase 2 and should not be relitigated
without explicit reopening:

- **Session is a principal-bound causal-context node** — first-class,
  durable, queryable, lifecycle-gated.
- **Session does not own an ingress queue.** `HeadInbox` /
  `IngressTarget` remain the sole ingress and routing surface.
- **Session does not own a mutation-ordering lane.** Worldline / head /
  tick / strand / braid remain the mutation-ordering authority.
- **`SessionId` is unified across read and write.** The existing
  `warp-core::playback::SessionId` shape stays; its semantics widen.
  Two live `SessionId` concepts in warp-core are forbidden.
- **`ViewSession` is a read/playback facet** attached to the unified
  `SessionId`, holding cursors and subscriptions.
- **`SessionEventLog` is a derived projection,** not a lane. Ordered by
  the engine event log's deterministic event order (`LogicalTime` /
  event sequence with stable content-address tie-breaker). Explicitly
  not mutation order.
- **`system/genesis` is primordial.** Created at genesis time, not by
  an intent. Carries a concrete `PrincipalRef::system("genesis")`-shaped
  identity. All other system sessions open through it via normal Open
  intents.
- **No null session anywhere.** No null principal anywhere — including
  `system/genesis`.
- **Two settlement events, not one.** `IntentReceiptIssued` and
  `IntentEffectsQuiesced` are distinct events. `IntentSettled` as a
  name is rejected.
- **`IntentEffectsQuiesced` is a one-way gate per intent.** After it
  fires, no new bounded child work may be registered for that intent.
- **`IntentRejected` records stage and reason.** Stages: `Decode`,
  `SessionGate`, `HeadInboxAdmission`, `Execution`. `MissingSession`
  (header absent at decode) and `UnknownSession` (header present, does
  not resolve at gate) are distinct reasons that must not be silently
  folded together.
- **EINT wire target is `IngressAddress`-shaped.** The wire serializes
  an `IngressAddress` protocol value through the 0024 universal LE
  binary codec; the decode boundary maps it to
  `warp-core::head_inbox::IngressTarget`. The wire does not serialize
  the Rust enum directly.
- **V1 routing is single-target.** `target: IngressTarget` in v1.
  Atomic multi-target submissions are deferred to a follow-up cycle
  with a future shape (`NonEmptyList<IngressTarget>` or
  `AtomicIngressBatch`).
- **V1 cross-session concurrency is obstruction-only.** Multiple
  sessions may target the same worldline; conflicts surface via the
  existing `baseHeadMismatch` admission machinery. True concurrent
  multi-writer lanes are deferred.
- **V1 abortive close is the weaker variant (B).** Blocks new
  submissions, cancels interruptible accepted work. Pending-ingress
  cancellation is deferred to a follow-up cycle. `IngressEnvelope` is
  not extended with `session_id` attribution in v1. `HeadInbox` does
  not gain a cancel-by-`session_id` API in v1.
- **`PrincipalRef` is the existing type** at
  `warp-core::optic_artifact::PrincipalRef`. Reused, not redefined.
- **`echo-session-proto` is retired.** The unqualified name "Session"
  is reclaimed for the causal-context sense. Transport-layer concepts
  should use `Connection` / `TransportConnection` / similar.

---

## Pre-RED decisions still required

These are the concrete decisions the Phase 2 agent must lock in before
writing RED tests. The design names the question; the answer is
implementation-level and was deliberately not forced in Phase 1.

1. **Abortive close v1 scope — confirm B in code.** The design commits
   to the weaker variant. Phase 2 RED tests should explicitly assert
   that pending-ingress cancellation is _not_ available in v1 (so the
   deferral is honest). If the Phase 2 agent re-evaluates and wants to
   try A (full `HeadInbox` cancel-by-`session_id`), that is a design
   reopening, not an implementation choice.

2. **`MissingSession` representation.** The design specifies a `Decode`
   stage and a `MissingSession` reason in `IntentRejected`. The Phase 2
   agent must decide whether the decode layer raises this as an
   `IntentRejected` event or as a separate hard-reject type at the
   decode boundary. Both are acceptable; "silently fold into
   `UnknownSession`" is not.

3. **`IngressAddress` type home.** Where does the protocol-shaped
   `IngressAddress` type live? Options: a new module in
   `echo-wasm-abi`, in `warp-core` alongside `IngressTarget`, or in a
   new codec module. How is it generated / encoded under 0024 — by
   hand, or via the Wesley emit path? Phase 2 picks the home before
   writing the decode boundary tests.

4. **Session node storage.** Which graph / schema layer owns the
   durable `Session` node and its lifecycle facts? The design says
   "the causal graph schema" but does not pin the concrete crate /
   module. Candidates include `warp-core::graph`, a new
   `warp-core::session` module, or attachment to existing causal-WAL
   machinery. Phase 2 chooses and documents.

5. **`SessionEventLog` strategy.** Compute on read or materialize?
   Both are correctness-compatible with the design's "derived
   projection" framing. V1 default recommendation: compute on read
   unless benchmark data forces materialization. Phase 2 confirms.

---

## Phase 2 RED matrix

The design's hard invariants, expressed as RED test targets. **This is a
matrix, not test code.** Phase 2 RED expands each row into actual Rust
test cases (or shell assertions where appropriate per METHOD).

### Session admission gate

- Unknown `SessionId` rejects at stage `SessionGate` with reason
  `UnknownSession`; the rejection is observable on the engine event log
  but produces no `SessionEventLog` entry.
- Missing `session_id` (header absent for a session-aware operation)
  rejects at stage `Decode` with reason `MissingSession`; no
  `SessionEventLog` entry. `MissingSession` and `UnknownSession` do not
  collapse.
- Closed `SessionId` rejects at stage `SessionGate` with reason
  `ClosedSession`; the rejection appears in that closed session's
  `SessionEventLog` (closed sessions remain queryable as provenance).
- Valid open `SessionId` passes the session gate and reaches
  `HeadInbox` for normal admission.

### Attribution and admission

- An accepted intent is attributed to exactly one `SessionId`.
- An accepted intent's `IntentAccepted` event appears in that session's
  `SessionEventLog`.
- `IntentRejected` records `stage` and `reason` for every refusal.
- `BaseHeadMismatch` remains the existing `HeadInbox` / worldline
  admission behavior; the rejection is attributed to the valid
  submitting session and appears in that session's `SessionEventLog`
  with stage `HeadInboxAdmission`.

### SessionEventLog

- `SessionEventLog(session_id)` orders entries by the engine event
  log's deterministic event order (`LogicalTime` / event sequence with
  stable content-address tie-breaker), not by mutation order.
- Querying `SessionEventLog` for an unknown `SessionId` does not
  return a partial result derived from `MissingSession` /
  `UnknownSession` rejections (those rejections are engine-level, not
  session-level).

### runUntilIdle

- `runUntilIdle(session_id, until: receipt)` returns when every
  in-flight accepted intent attributed to that session has emitted
  `IntentReceiptIssued`, regardless of effect rippling.
- `runUntilIdle(session_id, until: quiescent)` returns only when every
  in-flight accepted intent has additionally reached
  `IntentEffectsQuiesced`.
- Late bounded-child registration — attempting to register bounded
  child work for an intent that has already emitted
  `IntentEffectsQuiesced` — is rejected. (The one-way quiescence gate.)

### Lifecycle and close

- `SessionOpened`, `SessionCloseRequested`, `SessionClosing`,
  `SessionClosed` events appear in expected order.
- Graceful close drains accepted bounded work to quiescence before
  `SessionClosed` fires.
- Abortive close cancels interruptible accepted work, quiesces
  cancellation receipts, and ends at `SessionClosed`.
- Abortive close v1 does _not_ cancel pending `HeadInbox` envelopes
  (the deferral is asserted explicitly, not silently observed).
- `SessionClosed` implies no accepted in-flight bounded work remains
  for that session.

### System sessions

- `system/genesis` exists at engine boot without an originating
  intent and carries a concrete `PrincipalRef::system("genesis")`-shaped
  identity (no null principal).
- `system/bootstrap` is opened by an Open intent submitted under
  `system/genesis`; its `SessionOpened` event is attributed to
  `system/genesis`.
- Other system sessions follow the same pattern. There is no null
  / default session anywhere in the system.

### Wire and decode boundary

- An `IngressAddress` wire value round-trips through the 0024 LE binary
  codec without loss for every variant
  (`DefaultWriter { worldline_id }`, `InboxAddress { worldline_id, inbox }`,
  `ExactHead { key }`).
- The decode boundary maps `IngressAddress` to runtime
  `warp-core::head_inbox::IngressTarget`. The wire does not serialize
  `IngressTarget` directly. Tests for the wire surface assert on
  `IngressAddress`; tests for admission assert on `IngressTarget`.
- A multi-target / atomic-batch submission is rejected in v1 (covers
  the deferred multi-target invariant).

### Unification with `playback::SessionId`

- `playback::ViewSession` continues to function attached to the
  unified `SessionId` (no regression on the existing read-side
  concept).
- No second `SessionId` concept exists in `warp-core` after this
  cycle's implementation. Grep / static check that catches this is
  acceptable.

---

## What this handoff is not

- Not a code change. No Rust, no Wesley IR edits, no test files.
- Not an API surface commitment. Type signatures named in this doc
  (`Session::new`, `IntentRejected::stage`, etc.) are illustrative;
  Phase 2 picks the exact shape.
- Not authorization to implement. Phase 2 RED begins only with
  explicit implementation greenlight; the design conversation that
  produced this cycle was scoped to design only.

---

## How Phase 2 should start

1. Read `design.md` end-to-end. Confirm the locked decisions above
   match the doc; if any drift is found, the discrepancy is a design
   bug to be fixed before Phase 2 begins.
2. Resolve the five pre-RED decisions above with explicit answers
   captured in `phase-2-notes.md` (or by amending `design.md` if the
   resolution rises to the design level).
3. Reconcile the `wesley-core` dependency pinning. Echo currently
   pins `wesley-core = 0.0.4` from crates.io; wesley trunk has
   moved. Session as a node type means Wesley emits matter; the
   pinning must be sorted before the schema lands or generated
   types will diverge between echo and consumers (jedit).
4. Write RED Rust tests in `crates/warp-core/tests/` (or wherever
   convention places them) for each row in the RED matrix above.
   Confirm they fail (compile failure on missing types is the
   literal RED state).
5. Proceed to Phase 2 GREEN: implement Session, the admission gate,
   `SessionEventLog`, `runUntilIdle`, lifecycle events,
   `IngressAddress` codec, and the `playback::SessionId`
   unification, until RED tests pass.
6. Phase 3 (playback) and Phase 4 (close) follow METHOD.

The jedit migration card
(`jedit/docs/method/backlog/asap/sessions-migration.md`) consumes
this cycle's output and tracks the client-side migration off
`JeditWorldlineSessionPort`. It is downstream of Phase 2 GREEN.

---

## Provenance

This handoff was produced in the same conversation as the Phase 1
design, immediately after Phase 1 was approved. Path 1 (stop after
Phase 1, hand off Phase 2 cleanly) was chosen explicitly to keep API
surface introduction out of a design conversation. Phase 2 RED stubs
in any form — even `todo!()` placeholders — would have crossed that
line.
