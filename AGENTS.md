<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Agent Briefing

Welcome to the **Echo** project. This file captures expectations for any LLM agent (and future-human collaborator) who touches the repo.

## Core Principles

- **Honor the Vision**: Echo is a deterministic, multiverse-aware ECS. Consult `docs/architecture-outline.md` before touching runtime code.
- **Document Ruthlessly**: Every meaningful design choice should land in `docs/` (specs, diagrams, ADRs) or PR descriptions.
- **Docstrings Aren't Optional**: Public APIs across crates (`warp-core`, `warp-ffi`, `warp-wasm`, etc.) must carry rustdoc comments that explain intent, invariants, and usage. Treat missing docs as a failing test.
- **Determinism First**: Avoid introducing sources of nondeterminism without a mitigation plan.
- **Temporal Mindset**: Think in timelines—branching, merging, entropy budgets. Feature work should map to Chronos/Kairos/Aion axes where appropriate.

## The Drill Sergeant Discipline

Continuum (formerly JITOS) enforces a high-integrity "Drill Sergeant" discipline for all contributors (human or agent):

1. **Tests as Spec**: Every `feat:` or `fix:` commit MUST include a "red -> green" test story. If you change source code without changing or adding a test, CI will flag it as a policy violation.
2. **Zero-Warning Tolerance**: All determinism-critical crates (`warp-core`, `echo-wasm-abi`, `echo-scene-port`) are compiled with `RUSTFLAGS="-Dwarnings"`. Unused imports, dead code, or silenced lints are treated as build failures.
3. **Determinism Integrity**: We assert inevitability, not just correctness.
    - Bit-exact consistency across Rust and JavaScript/Node.js is mandatory for all float-to-int operations.
    - Never iterate over `std::collections::HashMap` or `HashSet` in paths that affect the state hash; use `BTreeMap` or sorted iterators.
    - Use the DIND (Deterministic Ironclad Nightmare Drills) harness to verify any changes against golden hash chains.
4. **Panic Ban**: Library code must return `Result` or `Option` instead of panicking. `unwrap()` and `expect()` are forbidden in non-test code.

## Timeline Logging

- Capture milestones, blockers, and decisions in relevant specs, ADRs, or PR descriptions.
- AGENTS.md and `TASKS-DAG.md` are append-only; see `docs/append-only-invariants.md` plus `scripts/check-append-only.js` for the enforcement plan that CI will run before merges.

## Agent Context System (2-Tier)

Agents use a **2-tier context system** to maintain continuity across sessions:

| Tier          | Store           | Purpose                                    | Update Frequency                 |
| ------------- | --------------- | ------------------------------------------ | -------------------------------- |
| **Immediate** | Redis stream    | Current task state, branch, blockers       | Every significant action         |
| **Deep**      | Knowledge graph | Architecture decisions, patterns, entities | When learning something reusable |

### Session Start (Bootstrap)

1. **Read this file** (`AGENTS.md`) for project conventions

2. **Check Redis handoff stream**: `echo:agent:handoff` (most recent entry)

    ```text
    XRANGE echo:agent:handoff - + COUNT 5
    ```

3. **Query knowledge graph** for relevant entities:

    ```python
    search_nodes("<feature_name>")  # e.g., "BOAW", "MaterializationBus"
    search_nodes("Echo")            # General project context
    ```

### During Work (Continuous Updates)

**Redis stream** — Update after every significant action:

- Completing a task or subtask
- Encountering a blocker
- Making a key decision
- Changing branches or PRs

```bash
XADD echo:agent:handoff * \
  branch "graph-boaw" \
  status "IN_PROGRESS" \
  summary "Fixing determinism bug in view op emission" \
  current_task "Updating emit_view_op_delta_scoped()" \
  blockers "none" \
  timestamp "<ISO-8601 timestamp>"
```

**Knowledge graph** — Create/update entities when you:

- Discover an architectural pattern worth preserving
- Complete a milestone (create `<Feature>_Phase<N>` entity)
- Fix a non-obvious bug (create `<Feature>_BugFix` entity)
- Make a decision that future agents should know about

```json
{
    "name": "BOAW_Determinism_Fix",
    "entityType": "BugFix",
    "observations": [
        "Root cause: emit_view_op_delta() used delta.len() for view op IDs",
        "delta.len() is worker-local and varies by shard claim order",
        "Fix: derive op ID from intent scope (NodeId) which is content-addressed"
    ]
}
```

