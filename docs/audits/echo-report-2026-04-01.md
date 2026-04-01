<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->

# Echo Report

Date: `2026-04-01`

## Scope

This is a static audit of the repository in its current state. I did not run the full workspace, boot every binary, or execute the full test matrix for this report. I read the code, the manifests, the key docs, the validation scripts, the largest modules, and the source-document inventory.

Source-doc scope for the editorial review:

- Included: `README.md`, source docs under `docs/`, crate/app/package/schema/tool `README.md` files.
- Excluded: generated VitePress output, `node_modules`, and other vendored third-party collateral.

## The Unfiltered Verdict

Grade: `C`

Echo has real engineering substance. It also has an obvious self-control problem. The repo contains a genuinely interesting deterministic runtime, but that quality is buried under god modules, optional correctness checks, boundary hardening gaps, and a docs layer that keeps inflating faster than the architecture is converging.

The shortest honest summary is this: the codebase is being pulled in two directions at once. One direction is serious systems engineering. The other is sprawling product/spec/theory/tooling ambition without enough structural discipline.

## What Echo Gets Right

Before getting into the bleeding, here is the part worth protecting:

- The determinism obsession is real, not decorative.
- The Rust workspace lint posture is unusually strong.
- The DIND/verification infrastructure is strategically important and, in several places, very well done.
- The core replay/provenance story is ambitious in a way that is actually rare and valuable.
- Several implementation-facing docs are much better than average and could become the backbone of a disciplined docs set.

### Evidence Bundles: the good stuff

Claim: The workspace lint policy is serious and unusually well-calibrated.

File: Cargo.toml

Lines: 64-116

SHA & Age: 5254e9719354841d0b37958e4dc35ddb8b633ea3 (Written 9 days ago)

The Argument: This is not a fake “strict mode” sticker. The workspace denies `missing_docs`, `unused_must_use`, `unsafe_code`, `unwrap_used`, `expect_used`, `panic`, `todo`, and `unimplemented`, then selectively re-allows only the lints that would otherwise turn the repo into ceremony. That is the right kind of strictness: opinionated, explicit, and still usable.

Claim: Determinism guardrails are enforced by code, not just by marketing copy.

File: scripts/ban-nondeterminism.sh

Lines: 53-119

SHA & Age: b94868b54f137aa1049c9af544c556c4c925c767 (Written 3 weeks ago)

The Argument: The script explicitly scans critical paths for time, randomness, unordered containers, JSON in core paths, float hazards, and host-environment variability. That is what mature systems teams do when they know determinism is too important to leave to “best practices.”

Claim: The local verification path is one of the strongest operational assets in the repo.

File: scripts/verify-local.sh

Lines: 540-805

SHA & Age: 847650677d4269093d35cdeefcb2d715a5917021 (Written 7 days ago)

The Argument: The script classifies changes into docs/reduced/full paths, scopes checks to changed crates, records timing, uses stamps, and routes docs/schema/shell/runtime surfaces through targeted gates. That is better than the usual cargo-cult “just run the whole workspace every time” approach.

Claim: The core tests attack real determinism invariants instead of assertion theater.

File: crates/warp-core/tests/parallel_engine_worker_invariance.rs

Lines: 30-149

SHA & Age: f8b969cbed7f0d6a1eec8545303a2792c763e1bf (Written 3 weeks ago)

The Argument: These tests execute real scenarios across different worker counts and permuted ingress order, then compare `commit_hash`, `state_root`, and `patch_digest`. That is evidence at the real artifact boundary, not fake confidence produced by mocking everything that matters.

## Architecture & Reality Check

### Core stack

- Rust 2021 workspace centered on `warp-core`
- Axum + WebSocket/Unix-socket session infrastructure
- WASM bindings and browser-facing TTD surfaces
- React/Vite/VitePress tooling around the edges
- Heavy spec/ADR/roadmap/book collateral under `docs/`

### What architecture this repo is actually using

Regardless of what the higher-level docs want to grow into, Echo is currently a **monolithic deterministic domain kernel with adapters**. The real product center of gravity is `warp-core`, plus a thin orbit of transport, wasm, viewer, test harness, and docs/tooling surfaces.

This is not yet a clean hexagonal platform with crisp ports and independently understandable layers. It is a powerful core with a lot of secondary surfaces wired around it, some cleaner than others.

## Problem Register

The items below are the actual stop-the-bleeding problems. Each one includes what it is, where it lives, why it is bad, how to fix it, and what the healthy end state would look like.

### 1. The coordinator path is a transaction script wearing a subsystem costume

Claim: The central routing/data-flow path is a hand-rolled transaction script, not a clean orchestration boundary.

File: crates/warp-core/src/coordinator.rs

Lines: 395-544

SHA & Age: 2521150288c7bbf8288d1360285270f190ca5c65 (Written 9 days ago)

