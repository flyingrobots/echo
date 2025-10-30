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
| 2025-10-28 | PR #7 merged | Reachability-only snapshot hashing; ports demo registers rule; guarded ports footprint; scheduler `finalize_tx()` clears `pending`; `PortKey` u30 mask; hooks+CI hardened (toolchain pin, rustdoc fixes). | Determinism + memory hygiene; remove test footguns; pass CI with stable toolchain while keeping rmg-core MSRV=1.68. | Queued follow-ups: #13 (Mat4 canonical zero + MulAssign), #14 (geom train), #15 (devcontainer). |
| 2025-10-27 | MWMR reserve gate | Engine calls `scheduler.finalize_tx()` at commit; compact rule id used on execute path; per‑tx telemetry summary behind feature. | Enforce independence and clear active frontier deterministically; keep ordering stable with `(scope_hash, family_id)`. | Toolchain pinned to Rust 1.68; add design note for telemetry graph snapshot replay. |
## 2025-10-28 — Mat4 canonical zero + MulAssign (PR #13)

- Decision: Normalize -0.0 from trig constructors in Mat4 and add MulAssign for in-place multiplication.
- Rationale: Avoid bitwise drift in snapshot/matrix comparisons across platforms; improve ergonomics in hot loops.
- Impact: No API breaks. New tests assert no -0.0 in rotation matrices at key angles; added `MulAssign` for owned/&rhs.
- Next: Review feedback; if accepted, apply same canonicalization policy to other math where applicable.
 
## 2025-10-28 — Geometry merge train (PR #14)

- Decision: Use an integration branch to validate #8 (geom foundation) + #9 (broad-phase AABB) together.
- Rationale: Surface cross-PR interactions early and avoid rebase/force push; adhere to merge-only policy.
- Impact: New crate `rmg-geom` (AABB, Transform, TemporalTransform) and baseline broad-phase with tests. No public API breaks in core.
- Next: If green, merge train PR; close individual PRs as merged-via-train.

## 2025-10-28 — rmg-geom foundation (PR #8) compile + clippy fixes

- Decision: Keep PR #8 scoped to geometry foundations; defer `broad` module to its own PR to avoid E0583.
- Changes: Use `Quat::to_mat4` + `Mat4::new` in `Transform::to_mat4`; replace `Vec3::ZERO` with `Vec3::new(0,0,0)` for MSRV; rename variables to satisfy `similar_names`.
- CI: Merged latest `main` to pick up stable-toolchain overrides for workspace clippy/test; crate-level clippy gates relaxed (drop `nursery`/`cargo`) to avoid workspace metadata lints.
- Next: Land PR #9 for broad-phase on top; revisit clippy gates once workspace has uniform metadata.
## 2025-10-28 — Devcontainer added

- Decision: Provide a reproducible local environment matching CI runners.
- Details: VS Code devcontainer (Ubuntu 24.04) with Rust stable + MSRV toolchains, clippy/rustfmt, Node 20, gh CLI; post-create script installs 1.68.0 and wasm target.
- Outcome: Faster feedback loops; easier reproduction of CI issues (clippy, rustdoc, Docs Guard).

## 2025-10-28 — Pre-commit formatting flag renamed

- Decision: Use an Echo-scoped env var for auto-format on commit.
- Change: `AUTO_FMT` → `ECHO_AUTO_FMT` in `.githooks/pre-commit`.
- Docs: README, AGENTS, CONTRIBUTING updated with hook install and usage.

## 2025-10-29 — Snapshot header v1 + tx/rule hardening (rmg-core)

- Context: PR #9 base work on top of PR #8; integrate deterministic provenance into snapshots without changing reachable‑only state hashing.
- Decision: Model snapshots as commit headers with explicit `parents` and metadata digests (`plan`, `decision`, `rewrites`). Keep `decision_digest = blake3(len=0_u64)` (canonical empty list digest) until Aion/agency lands.
- Changes:
  - `Snapshot { parents: Vec<Hash>, plan_digest, decision_digest, rewrites_digest, policy_id }`.
  - `Engine::commit()` computes `state_root`, canonical empty/non‑empty digests, and final commit hash.
  - `Engine::snapshot()` produces a header‑shaped view with canonical empty digests so a no‑op commit equals a pre‑tx snapshot.
  - Enforce tx lifecycle (`live_txs` set; deny ops on closed/zero tx); `begin()` is `#[must_use]` and wraps on `u64::MAX` skipping zero.
  - Rule registration now rejects duplicate names and duplicate ids; assigns compact rule ids for execution hot path.
  - Scheduler is crate‑private; ordering invariant documented (ascending `(scope_hash, rule_id)`).
