<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# AGENTS

This guide is for AI agents and human operators recovering context in the Echo repository.

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

## End of Turn Checklist

After altering files:

1. **Verify Truth**: Ensure documentation is updated if behavior or structure changed.
2. **Log Debt**: Add follow-on backlog items to `bad-code/` or `cool-ideas/`.
3. **Commit**: Use focused, conventional commit messages. Propose a draft before executing.
4. **Validate**: Run `cargo check` and relevant tests.

---

**The goal is inevitably. Every feature is defined by its tests.**