The Argument: `super_tick` resolves runnable heads, checkpoints runtime and provenance, admits inbox work, calls into engine commit, appends provenance, records committed ingress, advances ticks, and manually restores state on both errors and panics. That is too much authority and too much failure choreography concentrated in one function.

What it is today:

- A single coordinator function owns routing, execution, bookkeeping, rollback, panic handling, and advancement semantics.
- The happy path is hard enough to follow already. The failure path is worse.

Why it is bad:

- It raises the cost of every change to routing or scheduler semantics.
- It makes correctness depend on a single oversized function continuing to stay internally coherent.
- It prevents the repo from having a stable orchestration contract that other layers can reason about.

How to fix it:

- Split inbox admission, runnable-head selection, commit invocation, provenance append, and tick advancement into separate modules with narrow inputs/outputs.
- Make panic/error rollback a shared transaction primitive, not a one-off branch inside `super_tick`.

Ideal end state:

- `super_tick` becomes a small orchestration wrapper over explicit operations with clear invariants and typed failure modes.

### 2. The engine commit path still relies on a state-swapping shell game

Claim: `engine_impl.rs` contains a manual state-swapping rollback machine because the engine and runtime state models are fighting each other.

File: crates/warp-core/src/engine_impl.rs

Lines: 501-629

SHA & Age: 2521150288c7bbf8288d1360285270f190ca5c65 (Written 9 days ago)

The Argument: `RuntimeCommitStateGuard` saves and restores a large set of engine internals with `std::mem::replace`, `std::mem::take`, an `armed` flag, and mirrored success/error restoration paths. That pattern exists because the engine shape is not aligned with the transaction shape it is being asked to support.

What it is today:

- Commit logic temporarily swaps worldline/runtime state into the engine.
- Success, error, and unwind paths each have restoration behavior.

Why it is bad:

- This is the kind of code that tends to survive until the worst possible moment and then fail under change pressure.
- It is difficult to audit because the real invariant is “did we remember to put the entire machine back the way we found it?”
- It makes refactors to commit, provenance, and runtime coupling much riskier than they need to be.

How to fix it:

- Introduce an explicit `WorldlineCommitContext` or equivalent state object that owns the transaction-local state directly.
- Move receipt generation, patch generation, and runtime mutation decisions behind that context instead of swapping fields in and out of the main engine.

Ideal end state:

- The engine exposes a small commit surface that operates over transaction-local context and produces artifacts, with rollback handled structurally rather than manually.

### 3. The repo promises runtime proof, but production correctness checks are optional

Claim: The repo publicly markets footprint validation as runtime proof.

File: README.md

Lines: 55-69

SHA & Age: 2edadb8b2f776e2b8642a68d32961b45baa286e1 (Written 3 days ago)

The Argument: The front door says reads are checked, writes are validated, violations poison the delta, and “This isn't honor system. It's runtime proof.” That is a strong public claim about the default behavior of the system.

Claim: The actual crate defaults disable footprint enforcement in release, and even exposes an escape hatch to disable it entirely.

File: crates/warp-core/src/lib.rs

Lines: 52-85

SHA & Age: 55f40217fe397549cec512d47f929424089cb76b (Written 4 days ago)

The Argument: The crate-level docs state that release builds disable enforcement unless `footprint_enforce_release` is enabled, and that `unsafe_graph` disables enforcement unconditionally. The Cargo feature definitions reinforce that reality. That means the marketing claim and the runtime default are not aligned.

What it is today:

- Debug/test posture is strong.
- Release posture is throughput-first unless someone explicitly enables enforcement.
- There is a named escape hatch whose description explicitly says it removes determinism safety checks.

Why it is bad:

- It makes the strongest trust claim in the repo partially dependent on build flavor and operator discipline.
- It blurs the difference between “proven in test harnesses” and “enforced in production paths.”
- It gives the docs surface permission to sound safer than the actual shipped defaults.

How to fix it:

- Decide whether footprint enforcement is a non-negotiable safety property or a debug/staging aid.
- If it is non-negotiable, turn it on by default in release and optimize the implementation rather than the truth claim.
- If it is optional, stop calling it runtime proof on the front page and clearly scope the guarantee.

Ideal end state:

- The docs, default binary behavior, and claim register say the same thing.

### 4. The WSC/snapshot story is still overstated

Claim: The snapshot pipeline knowingly cheats on multi-warp state by writing only the first warp and leaving the real problem as a TODO.

File: crates/warp-core/src/snapshot_accum.rs

Lines: 482-493

SHA & Age: 95ffd82735812dd40723777f89011f149c9e1081 (Written 9 weeks ago)

The Argument: The code literally says “use the first (root) warp's input” and “TODO: Support multi-warp WSC files.” That is not a minor edge-case comment. That is an explicit admission that the artifact path is incomplete for a core runtime concept.

What it is today:

- Snapshot/WSC accumulation exists.
- Multi-warp output is not actually fully supported.
- The TODO has been sitting for 9 weeks while high-level docs still sell a stronger story.

