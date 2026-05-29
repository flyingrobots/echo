<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# 0025 — Sessions as Causal Contexts

_Promote Session from a client-side scaffold to a first-class addressable
causal-context node in Echo, so coherent streams of work have a durable home
in the graph instead of being mirrored in each client._

Legend: `PLATFORM`

Depends on:

- `0024 — Universal LE Binary Codec` — EINT envelope is the wire surface
  where session addressing attaches. This cycle branches from
  `stack/echo-le-binary-codec` rather than `main`.

---

## Why this cycle exists

The jedit Slice B cutover (commit `26a8f43` on `stack/jedit-rope-rename`)
moved jedit's mutation wire from JSON-with-session to EINT envelopes carrying
only `(op_id, vars)`. To survive that, jedit had to introduce a client-side
`JeditWorldlineSessionPort` — an in-process map of `worldlineId → session` —
because Echo had no name for the thing that session represented. The
in-process transport reads from the same port the optic client writes to;
they cooperate by sharing memory.

That works because both live in the same process. It does not work across a
real WASM transport, and it forces every client to invent the same
abstraction. The smell is not jedit's; the smell is that Echo lacks a name
for the **coherent stream of causal work originating from some writer**.

The fix is to make Session a first-class concept in Echo: a durable causal
context with a writer reference, an ingress mailbox, an accepted causal
lane, and an explicit lifecycle. Intents address sessions; effects and
receipts attribute back to them; lifecycle becomes a queryable property of
the graph instead of a property of jedit's heap.

The session-port becomes a temporary compatibility bridge with a defined
deletion criterion (see `jedit/docs/method/backlog/asap/sessions-migration.md`).

---

## What Session is, and what it is not

Session is a **new primitive** that composes existing primitives rather than
replacing them. The clean separation:

