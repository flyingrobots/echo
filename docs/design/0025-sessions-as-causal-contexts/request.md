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
causal-context node with a principal reference, a lifecycle gate, an
attribution surface on every intent / effect / receipt, and a queryable
event projection. Session does **not** own an ingress queue (the existing
`HeadInbox` does) and does **not** own a causal-ordering lane (the existing
worldline head / tick / strand / braid machinery does). Jedit then holds
only a `SessionId` and threads it through intent submission. The
session-port becomes a temporary compatibility bridge slated for deletion.

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
  Writer/Principal, Worldline, Connection, and Optic.
- Session integrates with existing warp-core primitives rather than
  introducing parallels: ingress remains owned by `head_inbox.rs`
  (`HeadInbox` / `IngressTarget`); mutation ordering remains owned by
  worldline head / tick / strand / braid; `SessionId` is unified with
  the existing `playback::SessionId`.
- Lifecycle event set is named and bounded: `SessionOpened`,
  `IntentSubmitted`, `IntentAccepted`, `IntentRejected`, `IntentStarted`,
  `EffectEmitted`, `IntentReceiptIssued`, `IntentEffectsQuiesced`,
  `SessionCloseRequested`, `SessionClosing`, `SessionClosed`.
- `IntentSettled` is explicitly rejected as ambiguous in favor of the
  `ReceiptIssued` / `EffectsQuiesced` split.
- `IntentRejected` records `stage` (`Decode` / `SessionGate` /
  `HeadInboxAdmission` / `Execution`) and `reason` (`MissingSession` /
  `UnknownSession` / `ClosedSession` / `CapabilityDenied` /
  `BaseHeadMismatch` / `Cancelled` / …). `MissingSession` and
  `UnknownSession` do not silently collapse.
- `runUntilIdle(session_id, until: receipt | quiescent)` semantics are
  defined with explicit caller modes — no fake idle.
- `IntentEffectsQuiesced` is a one-way gate per intent; late
  bounded-child registration is rejected.
- Two-stage close (`graceful` vs `abortive`) is specified; detached
  post-close sub-lanes are explicitly out of v1; abortive close v1 does
  NOT cancel pending `HeadInbox` envelopes (the deferral is honest).
- Cross-session concurrency on a shared worldline is explicitly v1 =
  obstruction-only (base-head mismatch). True concurrent worldline lanes
  are deferred and named as deferred.
- V1 routing is single-target: submissions carry one `IngressTarget` from
  existing warp-core. Atomic multi-target submissions are deferred to a
  future cycle along with their shape and semantics.
- Principal identity uses the existing `PrincipalRef` from
  `warp-core::optic_artifact`; v1 may carry identity binding only without
  capability-enforcement teeth.
- `system/genesis` is primordial (created at genesis, not by an intent)
  and carries a concrete `PrincipalRef::system("genesis")`-shaped
  identity. No null session and no null principal anywhere.
- The EINT envelope `target` field is an `IngressAddress`-shaped protocol
  value (serialized through 0024) that the decode boundary maps to
  runtime `IngressTarget`. The wire does not serialize the Rust enum
  directly.
- Jedit migration consequences are documented in
  `jedit/docs/method/backlog/asap/sessions-migration.md` and link back to
  this packet.

## Non-goals (v1)

- A second ingress queue (`HeadInbox` remains the sole one).
- A second mutation-ordering lane (worldline head / tick / strand / braid
  remain authoritative).
- A second `SessionId` concept (`playback::SessionId` is unified, not
  duplicated).
- Multi-writer concurrent merge on a single worldline.
- Atomic multi-target submissions (v1 type is singular `IngressTarget`;
  future cycle picks both shape and semantics together).
- Backpressure, fairness, or priority policies on ingress.
- Capability-enforcement teeth on `PrincipalRef` (the seam is preserved;
  v1 may carry identity binding only).
- Replacing or renaming `echo-session-proto` — retired entirely (moved to
  graveyard); the unqualified name "Session" is reclaimed for the
  causal-context sense defined here.
- Detached post-close sub-lanes; sub-sessions / forking / merging;
  garbage-collecting closed sessions (all explicitly out of v1).

## Dependencies

- `0024 — Universal LE Binary Codec` — the EINT envelope shape this cycle
  extends. Cycle branches from `stack/echo-le-binary-codec`.

## Source

This card was created from a cross-repo design conversation rooted in the
jedit Slice B cutover. The full prose record of the design decisions is
captured directly in the design packet that follows the pull.