Why it is bad:

- It creates a truth gap between product claim and artifact reality.
- It makes downstream trust in replay/serialization docs weaker, because readers now have to guess which claims are complete and which are aspirational.

How to fix it:

- Either implement full multi-warp WSC support immediately or explicitly mark WSC as partial/experimental in all relevant docs.
- Add a failing regression test that represents multi-warp accumulation truthfully so the gap cannot hide.

Ideal end state:

- The artifact pipeline is either complete for the runtime model or honestly marked as incomplete in every place the repo describes it.

### 5. The browser gateway is open by default when it should be defensive by default

Claim: The WebSocket gateway ships with insecure defaults: bind everywhere, trust everyone unless the operator remembers to harden it.

File: crates/echo-session-ws-gateway/src/main.rs

Lines: 467-483

SHA & Age: 537fe83f08b06dbabfbff8fde430d8f4112052c1 (Written 10 days ago)

The Argument: The CLI default is `0.0.0.0:8787`, and the help text says that if no `--allow-origin` values are provided, all origins are accepted.

Claim: The runtime origin check really does allow everything when the allowlist is absent.

File: crates/echo-session-ws-gateway/src/main.rs

Lines: 1289-1298

SHA & Age: 537fe83f08b06dbabfbff8fde430d8f4112052c1 (Written 10 days ago)

The Argument: `origin_allowed` returns `true` immediately when no allowlist is configured. For a browser-facing bridge, that is the wrong default posture.

What it is today:

- The gateway is convenient for local demos.
- The same convenience path can become the default for non-local use.

Why it is bad:

- It normalizes a security posture that depends on operators remembering to harden flags.
- It increases the chance that “temporary local” becomes accidental LAN/public exposure.
- It is especially sloppy because the file is already large enough that policy logic is easy to miss in review.

How to fix it:

- Default to `127.0.0.1`.
- Require explicit opt-in for non-loopback listening.
- Require explicit origin policy or auth token configuration for any non-local exposure.
- Split the module so the security policy is not buried in a 1.6k LOC `main.rs`.

Ideal end state:

- Safe local defaults, explicit remote exposure, and auditable boundary policy.

### 6. The determinism harness contains a documented self-inflicted desync risk

Claim: The DIND rules layer still contains a known double-decoding desync footgun.

File: crates/echo-dind-tests/src/rules.rs

Lines: 292-319

SHA & Age: 08e8dd45d91d075e6983eadbda82e384a4891a07 (Written 3 weeks ago)

The Argument: The file explicitly warns that `decode_op_args` is called in both executor and `compute_footprint`, creating a double-decoding desync risk if decoding has side effects or attachment data changes between calls. That is the wrong kind of TODO to leave in determinism test infrastructure.

What it is today:

- One of the proof surfaces carries a known inconsistency hazard in its own rule definitions.

Why it is bad:

- It weakens trust in the harness that is supposed to prove correctness.
- It introduces the possibility of test behavior diverging from engine behavior for the wrong reason.

How to fix it:

- Decode once and thread a parsed representation into both executor and footprint calculation.
- Add a regression test covering the exact scenario the TODO warns about.

Ideal end state:

- The proof harness is boring, explicit, and free of duplicated semantic work.

### 7. Operational automation is useful, but the control surface is bloated and fragmented

Claim: The maintenance entrypoint has become a giant multi-domain control panel.

File: xtask/src/main.rs

Lines: 1-61

SHA & Age: be1b2a69326cd768814f0e8297cd0d65fc6599ec (Written 4 hours ago)

The Argument: `xtask` now fronts benchmarks, DAG generation, Doghouse, PR flows, DIND, man pages, docs linting, and more. That is a lot of unrelated operational authority in a single binary and, worse, in a single source file.

What it is today:

- `make`, `cargo xtask`, `warp-cli`, and shell scripts all coexist.
- Some of this is healthy layering; some of it is surface duplication.

Why it is bad:

- It increases onboarding cost because there is no single, obvious operational contract.
- It lets operational complexity accumulate in a place that is easy to keep extending and hard to structurally simplify.

How to fix it:

- Decide what belongs in `make`, what belongs in `xtask`, and what belongs in `warp-cli`.
- Split `xtask` by subcommand domain immediately.
- Stop introducing new workflow entrypoints unless an old one is being removed.

Ideal end state:

- One authoritative local verification path, one authoritative developer CLI, and a modular maintenance crate.

### 8. The docs layer keeps creating competing truths

Claim: The architecture doc explicitly says many sections are aspirational and that the current implementation is a Rust-first WARP runtime, not the planned ECS architecture.

File: docs/architecture-outline.md

Lines: 11-12

SHA & Age: 362efb529be3ab17f4685767f6d62379d2b1db74 (Written 2 days ago)

The Argument: The doc is self-aware, which is good, but it also reveals the core problem: architecture prose and implementation reality are not living on the same timeline.