### Session End (Handoff)

Before ending a session, **always** write a handoff entry:

```bash
XADD echo:agent:handoff * \
  branch "<current-branch>" \
  status "<COMPLETE|IN_PROGRESS|BLOCKED>" \
  summary "<1-2 sentence summary of what was done>" \
  commits "<recent commit hashes and messages>" \
  next_steps "<what the next agent should do>" \
  blockers "<any blockers, or 'none'>" \
  tech_debt "<any shortcuts taken that need cleanup>" \
  test_commands "<commands to verify the work>" \
  timestamp "<ISO-8601 timestamp>"
```

### Key Entities to Know

The knowledge graph contains ~300+ entities built by prior agents. Key patterns:

- `Echo Project` — Core project info and current focus
- `<Feature>_Architecture` — Design decisions for major features
- `<Feature>_Phase<N>` — Milestone completion records
- `<Feature>_BugFix` — Non-obvious bug fixes worth remembering
- `<Feature>_Tech_Debt_P<N>` — Tracked technical debt by priority

### Why This Matters

- **Quick tasks**: Redis handoff alone may suffice
- **Complex tasks**: Query knowledge graph for architectural context
- **Debugging**: Search for prior bug fixes in similar areas
- **Decisions**: Check if prior agents already explored an approach

The 2-tier system means handoffs are seamless—no context is lost between agents, and institutional knowledge accumulates over time.

## Workflows & Automation

- The contributor playbook lives in `docs/workflows.md` (policy + blessed commands + automation).
- Preferred repo maintenance entrypoint is `cargo xtask …` (see `xtask/` and `.cargo/config.toml`).
- Planning DAG artifacts live in `docs/assets/dags/` and are documented in `docs/dependency-dags.md`.
- For automated DAG refresh PRs, set `DAG_REFRESH_ISSUE=<issue-number>` as a GitHub Actions variable so the bot PR body includes `Refs #…`.

## Repository Layout

- `crates/warp-core`: Runtime core (WARP graph model, materialization bus).
- `apps/playground`: Vite sandbox and inspector (future).
- `docs/`: Specs, diagrams, memorials.
- `docs/notes`: Working notes and explorations (non-authoritative).

## Working Agreement

- **Isolated Branches**: Every new task, feature, or bugfix **MUST** begin on a fresh, isolated branch based on the latest `main` (unless context explicitly dictates otherwise). Never mix unrelated objectives on the same branch.
- Keep `main` pristine. Feature work belongs on branches named `echo/<feature>` or `timeline/<experiment>`.
- Tests and benchmarks are mandatory for runtime changes once the harness exists.
- Respect determinism: preferably no random seeds without going through the Echo PRNG.
- Run `cargo clippy --all-targets -- -D missing_docs` and `cargo test` before every PR; CI will expect a zero-warning, fully documented surface.

### PRs & Issues (Linkage Policy)

- Every PR must be tied to a GitHub Issue.
    - If no suitable issue exists, open one before you open the PR.
    - Use explicit closing keywords in the PR body: include a line like `Closes #<issue-number>` so the issue auto‑closes on merge.
    - Keep PRs single‑purpose: 1 PR = 1 thing. Avoid bundling unrelated changes.
- Branch naming: prefer `echo/<short-feature-name>` or `timeline/<experiment>` and include the issue number in the PR title.
- Project hygiene: assign the PR's linked issue to the correct Milestone and Board column (Blocked/Ready/Done) as part of the PR.

### Git Hooks & Local CI

- Install repo hooks once with `make hooks` (configures `core.hooksPath`).
- Formatting: pre-commit auto-fixes with `cargo fmt` by default. Set `ECHO_AUTO_FMT=0` to run check-only instead.
- Toolchain: pre-commit verifies your active toolchain matches `rust-toolchain.toml`.
- SPDX header policy (source): every source file must start with exactly:
    - `// SPDX-License-Identifier: Apache-2.0`
    - `// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>`
      Use the repository scripts/hooks; do not add dual-license headers to code.

## Git Real

1. **NEVER** use `--force` with any git command. If you think you need it, stop and ask the human for help.
2. **NEVER** use rebase. Embrace messy distributed history; plain merges capture the truth, rebases rewrite it.
3. **NEVER** amend a commit. Make a new commit instead of erasing recorded history.

In short: no one cares about a tidy commit graph, but everyone cares if you rewrite commits on origin.

Safe travels in the multiverse.
