# Decision Log

*Demo outcomes should prefix the Decision column with `Demo <number> — …` to keep entries searchable.*

| Date | Context | Decision | Rationale | Consequence |
| ---- | ------- | -------- | --------- | ----------- |
| 2025-11-03 | Scalar foundation | Add `rmg-core::math::Scalar` trait (operator supertraits + sin/cos) | Arithmetic via `Add/Sub/Mul/Div/Neg` supertraits for ergonomic `+ - * /`; `sin/cos` methods declared; canonicalization/LUTs deferred | Unblocks F32Scalar and DFix64 implementations; math code can target a stable trait |
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

## Recent Decisions (2025-10-28 onward)

The following entries use a heading + bullets format for richer context.
| 2025-11-06 | rmg-core scheduler Clippy cleanup | Make pre-commit pass without `--no-verify`: fix `doc_markdown`, `similar_names`, `if_not_else`, `option_if_let_else`, `explicit_iter_loop`; change `RewriteThin.handle` to `usize`; keep radix `counts16` as `Vec<u32>` (low bandwidth) with safe prefix-sum/scatter; fail fast in drain with `unreachable!` instead of `expect()` or silent drop; make `pending` field private (keep `PendingTx` private). | Preserve determinism and ordering while satisfying strict `clippy::pedantic` and `-D warnings`. Avoid truncation casts and private interface exposure. | Determinism preserved; panic on invariant violation; histogram remains 256 KiB on 64‑bit; pre-commit unblocked.
| 2025-11-06 | rmg-core test + benches lint fixes | Clean up `clippy::pedantic` failures blocking commit: (1) add backticks to doc comments for `b_in`/`b_out` and `GenSet(s)`; (2) refactor `DeterministicScheduler::reserve` into helpers to satisfy `too_many_lines`; (3) move inner test function `pack_port` above statements to satisfy `items_after_statements`; (4) remove `println!` and avoid `unwrap()`/`panic!` in tests; (5) use captured format args and `u64::from(...)`/`u32::from(...)` idioms; (6) fix `rmg-benches/benches/reserve_scaling.rs` imports (drop unused `CompactRuleId` et al.) and silence placeholder warnings. | Align tests/benches with workspace lint policy while preserving behavior; ensure CI and pre-commit hooks pass uniformly. | Clippy clean on lib + tests; benches compile; commit hook no longer blocks.
| 2025-10-30 | rmg-core determinism hardening | Added reachability-only snapshot hashing; closed tx lifecycle; duplicate rule detection; deterministic scheduler drain order; expanded motion payload docs; tests for duplicate rule name/id and no‑op commit. | Locks determinism contract and surfaces API invariants; prepares PR #7 for a safe merge train. | Clippy clean for rmg-core; workspace push withheld pending further feedback. |
| 2025-10-30 | Tests | Add golden motion fixtures (JSON) + minimal harness validating motion rule bytes/values | Establishes deterministic test baseline for motion; supports future benches and tooling | No runtime impact; PR-01 linked to umbrella and milestone |
| 2025-10-30 | Templates PR scope | Clean `echo/pr-templates-and-project` to contain only templates + docs notes; remove unrelated files pulled in by merge; fix YAML lint (trailing blanks; quote placeholder) | Keep PRs reviewable and single-purpose; satisfy CI Docs Guard | Easier review; no runtime impact |
| 2025-10-30 | Docs lint | Fix MD022 (blank line after headings) in `docs/spec-deterministic-math.md` on branch `echo/docs-math-harness-notes` | Keep markdown lint clean; improve readability | No content change; unblock future docs PRs |
| 2025-10-30 | Bug template triage | Add optional `stack_trace` and `version` fields to `.github/ISSUE_TEMPLATE/bug.yml` | Capture logs and version/SHA up front to speed debugging | Better triage signal without burdening reporters |
| 2025-10-30 | Bug template wording | Standardize bug template descriptions to imperative capitalization ("Provide …") | Consistent style and clearer prompts | Improved reporter guidance |
| 2025-10-30 | Proptest seed pinning | Add dev‑dep `proptest` and a pinned‑seed property test for motion rule (`proptest_seed_pinning.rs`) | Establish deterministic, reproducible property tests and document seed‑pinning pattern | Tests‑only; no runtime impact |
| 2025-10-30 | CI matrix | Add musl tests job (rmg-core; x86_64-unknown-linux-musl) and a manual macOS workflow for local runs | Cover glibc + musl in CI while keeping macOS optional to control costs | Determinism coverage improves; CI footprint remains lean |
| 2025-10-30 | Docs rollup | Add generator script and `docs/echo-total.md` rollup of all top‑level docs | Single-file reference for reviewers and readers; preserves source of truth in individual docs | Keep rollup refreshed via script when docs change |
| 2025-10-30 | Docs rollup review (PR-05) | Add MUSL job intent comment (rmg-core only) and fix generator script to use portable newlines (`printf`/`echo`) | Clarify CI intent and ensure rollup emits correct formatting | Merged `main` into branch (no rebase/force) |
| 2025-10-30 | Motion negative tests (PR-06) | Add tests documenting NaN/Infinity propagation and invalid payload size NoMatch in motion rule | Clarify expected behavior without changing runtime; improves determinism docs via tests | Tests-only; no runtime impact |
| 2025-10-30 | Docs rollup check (PR-07) | Add CI workflow to verify `docs/echo-total.md` is regenerated | Prevents drift between source docs and the rollup | Fails PR with guidance if rollup is stale |
| 2025-10-30 | Docs tooling (PR-08) | Add `make echo-total` target and README docs section for rollup | Makes rollup refresh one command; improves contributor experience | Small tooling quality-of-life |
| 2025-10-30 | BLAKE3 header tests (PR-09) | Add unit tests to verify commit header encoding order/endianness and hash equivalence | Codifies checklist guarantees in tests; prevents regressions | Tests-only; no runtime impact |
| 2025-10-30 | README CI tips (PR-10) | Document manual macOS workflow and how to reproduce CI locally | Lowers barriers to contributor validation | Docs-only |
| 2025-11-01 | Rollup automation (PR-10) | Pre-commit regenerates `docs/echo-total.md` when top-level docs change; aborts to preserve index until staged | Keeps rollup in sync pre-CI and reduces churn | Developer experience improvement |
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
  - Run cargo-audit on Rust 1.75.0 (via `RUSTUP_TOOLCHAIN=1.75.0`) to meet its MSRV; this does not change the workspace MSRV (1.71.1).