Claim: The `warp-core` README still points readers at ECS storage/scheduler/book material as if that is what the crate implements.

File: crates/warp-core/README.md

Lines: 62-68

SHA & Age: 7c5cb5463629bae6c317cecb17a919351dc72f77 (Written 11 days ago)

The Argument: The README says core engine specs include `spec-ecs-storage.md` and `spec-scheduler.md`, and that the Core booklet describes the architecture, scheduler flow, ECS storage, and game loop that the crate implements. That is not the same message as the architecture outline’s “future design target” note. The docs are arguing with each other.

What it is today:

- The repo has 252 source documents in the audited scope.
- `docs/book/*` contributes 53 files.
- `docs/ROADMAP/*` contributes 53 files.
- `docs/top-level` contributes 49 more source docs.

Why it is bad:

- The reader has to guess which doc is normative, which one is historical, and which one is aspirational.
- Document count is now high enough that “just update the docs” is not a realistic maintenance instruction.
- Some docs are genuinely excellent, which makes the weaker or conflicting ones more damaging.

How to fix it:

- Explicitly declare a doc hierarchy: `front door`, `implementation specs`, `ADR history`, `active plans`, `archive/book collateral`.
- Move book production sources and generated/reference artifacts out of the main docs surface.
- Collapse roadmap shards into a smaller number of milestone/epic docs tied to actual issues.
- Rewrite or cut crate READMEs that still point to planned ECS material as current truth.

Ideal end state:

- A newcomer can tell in one minute what is true today, what is planned, and what is historical context.

## Crate Matrix

