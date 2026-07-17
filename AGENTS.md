<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# AGENTS

This guide is for AI agents and human operators recovering context in the Echo repository.

## Architecture North Star

Echo is not a graph database, app framework, sync daemon, or mutable state
server. Echo is a deterministic WARP runtime over witnessed causal history.

The durable territory is admitted causal history: transitions, frontiers, lane
identities, payload hashes, receipts, witnesses, checkpoints, suffixes, and
retained boundary artifacts. Graphs, files, editor buffers, UI state, and debug
views are materialized readings emitted by observers or optics over that
history. They may be cached, retained, transported, compared, or revealed, but
they are not the substrate ontology.

Public Echo surfaces should follow the WARP optic shape:

```text
explicit causal basis/site
+ bounded aperture
+ law
+ support, capability, budget, and evidence posture
-> witnessed hologram
```

External callers propose explicit-base intents or observe through bounded
optics. Echo admits, stages, pluralizes, conflicts, or obstructs those claims
under named law and emits receipts, reading envelopes, witnesses, or retained
shells. Transport is witnessed suffix admission, not state sync. Application
nouns belong in authored contracts and generated adapters, not in Echo core.

Keep these sentences in view when changing architecture, docs, or APIs:

```text
There is witnessed causal history.
WARP optics chart it.
Holograms witness those charts.
Materialized graphs are optional readings.
Continuum is the protocol for lawful causal-history exchange.
```

## Derived Trace Principle

> Witnessed causal history is truth. Trace is derived evidence. Receipt is
> proof only for the proposition it actually binds.

An execution trace must never become a second event-sourcing reality. If Echo
retains a trace, it derives that material from a sealed causal commit and binds
it to the real worldline, tick, frontier, receipt, schema, and payload identity.
WSC rows, compressed indexes, and prover-specific matrices remain physical
representations or projections, not causal authority. Disabled or obstructed
tracing yields an explicit posture, never a successful all-zero receipt.

## Git Rules

- **NEVER** amend commits.
- **NEVER** rebase or force-push.
- **NEVER** push to `main` without explicit permission.
- Always use standard commits and regular pushes.

## Repository Knowledge Model

Repository knowledge has one owner for each kind of truth:

- **Current architectural truth**: `docs/architecture/`, `docs/spec/`,
  `docs/invariants/`, and `docs/topics/`.
- **Durable architectural decisions**: accepted ADRs in `docs/adr/`.
- **Live work, priority, dependencies, and status**: GitHub Issues, Projects,
  pull requests, and review threads.
- **Shipped externally meaningful behavior**: `CHANGELOG.md`.
- **Historical source material**: Git history.

Use `README.md`, `GUIDE.md`, and `docs/README.md` as entrances. Do not recreate
cycles, retrospectives, a checked-in backlog, a checked-in status ledger, or a
post-hoc design document. Change-local design and test plans may live in the
issue or pull request. Write an ADR only when a decision changes a durable
architectural boundary or invariant.

When recovering context, read the relevant canonical topic/spec/invariant and
ADR, then inspect the current GitHub issue or pull request, `git log -n 5`, and
`git status`.

## Work Loop

```text
scope claim
-> decide whether an ADR is required
-> name the test plan and executable witness
-> RED
-> GREEN
-> update current docs and CHANGELOG when applicable
-> validate
-> stop
```

## Executable Claim Protocol

Engineering work must converge around executable evidence, not broad repository
interpretation. Before editing code, reduce the task to one executable claim:

1. **Bound the Claim**: State the behavior, invariant, or artifact that must
   change.
2. **Name the Witness**: Identify the smallest test, check, script, compile
   contract, golden vector, schema validation, or artifact inspection that can
   prove the claim.
3. **Run the Witness When Feasible**: Prefer a failing witness before the fix.
   If the witness cannot execute because the repository is already broken,
   repair only the minimal compile/runtime blocker required to run it.
4. **Do Not Expand from an Unblocker**: Do not turn a compile blocker into
   nearby architecture cleanup, docs cleanup, lint cleanup, migration sweep, or
   opportunistic refactor.
5. **Green the Claim**: Implement the smallest fix, rerun the witness, and run
   only directly relevant surrounding checks unless broader validation is
   explicitly requested.
6. **Stop on Green**: When the witness passes and the requested scope is
   satisfied, stop. Do not inspect more PR comments, audit more files, or clean
   unrelated residue without a new executable claim.

If unrelated failures remain, isolate them in the final report instead of
absorbing them into the task. Report files changed, symbols or behavior changed,
witness commands, pass/fail results, and intentional non-actions such as no
commit, no push, or no PR comment.

## Validation

After altering files, update canonical documentation when behavior or structure
changed, update `CHANGELOG.md` when shipped behavior changed, run the narrow
witness and directly relevant checks, and commit the result as a focused new
commit. Record follow-on work in GitHub rather than in the repository.

## Pre-PR Documentation Accuracy Gate

Immediately before opening a pull request, and again after material review
changes, compare the branch's actual behavior and artifacts with the current
documentation. Revisit the relevant entrances and canonical owners:
`README.md`, `GUIDE.md`, `docs/README.md`, `docs/architecture/`, `docs/spec/`,
`docs/invariants/`, `docs/topics/`, and the evidence anchors of any applicable
accepted ADRs.

Search specifically for stale current-state claims such as “not implemented,”
“fixture-only,” old ownership or authority boundaries, obsolete version or
compatibility statements, and examples that still prescribe a superseded path.
Correct inaccuracies in their owning documents before opening the PR. Do not
create a status ledger or duplicate change-local implementation detail in
multiple documents; keep durable boundaries in their canonical owners and link
to them. If the review finds no documentation impact, say so explicitly in the
pull-request body.

---

**The goal is inevitability. Every feature is defined by its tests.**
