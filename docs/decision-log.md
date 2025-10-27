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
| 2025-10-27 | Time‑aware collision design | Adopt deterministic, Chronos/Kairos/Aion‑aware collision + CCD spec | Quantized TOI, fat AABBs, stable sorts; events via Temporal Bridge | Author `spec-geom-collision.md` and `phase1-geom-plan.md`; scaffold `rmg-geom` next |
| 2025-10-27 | Echo vs Unity doc | Clarify RMG rewrites vs GameObject frame loops | Align messaging for contributors and readers | Add `echo-vs-unity.md` to docs index |
| 2025-10-27 | rmg-geom crate + docs wiring | Added `rmg-geom` with `types/{transform,aabb}`, `temporal/{tick,temporal_transform,temporal_proxy}`, and `broad/aabb_tree.rs` (`BroadPhase` trait + simple tree). | Enable deterministic broad-phase pairing and fat AABB computation for the collision tour; add unit tests for fat AABB and pair ordering. | Updated workspace; added VitePress skeleton with a page linking the Collision DPO Tour; added Playwright smoke tests for tour (load, tabs, prev/next). |