| Crate                     | What it has inside                                                                                        | What it does for Echo                                           | Current State                                       | How Critical Is This? | Recommendation                                                                    | Remarks                                                                                        |
| ------------------------- | --------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------- | --------------------------------------------------- | --------------------- | --------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `echo-app-core`           | Small app-service layer: config, preferences, toasts, shared helpers.                                     | Supports local developer-facing apps and UI shells.             | Healthy small helper crate.                         | Medium-Low            | Keep small; stop app-specific logic from leaking into it.                         | Good place for boring shared services, not a second framework.                                 |
| `echo-cas`                | CAS primitives and blob storage logic.                                                                    | Backs artifact, snapshot, or content-addressed storage stories. | Promising but secondary.                            | Medium                | Keep, but tie it to one concrete storage contract and integration path.           | Avoid speculative storage layers before the runtime core is simpler.                           |
| `echo-config-fs`          | Tiny filesystem-backed config adapter.                                                                    | Lets tools persist local config without polluting core crates.  | Tiny and fine.                                      | Low                   | Leave it boring and don't over-abstract it.                                       | This is the kind of small crate the repo needs more of.                                        |
| `echo-dind-harness`       | Harness code, scenario runner, cross-run verification plumbing.                                           | Provides determinism proof infrastructure.                      | Strong and strategically important.                 | High                  | Invest more here; it protects the repo's central claim.                           | One of the better examples of focused engineering in the tree.                                 |
| `echo-dind-tests`         | Scenario kernel, rules, fixtures for DIND suites.                                                         | Exercises determinism and replay end-to-end.                    | Useful but has correctness footguns.                | High                  | Fix duplicate decode paths and keep scenarios close to real engine semantics.     | Test infrastructure that can create its own bugs is dangerous.                                 |
| `echo-dry-tests`          | Shared test doubles, fixtures, support utilities.                                                         | Reduces duplication across crate tests.                         | Useful but too large for a helper crate.            | Medium                | Split by domain or crate family before it becomes a fake platform.                | A 2.5k LOC test helper crate is already a warning sign.                                        |
| `echo-graph`              | Canonical renderable graph DTOs and payload structures.                                                   | Bridges runtime state to render/tooling surfaces.               | Reasonably bounded.                                 | High                  | Keep contract narrow and versioned.                                               | Good seam if the repo avoids stuffing behavior into DTO crates.                                |
| `echo-registry-api`       | Thin registry traits and schema hooks.                                                                    | Lets wasm/schema layers ask for codecs and ops catalogs.        | Small and fine.                                     | Medium-Low            | Keep minimal and resist feature creep.                                            | An API crate should stay painfully boring.                                                     |
| `echo-runtime-schema`     | ADR-0008 runtime schema types and mapping contracts.                                                      | Defines shared runtime schema surfaces.                         | Valuable but should freeze soon.                    | High                  | Treat as contract-first and keep churn low.                                       | Schema crates should feel stable even when the app layer is not.                               |
| `echo-scene-codec`        | Canonical CBOR codec, decoding/encoding tests, harnesses.                                                 | Owns deterministic scene serialization.                         | Important and fairly mature.                        | High                  | Keep hardening decoders and property/integration tests.                           | Serialization is a contract surface, not a playground.                                         |
| `echo-scene-port`         | Scene/render port traits and payload boundaries.                                                          | Defines the rendering boundary for Echo/TTD.                    | Good architectural seam.                            | High                  | Keep strict and small; do not let renderer concerns backflow inward.              | This is one of the healthier boundaries in the repo.                                           |
| `echo-session-client`     | Client helpers, hub connection wrappers, protocol glue.                                                   | Lets tools talk to the session service.                         | Useful but ordinary.                                | Medium                | Add/keep integration coverage and avoid custom retry labyrinths.                  | Should remain a thin convenience layer.                                                        |
| `echo-session-proto`      | Session hub wire types, commands, notifications, stream schemas.                                          | Defines browser/native session transport contracts.             | Important but bulky.                                | High                  | Generate more of it or split by protocol version/domain.                          | Protocol crates bloat quickly when every message shape lives together forever.                 |
| `echo-session-service`    | Headless hub server and bridge logic.                                                                     | Runs the central session hub for tools.                         | Important, moderate size.                           | High                  | Strengthen integration tests and keep public surface explicit.                    | This is operational infrastructure and should behave like one.                                 |
| `echo-session-ws-gateway` | Browser WebSocket bridge, dashboard, metrics, TLS/origin handling.                                        | Exposes session service to browser tools.                       | Security-sensitive and too monolithic.              | High                  | Harden defaults and split transport, auth/origin, and dashboard code paths.       | A 1.6k LOC `main.rs` for a public gateway is asking for sloppy boundary logic.                 |
| `echo-ttd`                | TTD compliance and engine glue.                                                                           | Supports time-travel debugger behavior and contracts.           | Useful but not minimal.                             | Medium-High           | Keep focused on TTD contracts, not UI policy.                                     | Should be a compliance engine, not a mystery box.                                              |
| `echo-wasm-abi`           | WASM-friendly DTOs and graph/rewrite types.                                                               | Defines cross-language contract surfaces.                       | Important but large.                                | High                  | Freeze ABI surfaces and reduce hand-maintained shape drift.                       | ABI crates are expensive places to improvise.                                                  |
| `echo-wasm-bindings`      | wasm-bindgen shim layer and exports.                                                                      | Exposes narrow Rust surfaces to WASM consumers.                 | Thin and useful.                                    | Medium-High           | Keep tiny and adapter-like.                                                       | Bindings should not become a second business-logic layer.                                      |
| `echo-wesley-gen`         | Code generator from Wesley IR to Rust artifacts.                                                          | Promises typed generated surfaces instead of manual drift.      | Potentially strategic.                              | Medium                | Either lean into generation or cut back the implied scope.                        | Generators are only worth it if they actually shrink hand-maintained surface area.             |
| `ttd-browser`             | WASM/browser TTD engine glue in one large lib.                                                            | Runs debugger/runtime affordances in the browser.               | Important but oversized.                            | High                  | Split protocol, runtime state, and UI-facing adapter layers.                      | Browser boundary code should not require reading a 1.6k LOC lib.rs.                            |
| `ttd-manifest`            | Vendored manifest/data-only crate.                                                                        | Pins protocol manifest assets.                                  | Fine as data-only.                                  | Medium                | Keep generated/readonly and out of architectural debates.                         | Good candidate for zero drama.                                                                 |
| `ttd-protocol-rs`         | Generated Rust protocol types.                                                                            | Provides typed protocol structs for TTD surfaces.               | Fine if treated as generated output.                | Medium-High           | Do not hand-edit; regenerate and document provenance.                             | Generated code should never masquerade as artisanal architecture.                              |
| `warp-benches`            | Criterion benches and benchmark scenarios.                                                                | Produces evidence for performance claims.                       | Useful, with some missing gates.                    | Medium                | Make artifact persistence and regression gates real, not TODOs.                   | Bench suites matter more when they drive decisions than when they decorate docs.               |
| `warp-cli`                | Developer CLI: verify, bench, inspect, maybe more over time.                                              | Offers a repo-native operational entrypoint.                    | Useful but overlapping with xtask/make.             | Medium-High           | Clarify ownership versus `xtask` and make one path authoritative.                 | Two operational CLIs is how command surfaces rot.                                              |
| `warp-core`               | 130 source files implementing engine, scheduler, provenance, patching, observation, runtime coordination. | It is the heart of Echo.                                        | Technically impressive and structurally overloaded. | Critical              | Refactor around smaller authority centers before adding more capability.          | This crate contains the repo's best ideas and worst concentration risk.                        |
| `warp-geom`               | Geometry/math primitives, broad-phase scaffolding, deterministic helpers.                                 | Supports deterministic geometry and collision-adjacent work.    | Reasonable but not finished.                        | Medium-High           | Replace placeholder/reference implementations before more systems depend on them. | Geometry debt compounds fast once physics or gameplay starts leaning on it.                    |
| `warp-viewer`             | Interactive renderer, viewer shell, snapshot inspection plumbing.                                         | Gives humans a way to inspect WARP state.                       | Useful, somewhat large.                             | Medium-High           | Split rendering core, app shell, and protocol adapters.                           | Viewer code is a magnet for accidental architecture unless kept modular.                       |
| `warp-wasm`               | wasm-bindgen exports, kernel-facing WASM surface, browser/tool adapters.                                  | Makes core/runtime reachable from web tooling.                  | Important but larger than ideal.                    | High                  | Shrink the exported surface and keep runtime logic in core crates.                | WASM should be a boundary, not a second home for domain policy.                                |
| `spec-000-rewrite`        | Tiny living-spec scaffold/demo under `specs/`.                                                            | Acts as a demo/spec spike.                                      | Experimental and peripheral.                        | Low-Medium            | Either promote it into a maintained example or quarantine it as a spike.          | Spec demos are useful only when someone owns them.                                             |
| `xtask`                   | Single enormous maintenance CLI covering benches, docs, DAGs, PR workflows, Doghouse, DIND, manpages.     | Runs a lot of repo operations and glue automation.              | Operationally useful, architecturally bloated.      | Medium                | Split by subcommand domain into modules or separate crates.                       | 7.6k LOC in one file is not convenience. It is deferred maintenance with a clap derive on top. |