## 2025-10-29 — Snapshot commit spec (v1)

- Decision: Introduce `docs/spec-merkle-commit.md` describing `state_root` vs `commit_id` encodings and invariants.
- Rationale: Make provenance explicit and discoverable; align code comments with a durable spec.
- Changes: Linked spec from `crates/rmg-core/src/snapshot.rs` and README.
 
| 2025-10-30 | CI toolchain simplification | Standardize on Rust `@stable` across CI (fmt, clippy, tests, security audit); remove MSRV job; set `rust-toolchain.toml` to `stable`. | Reduce toolchain drift and recurring audit/MSRV mismatches. | Future MSRV tracking can move to release notes when needed. |
| 2025-10-30 | Rustdoc pedantic cleanup | Snapshot docs clarify `state_root` with code formatting to satisfy `clippy::doc_markdown`. | Keep strict lint gates green; no behavior change. | None. |
| 2025-10-30 | Spec + lint hygiene | Removed duplicate `clippy::module_name_repetitions` allow in `rmg-core/src/lib.rs`. Clarified `docs/spec-merkle-commit.md`: `edge_count` is u64 LE and may be 0; genesis commits have length=0 parents; “empty digest” explicitly defined as `blake3(b"")`; v1 mandates empty `decision_digest` until Aion lands. | Codifies intent; prevents ambiguity for implementers. | No code behavior changes; spec is clearer. |
| 2025-10-30 | Templates & Project | Added issue/PR/RFC templates and configured Echo Project (Status: Blocked/Ready/Done); fixed YAML lint nits | Streamlines review process and Kanban tracking | No runtime impact; CI docs guard satisfied |

