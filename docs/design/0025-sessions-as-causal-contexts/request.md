<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Sessions as Causal Contexts

Status: active sequencing card. Extension of the 0024 LE binary cutover.

## Why now

The jedit Slice B EINT cutover (jedit commit `26a8f43`, echo branch
`stack/echo-le-binary-codec`) surfaced a structural smell: the wire was
carrying jedit-internal session state because Echo had no name for the thing
the session represented — a coherent causal stream of work originating from
some writer. The fix in Slice B was a client-side `JeditWorldlineSessionPort`
(in-process map of `worldlineId → session`). That is a scaffold, not an
abstraction. It works because the optic client and the in-process transport
share the same memory; it cannot ship across a real WASM transport, and it
duplicates engine state on the client.

The honest fix is to make **Session** a first-class concept in Echo: a durable
causal actor / context node with a writer reference, an ingress mailbox, an
accepted causal lane, and an explicit lifecycle. Jedit then holds only a
`SessionId` and threads it through intent submission. The session-port becomes
a temporary compatibility bridge slated for deletion.

## What Session is

| Concept          | Role                                  |
| ---------------- | ------------------------------------- |
| Writer / Agent   | who is acting                         |
| **Session**      | coherent stream of interaction        |
| Worldline        | state lineage being edited / observed |
| Connection       | ephemeral transport                   |
| Intent           | requested causal work                 |
| Effect / Receipt | result of causal work                 |

Those layers stay separate. Mixing them is the bug class this cycle eliminates.

## Why this rides 0024 rather than waiting

The 0024 universal LE binary codec is mid-flight (`stack/echo-le-binary-codec`,
+7 ahead of `origin/main`). The wire shape under discussion in 0024 — EINT
envelopes carrying `(op_id, vars)` — is exactly the surface where session
addressing would be added. Deferring Session to a post-0024 cycle would mean
either rebuilding the EINT envelope shape twice or accumulating debt that
specifically blocks the jedit observe-path cutover.

This cycle therefore opens on a branch derived from `stack/echo-le-binary-codec`
rather than `main`. The 0024 stack lands first or in parallel; this cycle
extends it.

## Acceptance criteria

- Design doc 0025 exists at `docs/design/0025-sessions-as-causal-contexts/`
  and defines Session as a first-class causal-context node distinct from
  Writer, Worldline, Connection, and Optic.
- Lifecycle event set is named and bounded: `SessionOpened`,
  `IntentSubmitted`, `IntentAccepted`, `IntentRejected`, `IntentStarted`,
  `EffectEmitted`, `IntentReceiptIssued`, `IntentEffectsQuiesced`,
  `SessionCloseRequested`, `SessionClosing`, `SessionClosed`.
- `IntentSettled` is explicitly rejected as ambiguous in favor of the
  `ReceiptIssued` / `EffectsQuiesced` split.
- `runUntilIdle(session_id, until: receipt | quiescent)` semantics are
  defined with explicit caller modes — no fake idle.
- Two-stage close (`graceful` vs `abortive`) is specified; detached
  post-close sub-lanes are explicitly out of v1.
- Cross-session concurrency on a shared worldline is explicitly v1 =
  obstruction-only (base-head mismatch). True concurrent worldline lanes
  are deferred and named as deferred.
- `Intent.target_worldlines` is typed as `NonEmptyList<WorldlineRef>` with a
  v1 invariant of length == 1. Atomic multi-worldline transactions are
  deferred and named as deferred.
- Writer identity uses a `PrincipalRef`-shaped forward-compatible type even
  if v1 carries only a label.
- Naming collision with existing `echo-session-proto` (transport-protocol
  sense) is acknowledged and a disambiguation strategy is stated.
- Jedit migration consequences are documented in
  `jedit/docs/method/backlog/asap/sessions-migration.md` and link back to
  this packet.

## Non-goals (v1)

- Multi-writer concurrent merge on a single worldline.
- Atomic multi-worldline intents.
- Backpressure, fairness, priority, or capability enforcement on the inbox.
- Replacing the existing `echo-session-proto` transport types in this cycle
  (a separate cycle, `echo-session-proto-split`, already exists for that
  rename and will be coordinated with this naming reclamation).

## Dependencies

- `0024 — Universal LE Binary Codec` — the EINT envelope shape this cycle
  extends. Cycle branches from `stack/echo-le-binary-codec`.

## Source

This card was created from a cross-repo design conversation rooted in the
jedit Slice B cutover. The full prose record of the design decisions is
captured directly in the design packet that follows the pull.