## BIG BOIZ™: God Modules

The genuinely dangerous god modules are not just “large files.” They are the files that own too many unrelated reasons to change.

| Module                                             | LOC  | What It Does                                                                                      | Why It Got Huge                                                                                  | Cleanup Move                                                                                   | Remarks                                                                       |
| -------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `xtask/src/main.rs`                                | 7630 | Maintenance CLI for benches, docs, DAGs, PR workflows, Doghouse, DIND, and manpages.              | It kept absorbing every repo convenience until convenience became a 7.6k LOC control panel.      | Split by subcommand domain into `xtask/src/{bench,dogs,docs,pr,...}.rs` or separate crates.    | High operational drag; hard to review, hard to test, easy to keep adding to.  |
| `crates/warp-core/src/engine_impl.rs`              | 3464 | Engine commit pipeline, transaction handling, runtime state manipulation, receipt/patch plumbing. | Too much engine authority accumulated here because the internal model is not decomposed cleanly. | Extract commit pipeline, runtime context, rule application, and reservation/receipt flows.     | Primary kernel god module.                                                    |
| `crates/warp-core/src/provenance_store.rs`         | 3362 | Checkpointing, replay, BTR validation, lineage/provenance reconstruction.                         | Replay and provenance logic grew into one giant correctness file.                                | Split into checkpoint store, replay engine, BTR validation, and history adapters.              | Critical but cognitively heavy.                                               |
| `crates/warp-core/src/tick_patch.rs`               | 1928 | Patch artifacts, op encoding, replay boundary logic.                                              | Boundary artifact evolution piled into a single file instead of narrow versioned modules.        | Split by artifact version and by concerns: model, encoding, validation, helpers.               | Contract surface deserves cleaner version boundaries.                         |
| `crates/warp-core/src/coordinator.rs`              | 1740 | Worldline coordination, inbox admission, super-tick orchestration, runtime control.               | Routing/orchestration complexity accreted in one place.                                          | Extract inbox routing, scheduler cycle, control intents, and failure recovery.                 | Another major authority center.                                               |
| `crates/warp-core/src/tick_delta.rs`               | 1668 | Delta model, ops accumulation, validation support, patch-related helpers.                         | Delta semantics expanded without being modularized.                                              | Split data model, builders, validation, and merge/replay helpers.                              | Boundary code should be easier to audit than this.                            |
| `crates/echo-session-ws-gateway/src/main.rs`       | 1604 | CLI, gateway server, dashboard, metrics, TLS/origin rules, ws transport.                          | Everything lives in main because no one forced a real service/module split.                      | Split config parsing, HTTP routes, ws bridge, auth/origin policy, observer/dashboard.          | Security-sensitive god main.                                                  |
| `crates/ttd-browser/src/lib.rs`                    | 1593 | Browser-facing TTD engine state, wasm exports, integration glue.                                  | The browser boundary became a dumping ground for runtime-adjacent logic.                         | Split protocol DTO translation, session model, wasm exports, and browser adapter code.         | Important UX boundary with too much hidden complexity.                        |
| `crates/warp-core/src/scheduler.rs`                | 1580 | Reservation logic, active sets, ordering, tests, legacy path.                                     | Scheduler behavior and its evidence live together in a dense block.                              | Split core scheduler, active-set data structures, ordering helpers, and tests/docs references. | One of the healthier big files, but still too big.                            |
| `crates/warp-core/src/snapshot_accum.rs`           | 1555 | Snapshot accumulation, serialization assembly, WSC plumbing.                                      | Artifact code kept growing while the feature story stayed incomplete.                            | Split accumulation, WSC writer, multi-warp handling, and validation.                           | Contains a truth-gap TODO for multi-warp output.                              |
| `crates/warp-core/src/parallel/exec.rs`            | 1482 | Parallel execution worker orchestration and merge flow.                                           | Parallel runtime complexity is concentrated here because abstractions are still low-level.       | Split worker scheduling, guarded execution, and result collation.                              | Big but expected; keep under control before more features land.               |
| `crates/warp-core/tests/reducer_emission_tests.rs` | 1395 | Large reducer/materialization test suite.                                                         | Evidence accreted into one mammoth test file instead of themed suites.                           | Split by reducer operation or invariant family.                                                | Not production code, but still review-hostile.                                |
| `crates/echo-session-proto/src/ttdr_v2.rs`         | 1309 | Versioned TTD/session wire schema definitions.                                                    | Protocol surface is big because the product surface is big.                                      | Prefer generation or per-domain versioned modules.                                             | Large generated/contract file; less alarming than human-authored god modules. |
| `crates/warp-core/src/snapshot.rs`                 | 1219 | Snapshot model and helpers.                                                                       | Boundary artifacts are scattered across several large files.                                     | Split hashing/model/format glue.                                                               | Secondary god module.                                                         |
| `crates/warp-wasm/src/warp_kernel.rs`              | 1203 | WASM kernel exports and glue.                                                                     | Boundary glue got thick as more runtime semantics leaked outward.                                | Shrink export surface and split adapter helpers.                                               | WASM-facing complexity is accumulating.                                       |
| `crates/echo-scene-codec/src/cbor.rs`              | 1117 | Canonical CBOR codec implementation.                                                              | Serialization code is inherently fiddly, but still needs modular seams.                          | Split encoder, decoder, validation, and security tests/support.                                | Large but understandable for a codec crate.                                   |
| `crates/warp-core/tests/common/mod.rs`             | 1069 | Shared integration test setup and utilities.                                                      | Test common modules balloon when suites depend on too much shared harness magic.                 | Split fixture builders by subsystem.                                                           | Test sprawl signal.                                                           |
| `crates/warp-core/src/playback.rs`                 | 1052 | Playback and cursor logic.                                                                        | Feature grew into its own sizable subsystem.                                                     | Split cursor state, stepping logic, and observation helpers.                                   | Important but still manageable.                                               |
| `crates/ttd-browser/pkg/ttd_browser.js`            | 1018 | Large file.                                                                                       | Complexity accumulated here.                                                                     | Split by concern.                                                                              | Needs review.                                                                 |
| `crates/warp-core/src/observation.rs`              | 1010 | Observation service and projection logic.                                                         | Read-path semantics are rich enough to justify submodules.                                       | Split observation requests, projections, and result shaping.                                   | Another core subsystem nearing refactor threshold.                            |
| `crates/warp-core/tests/outputs_playback_tests.rs` | 983  | Large file.                                                                                       | Complexity accumulated here.                                                                     | Split by concern.                                                                              | Needs review.                                                                 |
| `crates/ttd-protocol-rs/lib.rs`                    | 961  | Large file.                                                                                       | Complexity accumulated here.                                                                     | Split by concern.                                                                              | Needs review.                                                                 |
| `crates/echo-dind-tests/src/rules.rs`              | 950  | Large file.                                                                                       | Complexity accumulated here.                                                                     | Split by concern.                                                                              | Needs review.                                                                 |
| `crates/warp-core/tests/slice_theorem_proof.rs`    | 949  | Large file.                                                                                       | Complexity accumulated here.                                                                     | Split by concern.                                                                              | Needs review.                                                                 |