## 2025-11-02 — M1: benches crate skeleton (PR-11)

- Decision: Add `crates/rmg-benches` with a minimal Criterion harness and a motion-throughput benchmark using public `rmg-core` APIs.
- Rationale: Establish a place for performance microbenches; keep PR small and focused before adding JSON artifacts/regression gates in follow-ups.
- Consequence: Benches run locally via `cargo bench -p rmg-benches`; no runtime changes.

## 2025-11-01 — Docs rollup automation (pre-commit + subdirs)

- Context: CI rollup check fails if `docs/echo-total.md` drifts; authors asked to trigger the rollup automatically on local commits and include subdirectories.
- Decision: Pre-commit now regenerates the rollup whenever any `docs/**/*.md` file changes (excluding the rollup) and aborts the commit if the rollup changed, prompting review/staging. The generator now includes Markdown files in subdirectories (e.g., `docs/guide/…`) and uses a locale-stable sort (`LC_ALL=C`) with a static header to avoid non-deterministic diffs.
- Consequence: Fewer CI round-trips on docs-only changes; deterministic rollup; preserved index semantics for partial staging.

## 2025-11-01 — PR-10 scope hygiene (tests split)

- Context: PR‑10 (README/CI/docs) accidentally included commit header tests in `snapshot.rs`, overlapping with PR‑09 (tests‑only).
- Decision: Remove the test module from PR‑10 to keep it strictly docs/CI/tooling; keep all BLAKE3 commit header tests in PR‑09 (`echo/pr-09-blake3-header-tests`).
- Consequence: Clear PR boundaries; no runtime behavior change in PR‑10.

## 2025-11-02 — Hotfix: CI docs rollup determinism

- Context: CI rollup job failed on branches where `scripts/gen-echo-total.sh` previously wrote a timestamp header, causing `git diff` to always differ.
- Decision: Update `.github/workflows/echo-total-check.yml` to normalize out `Generated:` lines before diffing, making the check deterministic across old/new rollup formats.
- Consequence: No more false failures; contributors still run `make echo-total` locally when docs change.

## 2025-11-02 — Hotfix follow-up: tighter normalization + annotations

- Decision: Limit normalization to the header region only and accept case/whitespace/legacy variants (`Generated:`, `generated at:`, `Generated by:`). Emit a GitHub Actions `::error` annotation targeting `docs/echo-total.md` when differences remain to improve diagnostics.
- Consequence: Clearer CI failures; minimal, targeted normalization avoids masking content issues.

## 2025-11-02 — CI hotfix: cargo-deny (benches)

- Context: CI `cargo-deny` job failed on PR-11 due to `rmg-benches` lacking a license and a prior wildcard dependency reference reported by CI logs.
- Decision: Add `license = "Apache-2.0"` to `crates/rmg-benches/Cargo.toml` and ensure `rmg-core` is referenced via a path dev-dependency (no wildcard).
- Rationale: Keep workspace policy consistent with other crates (Apache-2.0) and satisfy bans (wildcards = deny) and licenses checks.
- Consequence: `cargo-deny` bans/licenses should pass; remaining warnings are deprecations in `deny.toml` to be addressed in a later sweep.

## 2025-11-02 — cargo-deny modernization