- Tests: Added/updated motion tests (velocity preserved; commit after `NoMatch` is a no‑op), math tests (relative tolerances; negative scalar multiplies; extra mul order).
- Consequence: Deterministic provenance is now explicit; future Aion inputs can populate `decision_digest` without reworking the header. No behavior changes for state hashing.

## 2025-10-29 — Toolchain strategy: floor raised to 1.71.1

- Decision: Raise the workspace floor (MSRV) to Rust 1.71.1. All crates and CI jobs target 1.71.1.
- Implementation: Updated `rust-toolchain.toml` to 1.71.1; bumped `rust-version` in crate manifests; CI jobs pin 1.71.1; devcontainer installs only 1.71.1.

## 2025-10-29 — Docs E2E carousel init (PR #10)

- Context: Playwright tour test clicks Next to enter carousel from "all" mode.
- Decision: Do not disable Prev/Next in "all" mode; allow navigation buttons to toggle into carousel.
- Change: docs/assets/collision/animate.js leaves Prev/Next enabled in 'all'; boundary disabling still applies in single-slide mode.
- Consequence: Users can initiate the carousel via navigation controls; E2E tour test passes deterministically.

## 2025-10-29 — Docs make open (PR #11)

- Context: Make dev docs open automatically; fix routing and dead-link noise.
- Decisions:
  - Use a precise dead-link ignore for `/collision-dpo-tour.html` (exact regex) until the page is always present.
  - Convert tour/spec links to root‑relative paths to work site‑wide under VitePress routing.
  - Make the dev server polling loop portable (`sleep 1`).
- Consequence: Docs dev flow is consistent across environments; CI Docs Guard happy; links resolve from any page.

## 2025-10-29 — Hooks formatting gate (PR #12)

- Context: Enforce consistent formatting before commit; avoid CI/docs drift when non-doc files change.
- Decision: Pre-commit runs `cargo fmt --all -- --check` whenever staged Rust files are detected. Retain the PRNG coupling guard but remove the unconditional early exit so formatting still runs when the PRNG file isn’t staged.
- EditorConfig: normalize line endings (LF), ensure final newline, trim trailing whitespace, set 2-space indent for JS/TS/JSON and 4-space for Rust.
- Consequence: Developers get immediate feedback on formatting; cleaner diffs and fewer CI round-trips.

## 2025-10-29 — Geom fat AABB bounds mid-rotation

- Context: Broad-phase must not miss overlaps when a shape rotates about an off‑centre pivot; union of endpoint AABBs can under‑approximate mid‑tick extents.
- Decision: `Timespan::fat_aabb` now unions AABBs at start, mid (t=0.5 via nlerp for rotation, lerp for translation/scale), and end. Sampling count is fixed (3) for determinism.
- Change: Implement midpoint sampling in `crates/rmg-geom/src/temporal/timespan.rs`; add test `fat_aabb_covers_mid_rotation_with_offset` to ensure mid‑pose is enclosed.
- Consequence: Deterministic and more conservative broad‑phase bounds for typical rotation cases without introducing policy/config surface yet; future work may expose a configurable sampling policy.

## 2025-10-29 — Pre-commit auto-format policy

- Decision: When `ECHO_AUTO_FMT=1` (default), the pre-commit hook first checks formatting. If changes are needed, it runs `cargo fmt` to update files, then aborts the commit. This preserves index integrity for partially staged files and prevents unintended staging of unrelated hunks.
- Rationale: `rustfmt` formats entire files; auto-restaging could silently defeat partial staging. Aborting makes the workflow explicit: review, restage, retry.
- Consequence: One extra commit attempt in cases where formatting is needed, but safer staging semantics and fewer surprises. Message includes guidance (`git add -p` or `git add -A`).

## 2025-10-29 — CI + Security hardening

- Decision: Add `cargo audit` and `cargo-deny` to CI; expand rustdoc warnings gate to all public crates.
- Rationale: Catch vulnerable/deprecated crates and doc regressions early; keep public surface clean.
- Consequence: Faster failures on dependency or doc issues; small CI time increase.
- Notes:
  - Use `rustsec/audit-check@v1` for the audit step; avoid pinning to non-existent tags.
  - Add `deny.toml` with an explicit license allowlist to prevent false positives on permissive licenses (Apache-2.0, MIT, BSD-2/3, CC0-1.0, MIT-0, Unlicense, Unicode-3.0, BSL-1.0, Apache-2.0 WITH LLVM-exception).
  - Run the audit job on Rust 1.75.0 to meet `cargo-audit` MSRV; this does not change the workspace MSRV (1.71.1).

## 2025-10-29 — Snapshot commit spec (v1)

- Decision: Introduce `docs/spec-merkle-commit.md` describing `state_root` vs `commit_id` encodings and invariants.
- Rationale: Make provenance explicit and discoverable; align code comments with a durable spec.
- Changes: Linked spec from `crates/rmg-core/src/snapshot.rs` and README.
