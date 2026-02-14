<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Rollback Playbook — TTD Integration

## Scope

Rollback coverage for commit range:

- Base: `efae3e8`
- Head: `e201c9b`

## Preconditions

- Release owner approval logged.
- Current branch state saved/tagged.
- Incident ticket created.

## Scenario A — Full TTD Rollback

### Objective (Scenario A)

Return repository to pre-TTD integration state.

### Ordered actions

1. Create rollback branch:
    - `rollback/ttd-full-<date>`
2. Revert commits in reverse order from head to base+1:
    - `e201c9b`
    - `fd98b91`
    - `ce98d80`
    - `a02ea86`
    - `3187e6a`
    - `6e34a77`
    - `f138b8a`
3. Resolve conflicts preserving pre-TTD behavior.

### Validation Checklist (Scenario A)

- [ ] `cargo check --workspace` passes
- [ ] Determinism suite for non-TTD core passes
- [ ] Build pipelines pass
- [ ] Smoke test core runtime flows pass

---

## Scenario B — Partial Rollback (FFI/UI layer)

### Objective (Scenario B)

Remove unstable FFI/UI integration while preserving core hardening.

### Candidate revert target(s)

- `fd98b91` (UI/WASM Integration)
- `ce98d80` (Frontend Restoration)
- optionally `a02ea86` if FFI safety layer must be reverted together

### Dependency constraints

- Reverting `a02ea86` may break consumers expecting SessionToken/FFI contracts.
- Validate dependent crates/apps after each revert step.

### Validation Checklist (Scenario B)

- [ ] `apps/ttd-app` build status known (pass/fail expected documented)
- [ ] Core codec/scene crates compile and tests pass
- [ ] CI gate summary attached to incident

---

## Post-Rollback Evidence Packet (required)

- commit SHAs reverted
- CI run IDs
- failing/passing gate delta (before vs after)
- residual risk summary
- recommendation: GO / CONDITIONAL / NO-GO