| Concept          | Role                                                  |
| ---------------- | ----------------------------------------------------- |
| Writer / Agent   | who is acting                                         |
| **Session**      | coherent stream of interaction (this cycle's subject) |
| Worldline        | state lineage being edited / observed                 |
| Connection       | ephemeral transport                                   |
| Intent           | requested causal work                                 |
| Effect / Receipt | result of causal work                                 |

Session is **not** an Optic (a typed surface over data), **not** a Worldline
(a state lineage), **not** an Agent (writer identity), **not** a Connection
(transport identity). It is the missing causal context that binds those
things during a stretch of work.

### Naming collision

`echo-session-proto` currently uses "Session" in the transport-protocol
sense (WebSocket-style connection session). That crate is being split under
`PLATFORM_echo-session-proto-split` (rename of legacy transport types,
preserve the EINT/TTD framing). This cycle reclaims the unqualified
"Session" for the causal-context sense. Coordination plan: the proto-split
cycle renames its types to the connection-sense (`ConnectionSession`,
`TransportSession`, or whatever it lands on) before this cycle's Session
node ships. The two cycles must agree on the rename order; this cycle does
not block on completion of proto-split, only on its rename direction being
chosen.

---

## Session node shape (v1)

```text
Session
  id              : SessionId
  writer_ref      : PrincipalRef
  writer_label?   : String                  // human-readable, optional
  status          : open | closing | closed
  created_at      : LogicalTime
  closed_at?      : LogicalTime
  primary_worldline? : WorldlineId          // convenience, not authoritative
  lane_head?      : LaneCursor              // last accepted intent on this session's lane
```

**Writer identity** is `PrincipalRef`-shaped even in v1. The v1
implementation may carry only a label, but the type is the seam where
identity-with-teeth (signatures, capabilities, audit) eventually attaches.
A bare `String` here would be a one-way street.

**`primary_worldline`** is convenience metadata, not authoritative. Sessions
and Worldlines are orthogonal axes: a session may touch many worldlines, a
worldline may be touched by many sessions. Each intent names its target
worldline(s) explicitly; the session's `primary_worldline` is just a hint
for clients that want a default.

---

## Inbox vs lane

These are not the same thing. Conflating them is how queueing concerns get
mistaken for causal concerns.

- **Inbox** is ingress. Things submitted to the session. May be rejected,
  delayed, prioritized, or cancelled. Not part of causal history.
- **Lane** is accepted causal order. Things Echo has admitted into the
  session's causal history. Has a total order. Immutable.

The submit-then-accept seam lets us add backpressure, priority, fairness,
and cancellation later without lying about causality. v1 ships with the
seam but no scheduling sophistication on top of it.

---

## Lifecycle events

```text
SessionOpened
IntentSubmitted              // entered the inbox
IntentAccepted               // admitted onto the lane (causal commitment)
IntentRejected               // never made it onto the lane
IntentStarted                // execution began
EffectEmitted                // a downstream effect was produced
IntentReceiptIssued          // primary receipt for this intent is in
IntentEffectsQuiesced        // all bounded child work from this intent done
SessionCloseRequested        // close initiated
SessionClosing               // no new intents accepted; drain or abort
SessionClosed                // no accepted in-flight bounded work remains
```

### Why two settlement events instead of one

`IntentSettled` is rejected as a name because it carries two distinct
meanings that callers need to choose between:

- **`IntentReceiptIssued`** — the primary command has produced its direct
  receipt. Example: `replaceRangeAsTick` emitted the tick receipt.
- **`IntentEffectsQuiesced`** — all bounded downstream work causally rooted
  in that intent is complete. Example: render / re-index / checkpoint
  rebuild work spawned from the intent has finished.

A single `IntentSettled` event would force callers to either lie (declare
settled when the receipt is in even though effects are still rippling) or
overshoot (wait for all effects even when the caller only needed a
receipt). Splitting the event makes the choice explicit at the call site
instead of baked into the name.

### Bounded vs unbounded causal participation

Only **bounded** causal work participates in `EffectsQuiesced`. Long-lived
subscriptions, watchers, telemetry sinks, background daemons, and
open-ended observers do not keep an intent forever unquiesced. If such a
participant wants to register bounded child work, it does so explicitly via
the intent → effect attribution surface; otherwise it stays out of the
quiescence calculation.

---

## `runUntilIdle` semantics

```text
runUntilIdle(session_id, until: receipt | quiescent)
```

The mode is explicit and required:

- `until: receipt` — return when every in-flight intent on the session has
  produced its receipt. Fast. Suitable for interactive UI affordances and
  responsiveness-sensitive callers.
- `until: quiescent` — return when every in-flight intent has additionally
  reached `EffectsQuiesced`. Slower. Suitable for tests, deterministic
  engine calls, and sync workflows that need actual quiet.

Default modes:

- Tests, deterministic engine calls, sync workflows: `quiescent`
- Interactive UI affordances: `receipt`, unless caller asks deeper

**No fake idle.** Returning "idle" when work is still rippling is worse
than not having an idle primitive at all, because it gives bugs a hat and
a clipboard.

---

## Close semantics

Close is two-stage. The intermediate state is observable.

```text
SessionCloseRequested    // close has been asked for
SessionClosing           // no new intents accepted; in-flight work draining or being aborted
SessionClosed            // no accepted in-flight bounded work remains
```

During `SessionClosing`:

- No new intents are accepted.
- Already accepted intents remain causally attached to the session.
- Unaccepted inbox entries are rejected or cancelled.
- Accepted in-flight work either drains or is explicitly aborted, per
  close mode.

### Close modes (v1)

- **Graceful close.** Stop accepting new intents; drain accepted work to
  quiescence; then emit `SessionClosed`.
- **Abortive close.** Stop accepting new intents; cancel unstarted /
  interruptible work; emit obstruction/cancellation receipts; wait for
  cancellation quiescence; then emit `SessionClosed`.

**Detached post-close sub-lanes are out of v1.** Letting effects from a
closed session continue to land on a sentinel "post-close" sub-lane is
clever and tempting. It creates a forensic side-channel that future code
will accidentally start depending on. Out for now; revisit if a real use
case forces it.

### Invariant

`SessionClosed` means the session no longer has accepted in-flight bounded
work. If that is not true, the name is lying.

---

## Cross-session concurrency on a shared worldline (v1)

Multiple sessions may target the same worldline. v1 uses the **obstruction
model** — no automatic merge, no true concurrent worldline lanes, no
multi-writer conflict resolution.

```text
Session A edits worldline X at head H1  → produces H2
Session B tries edit against H1         → baseHeadMismatch obstruction
```

Mechanics:

- A worldline still has a current head.
- An intent that targets a worldline must name the base head it expects.
- If the base head does not match, the intent obstructs.

This is the right v1 because it preserves the path to real causal
concurrency without pretending we already built Google Docs in a trench
coat. Multi-writer concurrent merge semantics are intentionally deferred.

---

## Multi-worldline intents

The intent type admits plural targets:

```text
Intent {
  target_worldlines : NonEmptyList<WorldlineRef>
  ...
}
```

The v1 protocol enforces `target_worldlines.length == 1`. Atomic
multi-worldline transactions are deferred.

The plural type is intentional: it leaves room for future cross-buffer
atomic operations (project-wide rename, refactor-across-files) without
needing a wire-shape break. Implementing those atomically — multiple base
heads, all-or-nothing commit, cross-worldline obstruction, rollback /
compensation, lock ordering to avoid deadlock, cross-worldline receipt
semantics — is real machinery, not "just plural." Out of v1.

---

## System and headless intents

There is **no null / default session**. Every intent has a session, even
system-emitted intents. Especially system-emitted intents.

System sessions are explicit and named, e.g.:

```text
session: system/bootstrap
session: system/indexer
session: system/test-runner
session: agent/<id>/batch/<id>
```

A nullable session field would feel convenient for about a week, then
become the place causality goes to die.

---

## Durable vs ephemeral state

| Durable                      | Ephemeral / compactable    |
| ---------------------------- | -------------------------- |
| Session node                 | Raw inbox backlog          |
| Accepted intents             | Transport connection state |
| Effects                      | Transient dispatch timers  |
| Receipts                     | Retry bookkeeping          |
| Open / close lifecycle facts |                            |

Closed sessions remain queryable as provenance. Operational queue details
(raw inbox, transport bookkeeping) may be compacted or archived; the
causal skeleton stays. This distinction prevents the session graph from
becoming a dump truck.

---

## Wire surface

The EINT envelope carries addressing, not session semantics:

```text
Envelope {
  session_id     : SessionId
  intent_id      : IntentId
  correlation_id?: CorrelationId          // caller-supplied opt-in
  payload        : <vars bytes per 0024>
}
```

The engine decides everything else: is this session open, is this writer
authorized to submit here, what lane does this enter, what causal parent /
head does it attach to.

The codec does not smuggle session behavior. It serializes facts and
addresses.

**On correlation.** The session lane gives correlation a durable causal
home, but individual intent / effect correlation still needs explicit IDs.
A session can have many simultaneous or queued things happening; the lane
gives scope and ordering, not object identity. Concretely:

- `session_id` = scope
- `intent_id` = unit of requested work
- `effect_id` / `receipt_id` = result identity
- lane position = causal order

---

## Core invariants

1. Every accepted intent belongs to exactly one session.
2. Every effect / receipt references the accepted intent that caused it.
3. A session lane has a total order of accepted intents.
4. A closed session cannot accept new intents.
5. `runUntilIdle(session, quiescent)` returns true iff there is no pending
   accepted work AND no in-flight bounded child work for that session.
6. `SessionClosed` implies invariant 5 for the closed session.
7. No null / default session anywhere in the system.

---

## Human users / jobs / hills

### Primary human users

- **Engine maintainers** building on Echo's causal graph.
- **Application authors** wiring clients that submit intents (jedit being
  the proof case; future agent / IDE / browser hosts being the
  generalization).
- **Operators / auditors** inspecting what was done, by whom, in what
  order.

### Human jobs

1. Address an intent to a coherent stream of work without inventing a
   client-side session abstraction.
2. Wait on the right notion of "done" (receipt vs full quiescence) for the
   call site without lying.
3. Inspect a closed session's causal history after the fact for provenance.

### Human hill

A human can model multi-writer, multi-buffer, agent-driven work as
distinct sessions in Echo without each application reinventing a
session-port.

---

## Agent users / jobs / hills

### Primary agent users

- Agents submitting intents on behalf of a human or themselves.
- Test harnesses driving deterministic engine cycles.
- The jedit optic client (immediate consumer).

### Agent jobs

1. Open a session for a bounded piece of work, submit intents against it,
   close it cleanly.
2. Programmatically determine when a session is idle, with explicit
   knowledge of which kind of idle was asked for.
3. Address intents to multiple worldlines within one session without losing
   per-worldline obstruction semantics.

### Agent hill

An agent can submit, observe, and conclude a coherent stream of work
addressed to a single `SessionId` and programmatically determine when that
stream is fully settled in the engine's causal graph.

---

## Human playback

1. The human runs the cycle's introductory walkthrough (script TBD in
   implementation phase).
