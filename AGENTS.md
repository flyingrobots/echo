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

## Wesley Schema-First Development

Echo uses **Wesley** as a schema-first protocol compiler. The schema is the single source of truth for all protocol types.

### Golden Rule

> **Never modify generated code. Modify the schema, then regenerate.**

### Key Files

| Path                            | Purpose                                                    |
| ------------------------------- | ---------------------------------------------------------- |
| `schemas/ttd-protocol.graphql`  | Source of truth for TTD protocol types (**lives in Echo**) |
| `crates/ttd-protocol-rs/lib.rs` | Generated Rust types (**DO NOT EDIT**)                     |
| `packages/ttd-protocol-ts/`     | Generated TypeScript types (**DO NOT EDIT**)               |
| `crates/ttd-manifest/`          | Generated manifests (**DO NOT EDIT**)                      |
| `docs/wesley/wesley.lock`       | Provenance tracking (commit SHA + schema_hash)             |

### Workflow

1. **Edit the schema** in Echo: `schemas/ttd-protocol.graphql`
2. **Regenerate** in Echo: `cargo xtask wesley sync`
3. **Verify** the schema_hash matches across all consumers
4. **Commit** both the schema changes and generated outputs together

### Versioning

Every generated artifact embeds `SCHEMA_SHA256` (Rust) / `SCHEMA_HASH` (TypeScript). This hash **must match** across:

- `ttd-protocol-rs` (Rust types)
- `ttd-protocol-ts` (TypeScript types)
- `ttd-browser` WASM module (via dependency on `ttd-protocol-rs`)
- `ttd-manifest` JSON files
- `docs/wesley/wesley.lock` provenance file

If hashes diverge, the system is in an inconsistent state. Run `cargo xtask wesley sync` to realign.

### CI Enforcement

CI runs `cargo xtask wesley check` to verify generated outputs match the Wesley commit in `wesley.lock`. Pre-commit hooks also run this check. If it fails:

1. Someone edited generated code directly (bad - revert and edit the schema instead), or
2. The schema changed but outputs weren't regenerated (run `cargo xtask wesley sync`)

### Why This Matters

- **Determinism**: Same schema → same types → same wire format
- **Consistency**: Rust, TypeScript, and manifests always agree
- **Auditability**: `wesley.lock` records exactly which Wesley commit produced current outputs
- **Safety**: `schema_hash` in TTDR headers enables version compatibility checks at runtime

---

## Git Real

1. **NEVER** use `--force` with any git command. If you think you need it, stop and ask the human for help.
2. **NEVER** use rebase. Embrace messy distributed history; plain merges capture the truth, rebases rewrite it.
3. **NEVER** amend a commit. Make a new commit instead of erasing recorded history.

In short: no one cares about a tidy commit graph, but everyone cares if you rewrite commits on origin.

Safe travels in the multiverse.

---

## Policy Guardrails (Lint/CI Integrity)

- Do NOT modify or relax lint/format policies without explicit human instruction.
    - This includes `.markdownlint*`, `.prettierrc*`, `.editorconfig`, `.gitignore` / `.gitinfores`,
      CI configs, or hook scripts.
- Do NOT add ignore lists or disable rules to “make it pass.”
- If pre-commit or CI fails due to unrelated files, STOP and ask how to proceed.
    - Do not auto-fix or restage unrelated files.
- For generated outputs: only regenerate via the blessed commands (e.g. `cargo xtask wesley sync`),
  never edit outputs by hand, and never alter policy to accept missing docs, etc.
- If the user asks “commit all,” still respect the above; ask for clarification if
  “all” includes policy changes or unrelated files.
