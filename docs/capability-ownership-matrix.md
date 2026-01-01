<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Capability Ownership Matrix

Date: 2026-01-01  
Status: Draft (Phase 0.75)

Purpose: clarify “who owns what” across Echo’s layers, and lock down determinism/provenance expectations per capability.

This is not a bureaucracy artifact — it is a guardrail against accidental nondeterminism and boundary drift.

---

## Layers (quick definitions)

- **Platform**: durable artifacts + contracts (worldline storage, commit hashing, tick patch format, digests).
- **Kernel**: deterministic rewrite engine, scheduler, state transitions (HistoryTime-only).
- **Views**: controlled accessors/adapters that expose resources (clock, io, network, KV) as replay-safe claims/facts.
- **Tooling**: inspector UI, dashboards, diff viewers; read-only by default; capability-gated control surfaces.
- **Docs**: specs, decision log, guides; the “truth narrative” for implementers.

## Ratings

- **Determinism**
  - `none`: may vary per run; not replayable
  - `best-effort`: tries to be stable but not a contract
  - `deterministic`: replayable given pinned artifacts/inputs
- **Provenance**
  - `none`: no tracking
  - `basic`: timestamps/ids only
  - `strong`: hash/CID-linked; replay/audit friendly

---

## Matrix

Each cell uses:

`Role • Stability • Determinism • Provenance • External Deps`

| Capability ↓ \ Layer → | Platform | Kernel | Views | Tooling | Docs |
| --- | --- | --- | --- | --- | --- |
| **Scheduling** | owns • stable • deterministic • strong • deps: none | owns • stable • deterministic • strong • deps: none | consumes • beta • deterministic • strong • deps: none | consumes • beta • deterministic • basic • deps: none | owns • stable • deterministic • strong • deps: none |
| **Provenance (worldlines, ticks, receipts)** | owns • stable • deterministic • strong • deps: content hashing | consumes • stable • deterministic • strong • deps: none | consumes • beta • deterministic • strong • deps: none | consumes • beta • deterministic • basic/strong • deps: none | owns • stable • deterministic • strong • deps: none |
| **Schema / Interfaces** | owns • beta • deterministic • strong • deps: canonical encoding | consumes • beta • deterministic • strong • deps: none | consumes • beta • deterministic • strong • deps: none | consumes • beta • best-effort • basic • deps: browser/runtime UI | owns • beta • deterministic • strong • deps: none |
| **Storage / Ledger** | owns • beta • deterministic • strong • deps: fs/db | consumes • beta • deterministic • strong • deps: none | consumes • beta • deterministic • strong • deps: none | consumes • beta • best-effort • basic • deps: browser | owns • beta • deterministic • strong • deps: none |
| **Time / Clocks** | owns • beta • deterministic • strong • deps: digests | consumes • stable • deterministic • strong • deps: none | owns • beta • deterministic • strong • deps: HostTime access (gated) | consumes • beta • best-effort • basic • deps: browser clocks | owns • beta • deterministic • strong • deps: none |
| **Networking / IO** | owns • beta • deterministic • strong • deps: wire codecs | consumes • beta • deterministic • strong • deps: none | owns • beta • best-effort→deterministic via decisions • strong • deps: OS sockets | consumes • beta • best-effort • basic • deps: browser/network | owns • beta • deterministic • strong • deps: none |
| **Auth / Trust (capabilities)** | owns • beta • deterministic • strong • deps: crypto primitives | consumes • beta • deterministic • strong • deps: none | consumes • beta • deterministic • strong • deps: keystore/session | consumes • beta • best-effort • basic • deps: browser storage | owns • beta • deterministic • strong • deps: none |
| **Observability (metrics, inspector frames)** | owns • beta • deterministic • strong • deps: logging | produces • beta • deterministic • strong • deps: none | produces • beta • deterministic • strong • deps: adapters | consumes • beta • best-effort • basic/strong • deps: browser | owns • beta • deterministic • strong • deps: none |
| **Replay / Debug (time travel)** | owns • beta • deterministic • strong • deps: checkpoints/wormholes | consumes • beta • deterministic • strong • deps: none | owns • beta • deterministic • strong • deps: stream spools | owns • beta • best-effort UI / deterministic data • strong • deps: browser | owns • beta • deterministic • strong • deps: none |
| **Final Row: Shared Invariants** | **owns** | **owns** | **owns** | **consumes** | **owns** |

---

## Shared Invariants (locked)

These invariants are cross-cutting and should remain true regardless of implementation details.

1) **Deterministic Core**
- Kernel state transitions are pure functions of (prior state, admitted facts, pinned rule-pack/policy artifacts).
- No HostTime/OS IO calls inside kernel semantic transitions.

2) **Time As Data**
- HostTime can be consulted only in Views/adapters.
- Any HostTime consultation that affects semantics must emit a decision record (pinned by `policy_hash`) into history.

3) **Admission Is HistoryTime**
- Stream admission is a deterministic, auditable artifact (`StreamAdmissionDecision`).
- Admission integrity is pinned via `admission_digest` on snapshots (see `docs/spec-merkle-commit.md`).

4) **Provenance First**
- Important artifacts (tick patches, policies, rule packs, schema IR) are referenced by hash/CID.
- Tooling must be able to answer “who/what/why produced this value?” from history.

5) **Read-Only Tooling by Default**
- Inspector/tooling is read-only unless explicitly capability-authorized.
- Any mutation/control surface must be recorded as an event/decision in history (replay-safe).

---

## Near-Term TODOs

- Decide where “Wesley grammar/IR” lives in this matrix (Platform vs Schema layer), and whether its schema hash is required on all receipts.
- Specify the `StreamsFrame` inspector payload (backlog, cursors, `StreamAdmissionDecision` summaries).