2. The output shows: session opened, three intents submitted, all three
   accepted onto the lane, receipts issued, effects quiesced, session
   closed.
3. The human can query the closed session's lane and see the total order
   of accepted intents without opening any source file.

## Agent playback

1. The agent runs `runUntilIdle(session_id, until: quiescent)` after
   submitting a batch.
2. The output contains the session's `SessionClosed` event (or a
   confirmation that all bounded child work is done in the open-session
   case).
3. The agent determines that no further effects rooted in that session's
   intents will be emitted.

---

## Implementation outline

This is the design phase. The implementation outline below is **proposed**;
no code lands in this cycle's Phase 1.

1. Define `SessionId`, `PrincipalRef`, `LaneCursor`, `IntentId`,
   `EffectId`, `ReceiptId`, `CorrelationId` as Wesley-IR types so all
   languages emit identical shapes.
2. Add the Session node type to the causal graph schema.
3. Add the lifecycle event set as first-class events with stable IDs.
4. Extend EINT envelope shape (within 0024's encoding doctrine) to carry
   `session_id` and `intent_id` headers.
5. Implement the inbox / lane split, with v1 carrying no priority or
   fairness machinery.
6. Implement two-stage close with graceful and abortive modes.
7. Implement `runUntilIdle(session, until)` against the lifecycle event
   stream.
8. Add system sessions as a bootstrap concern (`system/bootstrap`,
   `system/indexer`, etc.).
9. Document the migration path for clients in companion docs
   (`jedit/docs/method/backlog/asap/sessions-migration.md`).
10. Deprecate jedit's `JeditWorldlineSessionPort` once the engine surface
    is available; remove it once jedit threads `SessionId` through its
    optic-client surface.

---

## Tests to write first

To be expanded in Phase 2. Minimum coverage required:

- A submitted intent appears on the lane in submit order, with `Accepted`
  preceding `ReceiptIssued`.
- A rejected intent never appears on the lane.
- `runUntilIdle(session, receipt)` returns when all in-flight intents
  have `ReceiptIssued`, even if effects are still rippling.
- `runUntilIdle(session, quiescent)` returns only when all in-flight
  intents have `EffectsQuiesced`.
- Graceful close drains; abortive close cancels and quiesces cancellation
  receipts; both end at `SessionClosed`.
- Two sessions targeting the same worldline: second-writer obstructs with
  `baseHeadMismatch`.
- An intent submitted to a closed session is rejected at the inbox, never
  reaches the lane.
- A `null` / missing session field on an envelope is a hard reject
  (covers the no-null-default invariant).
- An intent with `target_worldlines.length > 1` is rejected in v1
  (covers the deferred multi-worldline invariant).

---

## Risks / unknowns

- **Wire shape break.** EINT envelopes already exist (0024). Adding
  `session_id` to the header is a breaking change. Coordination with
  `0024` matters: ideally the Session header is part of EINT v2 or the
  next EINT revision, not a hot patch.
- **Naming collision with `echo-session-proto`.** Addressed above; the
  rename must happen in the right order. If proto-split lags, this cycle's
  "Session" name lands ambiguously for a window.
- **System session bootstrapping.** If every intent requires a session,
  the bootstrap itself needs a session to exist before the first user
  intent. Concretely: how does `system/bootstrap` come into being? It
  must be a primordial node, established by genesis, not by an intent.
  Design needs to spell this out before implementation.
- **Cross-session worldline interaction beyond v1.** The deferral is
  honest, but the underlying problem (multi-writer concurrent merge) is
  fundamental to causal computing. Deferring it is correct; pretending it
  is "later work" rather than "an open research direction" would be
  dishonest.
- **Wesley schema dependency.** Session as a node type means it appears
  in Wesley-generated contracts. The wesley-core dependency situation
  (echo currently pins `wesley-core = 0.0.4` from crates.io; wesley
  trunk has moved) needs to be reconciled before this cycle's
  implementation phase, otherwise the generated session types diverge.

---

## Postures

- **Accessibility:** Not applicable. Internal architecture cycle; no
  user-facing surface.
- **Localization:** Not applicable. Same reason.
- **Agent inspectability:** Strong. The whole point of Session as a
  causal-context node is that agents can query session state, lane
  contents, and quiescence via the standard graph surface. No special
  inspection API required.

---

## Non-goals

- Multi-writer concurrent merge on a single worldline (deferred; v1 is
  obstruction-only).
- Atomic multi-worldline intents (type admits plural; v1 enforces
  singleton).
- Backpressure, fairness, priority, or capability enforcement on the
  inbox (seam exists; policies do not).
- Replacing or renaming `echo-session-proto` (coordinated in its own
  cycle).
- Sub-sessions, session forking, session merging (potentially interesting,
  out of v1).
- Authentication and authorization of writers (`PrincipalRef` is
  forward-compatible; v1 carries label only).
- Detached post-close sub-lanes for effects landing after `SessionClosed`
  (forensic side-channel risk; explicitly out).
- Garbage-collecting closed sessions (closed sessions are durable
  provenance; only operational tail is compactable).

---

## Open questions for review

1. Is `PrincipalRef` already a named shape in Echo, or are we introducing
   it here? If introducing, does it belong to this cycle or to a separate
   identity cycle that this one depends on?
2. Is the EINT envelope shape modification additive or breaking? If
   breaking, does it ride 0024 directly or chain after it?
3. System sessions: established at genesis, or established by a
   bootstrap intent submitted to a primordial session? Recursion concern.
4. Does the existing causal graph have a notion of "actor-scoped lane"
   that Session subsumes, or is the lane structure new to this cycle?
5. The `PLATFORM_echo-session-proto-split` cycle is in `up-next/`. Should
   it be pulled and renamed before this cycle's implementation phase to
   eliminate the naming-collision window?

---

## Source

This design was extracted from a cross-repo conversation rooted in the
jedit Slice B EINT cutover (jedit commit `26a8f43`,
`stack/jedit-rope-rename`). The request stub is in `request.md`. The
jedit-side migration consequences are documented in
`jedit/docs/method/backlog/asap/sessions-migration.md`.