### Human-maintained god modules that matter most

If you only care about the files most likely to keep hurting the repo, focus here first:

1. `crates/warp-core/src/engine_impl.rs`
2. `crates/warp-core/src/provenance_store.rs`
3. `crates/warp-core/src/coordinator.rs`
4. `crates/warp-core/src/snapshot_accum.rs`
5. `crates/echo-session-ws-gateway/src/main.rs`
6. `xtask/src/main.rs`

Generated files and large test artifacts matter less than human-maintained authority centers.

## Docs: Ruthless Editor's Cut

The docs problem is not that the repo lacks writing. The docs problem is that the repo has too many things trying to be the truth at once.

High-level editorial judgment:

- Keep and actively maintain: `docs/index.md`, `docs/spec-warp-core.md`, `docs/scheduler-warp-core.md`, `docs/determinism/DETERMINISM_CLAIMS_v0.1.md`, the best contributor procedures, the best guides.
- Fix aggressively: `README.md`, `docs/architecture-outline.md`, `crates/warp-core/README.md`, any doc that blurs implemented runtime versus planned ECS/product future.
- Merge: roadmap shards, plan shards, some top-level theory/methodology/performance slices.
- Move: book build sources, diagram sources, generated benchmark artifacts, manpages/reference collateral.
- Cut: dated one-off plan docs and any document that no longer serves as either live truth or durable history.

