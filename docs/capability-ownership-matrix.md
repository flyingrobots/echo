<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Capability Ownership Matrix

Date: 2026-01-01
Status: Draft (Phase 0.75)

This document is a living boundary map for Echo.

It answers (explicitly, in one place):

- Who **owns** vs **consumes** each capability?
- What determinism level is required at each layer?
- What provenance is required to make replay / time travel honest?
- Which external dependencies (clocks, OS IO, networks) are allowed to influence state, and how?

It is intentionally redundant with specs: the point is to keep the architecture legible while it is evolving.

---

## Layers (Echo interpretation)

Use these columns consistently:

- **Platform**: host integration and durable artifacts/contracts (process, filesystem, sockets, timers, OS scheduling, worldline storage, commit hashing, tick patch format, digests). Nondeterministic by default.
- **Kernel**: deterministic semantic core (rewrite engine, scheduler, receipts, snapshot/tick structure, deterministic decision records including stream admission decisions). HistoryTime-only.
- **Views**: controlled accessors and projections over history (query APIs, inspectors, adapters). Any interaction with HostTime/IO must be recorded as replay-safe claims/decisions.
- **Tooling**: UIs, dashboards, CLI workflows (read-only by default; must be usable during pause/rewind; any control surface must be capability-gated and recorded).
- **Docs**: specs and procedures; the "human-facing API".

---

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

## Matrix Template (Copy/Paste)

For each capability × layer, fill the cell with:

- `Role`: owns | consumes
- `Stability`: experimental | beta | stable
- `Determinism`: none | best-effort | deterministic (replayable)
- `Provenance`: none | basic (timestamps) | strong (CID/hash-linked)
- `External Deps`: libs, services, clocks, networks, etc.

Cell format:

```text
Role: owns | consumes
Stability: experimental | beta | stable
Determinism: none | best-effort | deterministic
Provenance: none | basic | strong (CID/hash)
External Deps: <list or “none”>
```

---

## First-Pass Fill (Current Echo stack)

This is a starter fill that we will revise as Echo components stabilize.

Legend (compact):

- `owns/consumes`
- `exp/beta/stable`
- `det/best/none`
- `prov: strong/basic/none`

| Capability ↓ \ Layer → | Platform | Kernel | Views | Tooling | Docs |
| --- | --- | --- | --- | --- | --- |
| **Scheduling** | owns · beta · best · prov: basic · deps: OS scheduler, tokio | owns · beta · det · prov: strong · deps: none | consumes · beta · det · prov: strong · deps: none | consumes · beta · best · prov: basic · deps: browser/event-loop | owns · stable · det · prov: strong · deps: none |
| **Provenance** | consumes · beta · best · prov: basic · deps: FS/network | owns · beta · det · prov: strong · deps: CID/hash | consumes · beta · det · prov: strong · deps: none | consumes · beta · det · prov: strong · deps: none | owns · stable · det · prov: strong · deps: git |
| **Schema / Interfaces** | consumes · exp · best · prov: basic · deps: serde/json | owns · exp · det · prov: strong · deps: versioned schemas | owns · exp · det · prov: strong · deps: schema hash pinning | consumes · exp · best · prov: basic · deps: UI contracts | owns · beta · det · prov: strong · deps: docs/specs |
| **Storage / Ledger** | owns · beta · best · prov: basic · deps: FS/DB | owns · exp · det · prov: strong · deps: content hashing | consumes · beta · det · prov: strong · deps: read-only ledger | consumes · beta · best · prov: basic · deps: localStorage/IndexedDB | owns · beta · det · prov: strong · deps: docs/specs |
| **Time / Clocks** | owns · beta · best · prov: basic · deps: HostTime | consumes · beta · det · prov: strong · deps: Decision Records | consumes · beta · det · prov: strong · deps: Clock View | consumes · beta · best · prov: basic · deps: tool clock | owns · beta · det · prov: strong · deps: paper/spec |
| **Networking / IO** | owns · beta · best · prov: basic · deps: TCP/WS/UDS | consumes · exp · det · prov: strong · deps: recorded claims | consumes · beta · det · prov: strong · deps: stream backlog | consumes · beta · best · prov: basic · deps: Web APIs | owns · beta · det · prov: strong · deps: procedures |
| **Auth / Trust** | owns · exp · best · prov: basic · deps: keys/tokens | consumes · exp · det · prov: strong · deps: signed claims | consumes · exp · det · prov: strong · deps: receipts | consumes · exp · best · prov: basic · deps: auth UI | owns · exp · det · prov: strong · deps: policies |
| **Observability** | owns · beta · best · prov: basic · deps: logs/metrics | owns · beta · det · prov: strong · deps: receipts/events | owns · beta · det · prov: strong · deps: query/index | owns · beta · best · prov: basic · deps: UI | owns · beta · det · prov: strong · deps: docs |
| **Replay / Debug** | consumes · beta · best · prov: basic · deps: host capture | owns · beta · det · prov: strong · deps: replay log | owns · beta · det · prov: strong · deps: index | owns · beta · det · prov: strong · deps: dashboard | owns · beta · det · prov: strong · deps: paper/spec |
| **Shared Invariants** | - | - | - | - | - |

---

## Shared Invariants (Draft)

These are the guarantees that must hold across layers if we want deterministic replay and time travel to be “honest”:

1. **Deterministic Core**: kernel state transitions are pure functions of `(prior state, admitted inputs, pinned rule-pack / schema hashes)`; no HostTime/OS IO calls inside kernel semantic transitions.
2. **Time As Data**: kernel never consults HostTime directly; HostTime is only observed in Platform/Views and converted into Decision Records (HistoryTime) before it can influence semantics.
3. **Provenance First**: all externally meaningful artifacts (schemas, policies, rule packs) are referenced by content hash (CID) in receipts.
4. **Network Boundary**: IO is treated as external stimuli; any nondeterministic observation is recorded as a claim before it can affect semantic state.
5. **Replay Integrity**: if semantics change (schema/compiler), history carries a version/hash pin (fail closed or migrate deterministically).

---

## Notes / Follow-Ups

- This matrix should become part of the “phase overview” review checklist: when a capability moves from experimental → beta, update the cell and link evidence (PRs/specs/tests).
- When we formalize Wesley and/or a view grammar, split “Schema / Interfaces” into: boundary grammar, IR schema pinning, and codegen outputs.

## Near-Term TODOs

- (#174) Decide where “Wesley grammar/IR” lives in this matrix (Platform vs Schema layer), and whether its schema hash is required on all receipts.
- (#170) Specify the `StreamsFrame` inspector payload (backlog, cursors, `StreamAdmissionDecision` summaries).
