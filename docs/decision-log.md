# Decision Log

*Demo outcomes should prefix the Decision column with `Demo <number> — …` to keep entries searchable.*

| Date | Context | Decision | Rationale | Consequence |
| ---- | ------- | -------- | --------- | ----------- |
| 2025-10-23 | Repo reset | Adopt pnpm + TS skeleton | Monorepo scaffolding for Echo | Phase 0 tasks established |
| 2025-10-24 | Branch tree spec | Integrate roaring bitmaps and chunk epochs | Deterministic merges & diffs | Snapshot policy updated |
| 2025-10-24 | Codex’s Baby spec | Event envelopes, temporal bridge integration | Align with causality layer | Security envelopes + inspector updates |
| 2025-10-25 | Serialization protocol | Canonical encoding using BLAKE3 | Cross-platform determinism | Replay tooling groundwork |
| 2025-10-25 | Temporal bridge doc | Formalized retro delivery & paradox guard | Ensure cross-branch consistency | Entropy hooks refined |
| 2025-10-25 | Replay plan | Golden hashes + CLI contract | Ensure reproducibility | Phase 1 test suite scope |
| 2025-10-25 | Math validation harness | Landed Rust fixture suite & tolerance checks for deterministic math | Keep scalar/vector/matrix/quaternion results stable across environments | Extend coverage to browser + fixed-point modes |
| 2025-10-26 | EPI bundle | Adopt entropy, plugin, inspector, runtime config specs (Phase 0.75) | Close causality & extensibility gap | Phase 1 implementation backlog defined |
| 2025-10-26 | RMG + Confluence | Adopt RMG v2 (typed DPOi engine) and Confluence synchronization as core architecture | Unify runtime/persistence/tooling on deterministic rewrites | Launch Rust workspace (rmg-core/ffi/wasm/cli), port ECS rules, set up Confluence networking |
| 2025-10-27 | Core math split | Split `rmg-core` math into focused submodules (`vec3`, `mat4`, `quat`, `prng`) replacing monolithic `math.rs`. | Improves readability, testability, and aligns with strict linting. | Update imports; no behavior changes intended; follow-up determinism docs in snapshot hashing. |
| 2025-10-27 | PR #7 prep | Extracted math + engine spike into `rmg-core` (split-core-math-engine); added inline rustdoc on canonical snapshot hashing (node/edge order, payload encoding). | Land the isolated, reviewable portion now; keep larger geometry/broad‑phase work split for follow-ups. | After docs update, run fmt/clippy/tests; merge is a fast‑forward over `origin/main`. |
| 2025-10-28 | PR #7 finalize | Reachability-only snapshot hashing; ports demo registers rule; guarded ports footprint; scheduler `finalize_tx()` clears `pending`; `PortKey` u30 mask; hooks+CI hardened (toolchain pin, rustdoc fixes). | Determinism + memory hygiene; remove test footguns; pass CI with stable toolchain while keeping rmg-core MSRV=1.68. | Merge PR #7 after green CI; queue MWMR Phase 2 perf tests + retry policy work. |
| 2025-10-27 | MWMR reserve gate | Engine calls `scheduler.finalize_tx()` at commit; compact rule id used on execute path; per‑tx telemetry summary behind feature. | Enforce independence and clear active frontier deterministically; keep ordering stable with `(scope_hash, family_id)`. | Toolchain pinned to Rust 1.68; add design note for telemetry graph snapshot replay. |
## 2025-10-28 — Mat4 canonical zero + MulAssign (PR #13)

- Decision: Normalize -0.0 from trig constructors in Mat4 and add MulAssign for in-place multiplication.
- Rationale: Avoid bitwise drift in snapshot/matrix comparisons across platforms; improve ergonomics in hot loops.
- Impact: No API breaks. New tests assert no -0.0 in rotation matrices at key angles; added `MulAssign` for owned/&rhs.
- Next: Review feedback; if accepted, apply same canonicalization policy to other math where applicable.