The full doc-by-doc table is in the companion appendix:

- `docs/audits/echo-docs-editorial-cut-2026-04-01.md`

## Action Plan: How to Get This Repo from C to S+

### Phase 0: Stop the bleeding (week 1)

1. Make the gateway safe by default.
    - Bind to loopback by default.
    - Require explicit opt-in for remote exposure.
    - Require explicit auth/origin policy for non-local use.
2. Resolve the runtime-proof truth gap.
    - Either enable footprint enforcement by default in release or rewrite the claim surface so it stops pretending.
3. Fix the known DIND double-decoding footgun.
4. Mark WSC/multi-warp support honestly everywhere until it is complete.

### Phase 1: Decompose the kernel authority centers (weeks 2-4)

1. Split `engine_impl.rs` into commit pipeline, runtime context, and artifact/receipt modules.
2. Split `coordinator.rs` into routing/admission, scheduler cycle, and rollback/recovery.
3. Split `provenance_store.rs` into checkpoint, replay, and validation modules.
4. Split `snapshot_accum.rs` into accumulation, encoding, and validation layers.

### Phase 2: Clean the operational surface (weeks 3-5)

1. Break `xtask/src/main.rs` into subcommand-domain modules.
2. Decide whether `warp-cli` or `xtask` is the developer-facing CLI of record.
3. Keep `make` as a thin alias layer only.
4. Reduce ad hoc script proliferation where the logic already exists in `verify-local` or the chosen CLI.

### Phase 3: Repair the docs contract (weeks 4-6)

1. Establish doc classes:
    - `Front Door`
    - `Implementation Spec`
    - `ADR / Historical Decision`
    - `Active Plan`
    - `Reference / Generated`
    - `Archive / Book`
2. Move book sources and generated/reference artifacts out of the main docs path.
3. Cut or merge roadmap fragments until milestone docs are human-sized.
4. Rewrite root and crate READMEs so they stop pointing at planned ECS/product docs as implemented truth.

### Phase 4: Protect the repo with budgets (ongoing)

1. Enforce file-size and module-authority budgets for new code.
2. Require every new truth claim to point at a test, artifact, or validation gate.
3. Add integration tests for session-service <-> gateway <-> browser-facing flows.
4. Add artifact-level regression tests for the multi-warp snapshot path.

### What S+ looks like

An S+ version of this repo would have these properties:

- A newcomer can understand the runtime happy path in two days, not two weeks.
- The biggest kernel flows are split across comprehensible modules with explicit boundaries.
- Safe defaults are the default, not a flag people are expected to remember.
- The docs surface makes it obvious what is implemented, what is planned, and what is archival.
- Performance and determinism claims are tied to stable evidence and consistent runtime behavior.
- The repo stops shipping contradictory truths.

## Day One / Week One Ownership Advice

### Day One

Run these in order:

```bash
make verify-fast
cargo test --workspace
cargo xtask dind run
pnpm docs:build
```

If you want one visible surface after verification, use:

```bash
cargo run -p warp-viewer
```

### Week One

If I were taking ownership, the first three mandatory fixes would be:

1. Harden the WebSocket gateway defaults and exposure model.
2. Resolve the footprint-enforcement truth gap.
3. Start the `warp-core` authority split with `engine_impl.rs` and `coordinator.rs`.

## FIX THIS MESS Kit

If you want this cleanup to stick, do not stop at refactors. Put operating constraints in place.

### 1. Canonical repo map

Create a single short doc that answers:

- what Echo is today
- what crates are tier-1 runtime
- what surfaces are experimental
- what docs are authoritative

### 2. Authority budgets

Add explicit rules like:

- no new human-maintained source file over 1,000 LOC without a written exception
- no new module that owns more than one primary concern
- no new public runtime claim without a test or gate backing it

### 3. Truth policy for docs

Every doc should declare one of:

- implemented truth
- design intent
- active plan
- historical/archive
- generated/reference

### 4. Security defaults policy

Any network-facing surface must justify:

- default bind address
- auth story
- cross-origin story
- exposure model
- operational limits

### 5. Ownership map

Assign explicit owners for:

- `warp-core`
- session transport/gateway
- DIND/verification
- docs/editorial gatekeeping
- operational tooling

### 6. Delete path

The repo needs a real archival habit. Old plans, obsolete docs, and superseded operational surfaces should be merged, moved, or cut on purpose. Otherwise the next six months will recreate the same mess one layer deeper.

## Final Judgment

Echo is not a bad repo. It is a repo with enough genuine technical ambition to deserve much better structural discipline than it currently has.

If the team protects the determinism rigor, slims the kernel authority centers, hardens the external boundaries, and stops letting the docs pretend every future idea is already architecture, this can become a serious top-tier systems repo. If not, it will remain a clever, exhausting C-grade monorepo that keeps making its own best ideas harder to trust.
