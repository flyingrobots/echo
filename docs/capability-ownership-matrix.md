<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
# Capability Ownership Matrix

This document is a **living boundary map** for Echo.

It answers (explicitly, in one place):

- Who **owns** vs **consumes** each capability?
- What is the determinism requirement at each layer?
- What provenance is required to make replay / time travel honest?
- Which external dependencies (clocks, OS IO, networks) are allowed to influence state, and **how**?

It is intentionally redundant with specs: the point is to keep the architecture legible while it is evolving.

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

## Layers (Echo interpretation)

Use these columns consistently:

- **Platform**: host integration (process, filesystem, sockets, timers, OS scheduling). Nondeterministic by default.
- **Kernel**: deterministic semantic core (graph rewrite, receipts, snapshot/tick structure).
- **Views**: materialized/queryable projections over history (SWS, inspectors, query APIs).
- **Tooling**: UIs, dashboards, CLI workflows (must be usable during pause/rewind).
- **Docs**: specs, decision log, procedures; the “human-facing API”.

---

## First-Pass Fill (Current Echo stack)

This is a *starter* fill that we will revise as Echo components stabilize.

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
| **Storage / Ledger** | owns · beta · best · prov: basic · deps: FS/DB | owns · exp · det · prov: strong · deps: content hashing | consumes · beta · det · prov: strong · deps: read-only ledger | consumes · beta · best · prov: basic · deps: localStorage/IndexedDB | owns · beta · det · prov: strong · deps: docs/decision-log |
| **Time / Clocks** | owns · beta · best · prov: basic · deps: HostTime | consumes · beta · det · prov: strong · deps: Decision Records | consumes · beta · det · prov: strong · deps: Clock View | consumes · beta · best · prov: basic · deps: tool clock | owns · beta · det · prov: strong · deps: paper/spec |
| **Networking / IO** | owns · beta · best · prov: basic · deps: TCP/WS/UDS | consumes · exp · det · prov: strong · deps: recorded claims | consumes · beta · det · prov: strong · deps: stream backlog | consumes · beta · best · prov: basic · deps: Web APIs | owns · beta · det · prov: strong · deps: procedures |
| **Auth / Trust** | owns · exp · best · prov: basic · deps: keys/tokens | consumes · exp · det · prov: strong · deps: signed claims | consumes · exp · det · prov: strong · deps: receipts | consumes · exp · best · prov: basic · deps: auth UI | owns · exp · det · prov: strong · deps: policies |
| **Observability** | owns · beta · best · prov: basic · deps: logs/metrics | owns · beta · det · prov: strong · deps: receipts/events | owns · beta · det · prov: strong · deps: query/index | owns · beta · best · prov: basic · deps: UI | owns · beta · det · prov: strong · deps: docs |
| **Replay / Debug** | consumes · beta · best · prov: basic · deps: host capture | owns · beta · det · prov: strong · deps: replay log | owns · beta · det · prov: strong · deps: index | owns · beta · det · prov: strong · deps: dashboard | owns · beta · det · prov: strong · deps: paper/spec |
| **Shared Invariants** | - | - | - | - | - |

---

## Shared Invariants (Draft)

These are the guarantees that must hold across layers if we want deterministic replay and time travel to be “honest”:

1. **Deterministic Core**: kernel state transitions are pure functions of `(prior state, admitted inputs, pinned rule-pack / schema hashes)`.
2. **Time As Data**: kernel never consults HostTime directly; HostTime is only observed in Platform and converted into **Decision Records** (HistoryTime).
3. **Provenance First**: all externally meaningful artifacts (schemas, policies, rule packs) are referenced by content hash (CID) in receipts.
4. **Network Boundary**: IO is treated as external stimuli; any nondeterministic observation is recorded as a claim before it can affect semantic state.
5. **Replay Integrity**: if semantics change (schema/compiler), history must carry a version/hash pin (fail closed or migrate deterministically).

---

## Notes / Follow-Ups

- This matrix should become part of the “phase overview” review checklist: when a capability moves from experimental → beta, update the cell and link evidence (PRs/specs/tests).
- When we formalize Wesley and/or a view grammar, split “Schema / Interfaces” into: boundary grammar, IR schema pinning, and codegen outputs.
