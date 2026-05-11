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

## Git Rules

- **NEVER** amend commits.
- **NEVER** rebase or force-push.
- **NEVER** push to `main` without explicit permission.
- Always use standard commits and regular pushes.

## Documentation & Planning Map

Do not audit the repository by recursively walking the filesystem. Follow the authoritative manifests:

### 1. The Entrance

- **`README.md`**: Public front door, core value prop, and quick tour.
- **`GUIDE.md`**: Orientation and productive-fast path.
- **`docs/index.md`**: VitePress documentation map.

### 2. The Bedrock

- **`ARCHITECTURE.md`**: Authoritative structural reference (Hexagonal, Core, Memory).
- **`VISION.md`**: Core tenets and the causal mission.
- **`METHOD.md`**: Repo work doctrine (Backlog lanes, Cycle loop).

### 3. The Direction

- **`docs/BEARING.md`**: Current execution gravity and active tensions.
- **`docs/design/ROADMAP.md`**: Broad strategic horizon and targets.
- **`backlog/`**: The active source of truth for pending work.

### 4. The Proof

- **`CHANGELOG.md`**: Historical truth of merged behavior.
- **`cargo xtask dind`**: Determinism convergence verification.

## Context Recovery Protocol

When starting a new session or recovering from context loss:

1. **Read `docs/BEARING.md`** to find the current execution gravity.
2. **Read `METHOD.md`** to understand the work doctrine.
3. **Check `backlog/asap/`** for imminent work.
4. **Check `git log -n 5` and `git status`** to verify the current branch state.

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

## End of Turn Checklist

After altering files:

1. **Verify Truth**: Ensure documentation is updated if behavior or structure changed.
2. **Log Debt**: Add follow-on backlog items to `bad-code/` or `cool-ideas/`.
3. **Commit**: Use focused, conventional commit messages. Propose a draft before executing.
4. **Validate**: Run `cargo check` and relevant tests.

---

**The goal is inevitability. Every feature is defined by its tests.**
