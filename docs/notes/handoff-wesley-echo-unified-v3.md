<!-- SPDX-License-Identifier: Apache-2.0 OR MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Wesley + Echo Unified Architecture v3 - Handoff

**Date**: 2026-01-27
**Branch**: ttd-spec
**Status**: AWAITING APPROVAL
**Plan Document**: `docs/plans/wesley-echo-unified-v3.md`

---

## Summary of Session

This session conducted comprehensive research and planning to resolve confusion around Wesley's role, codec strategy, Rhai scripting, and footprint enforcement. The result is a unified architecture plan that supersedes all prior Wesley/Echo integration docs.

---

## Key Decisions Made (Pending Approval)

### 1. Wesley Becomes Rust

- Rewrite Wesley core as a Rust crate (`crates/wesley/`)
- WASM compilation for JS/browser backward compatibility
- No more Node.js subprocess invocation

### 2. Raw Little-Endian is Default Codec

- CBOR is overkill for most use cases
- flyingrobots.dev proves raw_le works perfectly
- `@wes_codec(format: "raw_le")` becomes default
- CBOR only for external system interop

### 3. Schemas Live in Echo

- New `janus/` subsystem directory
- Schema at `janus/wesley/schema/`
- Generated code at `janus/wesley/generated/{rust,typescript,rhai}/`
- Wesley becomes schema-agnostic tool

### 4. Wesley Generates Encoders/Decoders

- Not just type definitions
- Actual `to_bytes()` / `from_bytes()` methods
- No manual serialization code ever

### 5. Rhai for Game Rules (Not Rust)

- Developers write rules in Rhai scripts
- Sandboxed determinism (no time, IO, threads)
- All mutations through `warp.apply()`
- Hot-reload for fast iteration

### 6. Wesley Generates Rhai Bindings

- Type bindings: `Motion_new()`, `has_Motion()`, `get_Motion()`
- Rule scaffolds from `@wes_rule` directive
- Pattern helpers from `@wes_pattern` directive

### 7. Build-Time Footprint Enforcement

- Current: Runtime panics via `FootprintGuard`
- Proposed: Compile-time enforcement via scoped Rhai modules
- Wesley generates per-rule modules with ONLY declared types
- Violations become "undefined function" errors
- Static analysis detects conflicts between rules

---

## Documents Created/Modified

### Created

- `docs/plans/wesley-echo-unified-v3.md` — The comprehensive unified plan (700+ lines)

### Modified

- `docs/plans/wesley-architecture-v2.md` — Superseded by v3
- `README.md` — Fixed schema-first contradiction (removed manual encode/decode boilerplate)
- `packages/ttd-protocol-ts/package.json` — Updated Zod to v4.0.0

### Phase 2 Tasks Completed (Before Pivot to v3)

- ✅ Added missing types to Wesley schema (StepResult, Snapshot, etc.)
- ✅ Wired ttd-browser to use ttd-protocol-rs types
- ✅ Added wesley check to pre-commit hook
- ✅ Added wesley check to CI
- ✅ Created package.json for ttd-protocol-ts
- ✅ Wired ttd-app to use @echo/ttd-protocol-ts
- ✅ Moved TTD schema from Wesley repo to Echo (`schemas/ttd-protocol.graphql`)

---

## Research Conducted

### Wesley Codebase Analysis

- 16 packages in monorepo
- Core compilation in `wesley-core`
- Only CBOR implemented (json/protobuf declared but not implemented)
- TTD generator delegates Rust codegen to external `echo-ttd-gen`

### Echo Integration Analysis

- Current flow: Wesley (JS) → JSON manifests → echo-ttd-gen → Rust types
- Footprint system: Runtime enforcement via `FootprintGuard`
- Rhai planned for S1 milestone (issue #173)

### flyingrobots.dev Analysis

- Uses raw little-endian binary, NOT CBOR
- `ir.json` approach (legacy Wesley workflow)
- Proves raw_le works perfectly for game data

### Codec Research

- CBOR needed only for cross-language without shared codecs
- With Wesley-WASM, both Rust and JS use same encoder
- raw_le is faster, smaller, simpler

---

## Proposed Timeline (8 Weeks)

| Week | Phase           | Deliverable                              |
| ---- | --------------- | ---------------------------------------- |
| 1    | Reorganize      | `janus/` directory structure             |
| 2-3  | Wesley Rust     | Parser + IR + codegen                    |
| 4    | Codecs          | raw_le encoder/decoder generation        |
| 5    | WASM            | Wesley-WASM, deprecate JS                |
| 6    | Rhai Bindings   | Type bindings + rule scaffolds           |
| 7    | Scoped Bindings | Per-rule modules (footprint enforcement) |
| 8    | Static Analysis | Conflict detection, ordering constraints |

---

## Open Questions for Approval

1. **Directory naming**: Is `janus/` the right name for the TTD subsystem?
2. **Wesley location**: Should `crates/wesley/` live in Echo or remain separate repo?
3. **Rhai priority**: Should Rhai bindings come before or after Wesley Rust rewrite?
4. **Migration**: How to handle flyingrobots.dev migration from ir.json?

---

## Files to Review

1. **Main plan**: `docs/plans/wesley-echo-unified-v3.md`
2. **Current schema**: `schemas/ttd-protocol.graphql`
3. **Footprint system**: `crates/warp-core/src/footprint_guard.rs`
4. **Rhai docs**: `docs/rust-rhai-ts-division.md`

---

## Next Steps (After Approval)

1. Review and approve `docs/plans/wesley-echo-unified-v3.md`
2. Create GitHub issues for each phase
3. Begin Phase 1: Directory reorganization
4. Update ROADMAP.md with new timeline

---

## Redis Handoff Entry

```bash
XADD echo:agent:handoff * \
  branch "ttd-spec" \
  status "AWAITING_APPROVAL" \
  summary "Created Wesley+Echo unified architecture v3 plan with Rust core, raw_le codecs, Rhai rules, and build-time footprint enforcement" \
  current_task "Awaiting human approval of docs/plans/wesley-echo-unified-v3.md" \
  blockers "Human approval required before implementation" \
  next_steps "1. Review plan 2. Approve or request changes 3. Begin Phase 1" \
  key_docs "docs/plans/wesley-echo-unified-v3.md, docs/notes/handoff-wesley-echo-unified-v3.md" \
  timestamp "2026-01-27T12:00:00Z"
```