- Context: CI emitted deprecation warnings for `copyleft` and `unlicensed` keys in `deny.toml` (cargo-deny PR #611).
- Decision: Remove deprecated keys; rely on the explicit permissive `allow = [...]` list to exclude copyleft licenses; ensure all workspace crates declare a license (benches fixed earlier).
- Rationale: Keep CI quiet and align with current cargo-deny schema without weakening enforcement.
- Consequence: Same effective policy, no deprecation warnings; future license exceptions remain possible via standard cargo-deny mechanisms.
- CI Note: Use `cargo-deny >= 0.14.21` in CI (workflow/container) to avoid schema drift and deprecation surprises. Pin the action/image or the downloaded binary version accordingly.

## 2025-11-02 — PR-12: benches pin + micro-optimizations

- Context: CI cargo-deny flagged wildcard policy and benches had minor inefficiencies.
- Decision:
  - Pin `blake3` in `crates/rmg-benches/Cargo.toml` to exact patch `=1.8.2` and
    disable default features (`default-features = false, features = ["std"]`) to
    avoid rayon/parallelism in microbenches.
  - `snapshot_hash`: compute `link` type id once; label edges as `e-i-(i+1)` (no `e-0-0`).
  - `scheduler_drain`: builder returns `Vec<NodeId>`; `apply` loop uses precomputed ids to avoid re-hashing.
- Rationale: Enforce deterministic, single-threaded hashing in benches and satisfy
  cargo-deny wildcard bans; reduce noise from dependency updates.
- Consequence: Cleaner dependency audit and slightly leaner bench setup without
  affecting runtime code.

## 2025-11-02 — PR-12: benches constants + documentation

- Context: Pedantic review flagged magic strings, ambiguous labels, and unclear throughput semantics in benches.
- Decision: Extract constants for ids/types; clarify edge ids as `<from>-to-<to>`; switch `snapshot_hash` to `iter_batched`; add module-level docs and comments on throughput and BatchSize; retain blake3 exact patch pin `=1.8.2` with trimmed features to stay consistent with CI policy.
- Rationale: Improve maintainability and readability while keeping dependency policy coherent and deterministic.
- Consequence: Benches read as executable docs; CI docs guard updated accordingly.

## 2025-11-02 — PR-12: benches README + main link

- Context: Missing documentation for how to run/interpret Criterion benches.
- Decision: Add `crates/rmg-benches/benches/README.md` and link from the top-level `README.md`.
- Rationale: Improve discoverability and ensure new contributors can reproduce measurements.
- Consequence: Docs Guard satisfied; single-source guidance for bench usage and outputs.

## 2025-11-02 — PR-12: Sync with main + merge conflict resolution

- Context: GitHub continued to show a merge conflict on PR #113 (`echo/pr-12-snapshot-bench`).
- Decision: Merge `origin/main` into the branch (merge commit; no rebase) and resolve the conflict in `crates/rmg-benches/Cargo.toml`.
- Resolution kept:
  - `license = "Apache-2.0"`, `blake3 = { version = "=1.8.2", default-features = false, features = ["std"] }` in dev-dependencies.
  - `rmg-core = { version = "0.1.0", path = "../rmg-core" }` (version-pinned path dep per cargo-deny bans).
  - Bench targets: `motion_throughput`, `snapshot_hash`, `scheduler_drain`.
- Rationale: Preserve history with a merge, align benches metadata with workspace policy, and clear PR conflict status.
- Consequence: Branch synced with `main`; local hooks (fmt, clippy, tests, rustdoc) passed; CI Docs Guard satisfied via this log and execution-plan update.

## 2025-11-02 — Benches DX: offline report + server reliability

- Context: `make bench-report` started a background HTTP server that sometimes exited immediately; opening the dashboard via `file://` failed because the page fetched JSON from `target/criterion` which browsers block over `file://`.
- Decision:
  - Add `nohup` to the `bench-report` server spawn and provide `bench-status`/`bench-stop` make targets.
  - Add `scripts/bench_bake.py` and `make bench-bake` to generate `docs/benchmarks/report-inline.html` with Criterion results injected as `window.__CRITERION_DATA__`.
  - Teach `docs/benchmarks/index.html` to prefer inline data when present, skipping network fetches.
- Rationale: Remove friction for local perf reviews and allow sharing a single HTML artifact with no server.
- Consequence: Two paths now exist—live server dashboard and an offline baked report. Documentation updated in main README and benches README. `bench-report` now waits for server readiness and supports `BENCH_PORT`.
