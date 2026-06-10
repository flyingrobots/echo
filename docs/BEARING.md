<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# BEARING

Current bearing: make Echo a generic deterministic runtime for lawful,
witnessed, replayable causal history. Echo should prove itself through real
application pressure without importing application nouns into core runtime APIs.

This document is intentionally compact. Historical slice ledgers belong in
design docs, retros, pull requests, release notes, and git history.

## Current Truth

- Echo owns admission, scheduling, ticks, receipts, readings, retained evidence,
  WSC/WAL posture, and generic contract hosting.
- Application code submits canonical intent and observes bounded readings. It
  does not tick Echo, stage scheduler work, install packages, or recover runtime
  faults.
- Wesley-authored contracts and generated artifacts carry application nouns.
  Echo core remains generic.
- The contract-host path has generic external-consumer proof for mutation,
  QueryView reading, retained evidence, compatibility checks, trusted-host
  loops, and local replay/DIND posture.
- `v0.1.0` is gated by the sibling jedit proof, not by an in-repo toy fixture.
- jedit has progressed from fake transport pressure into an Echo-hosted product
  text session with WSC history/export/replay surfaces, but the final release
  claim still depends on retained evidence, durable replay, and app-owned
  generated contract follow-through staying honest.
- Causal WAL/WSC durability remains the next hard boundary: Echo may only claim
  what it can recover.

## Source Of Truth

Authority flows in this order:

1. Runtime behavior, generated artifacts, retained evidence, and release-gate
   output.
2. Cargo tests, DIND/replay witnesses, CLI reports, and command output.
3. GitHub issues, pull requests, commits, and review threads.
4. Design docs, specs, and roadmap docs.
5. This bearing note.
6. Coordination memory.

Memory and this file are coordination aids. They do not override source,
commands, generated output, tests, GitHub state, or committed artifacts.

## Roadmap Anchors

| Area                          | Current anchor                                                                                      |
| ----------------------------- | --------------------------------------------------------------------------------------------------- |
| Docs index                    | [`docs/index.md`](index.md)                                                                         |
| Architecture outline          | [`docs/architecture/outline.md`](architecture/outline.md)                                           |
| There is no graph             | [`docs/architecture/there-is-no-graph.md`](architecture/there-is-no-graph.md)                       |
| Application contract hosting  | [`docs/architecture/application-contract-hosting.md`](architecture/application-contract-hosting.md) |
| WARP optic implementation map | [`docs/design/warp-optic-implementation-map.md`](design/warp-optic-implementation-map.md)           |
| v0.1.0 release plan           | [`docs/design/v0.1.0-release-plan.md`](design/v0.1.0-release-plan.md)                               |
| jedit release gate            | [`docs/design/v0.1.0-jedit-release-gate.md`](design/v0.1.0-jedit-release-gate.md)                   |
| jedit next slices             | [`docs/design/v0.1.0-jedit-next-ten-slices.md`](design/v0.1.0-jedit-next-ten-slices.md)             |
| Causal WAL                    | [`docs/design/causal-wal-end-to-end.md`](design/causal-wal-end-to-end.md)                           |
| Trusted runtime control       | [`docs/design/trusted-runtime-control-history.md`](design/trusted-runtime-control-history.md)       |
| Deterministic math            | [`docs/determinism/SPEC_DETERMINISTIC_MATH.md`](determinism/SPEC_DETERMINISTIC_MATH.md)             |
| Technical teardown            | [`docs/technical-teardown.md`](technical-teardown.md)                                               |

## Active Work

The live release question is:

```text
Can a real sibling application use Echo through generic contracts, retained
evidence, bounded readings, trusted-host lifecycle, and replay without Echo
learning application semantics?
```

For this cycle, jedit is the external proof pressure. Echo-side work should
prioritize the generic seams that jedit cannot fake:

- typed obstruction posture for contract-hosted operations and readings;
- retained evidence refs for receipts, readings, witnesses, and artifacts;
- durable witnessed submission persistence;
- product-facing intent outcome APIs that do not tick synchronously;
- WAL/WSC recovery that rejects half-accepted or missing-evidence states;
- deterministic replay witnesses that compare semantic evidence, not host
  timing prose.

## Deferred Full Rewrite

Do not do a broad rewrite of `README.md`, `docs/technical-teardown.md`, or the
docs index until the next few Echo/jedit release-gate seams land.

Do compact truth passes when signposts become misleading. Save the full rewrite
for after the repo has executable proof for:

- durable WAL/WSC recovery of accepted submissions and tick outcomes;
- retained artifact lookup for readings, receipts, witnesses, and contract
  artifacts;
- a jedit release-gate proof over app-owned generated contracts;
- product-facing obstruction and intent-outcome APIs;
- deterministic replay that survives process lifecycle boundaries.

## Non-Negotiables

- Application optics do not create ticks.
- Application dispatch does not execute synchronously.
- Application dispatch does not command ticks.
- Trusted host lifecycle control stays behind host-owned adapters.
- `AdmissionTicket` is distinct from `TickReceipt`.
- Mutation handlers run only during scheduler-owned execution.
- Query observers are read-only.
- Conflict rejection is final for that tick attempt.
- Retry is explicit new causal input.
- Echo remains generic and must not learn jedit, Vim, Jim, text, buffer, pane,
  cursor, editor, or other product semantics as core runtime concepts.
