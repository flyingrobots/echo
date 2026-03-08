<!-- SPDX-License-Identifier: Apache-2.0 OR LicenseRef-MIND-UCAL-1.0 -->
<!-- © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots> -->
<!-- Branch-specific task list for docs/polish-41 (PR #292) -->

# Tasks: docs/polish-41

CodeRabbit review comments — all 66 remaining items. Nothing gets pushed until
every box is checked.

---

## Group A: Verify Local Fixes (32 items)

These files already have local edits. Verify each comment is fully addressed,
then check the box. If a comment is only _partially_ addressed, move it to
Group B.

### spec-scheduler.md (5 items — all locally fixed)

- [x] **[MAJ] line 50:** `phases` cannot be both required and defaulted.
      Made `phases` optional with `?` suffix. Registration already had `?? ["update"]` fallback. Contract is now consistent.
- [x] **[MAJ] line 143:** Dedup rule broken by unconditional `inDegree` bumps.
      Added `.has()` guard before `.add()` + inDegree increment in both after/before loops.
- [x] **[MAJ] line 183:** `initialize` never runs for systems added after tick 1.
      Changed to run INITIALIZE unconditionally every tick; only systems with `status: "pending"` execute.
- [x] **[MAJ] line 278:** `unpauseable` is not a conflict predicate.
      Removed `and not unpauseable` from batching condition. Unpauseable only affects pause handling.
- [x] **[MIN] line 406:** Wrong warp-core type name.
      Changed `FootprintInfo` → `Footprint` in open questions.

### spec-editor-and-inspector.md (7 items — 3 locally fixed, 4 need work)

- [x] **[MAJ] line 6:** Broken cross-reference links.
      Fixed `/guide/warp-primer` → `guide/warp-primer.md` and `docs/spec-time-streams...` → `spec-time-streams...`.
- [x] **[CRIT] line 36:** `object` typing too permissive.
      Changed `payload: object` → `payload: unknown`.
- [x] **[MAJ] line 40:** Draft note needs exact missing artifacts.
      Updated to name `types.ts` and `registry.ts` with specific missing types.
- [x] **[CRIT] line 43:** Sorting algorithm for deterministic frame ordering unspecified.
      **Done:** Added normative paragraph specifying stable sort ascending by `(tick, frameType)`, unsigned integer comparison for tick, UTF-8 lexicographic comparison for frameType, stable tie-breaking by insertion order. Applies to both in-memory buffer and JSONL log.
- [x] **[MAJ] line 51:** "Signed session token" undefined.
      **Done:** Added "Session Token Format" subsection specifying HMAC-SHA256 over `{sessionId, capabilities, issuedAt, expiresAt}`, `base64url` wire format, and 401 rejection semantics.
- [x] **[MAJ] line 62:** `filter` field structure undefined.
      **Done:** Added "Filter Semantics" subsection: flat key-value map, exact-match AND-combined predicates, unknown keys silently ignored, with example.
- [x] **[MAJ] line 96:** `producer` return type too loose.
      **Done:** Changed `producer` return type from `object` to `unknown`.

### xtask/src/main.rs (1 item — locally fixed)

- [x] **[MAJ] line 791:** SPDX repair ignores `--root`.
      Scoped `ensure_spdx.sh` to pass `md_files` as positional args instead of running repo-wide.

### memorials/2026-01-18-phase4-rubicon.md (3 items — 1 fixed, 2 need work)

- [x] **[MIN] line 111:** Emphasis half-fixed.
      **Done:** Prettier enforces underscore emphasis (`_..._`), overriding asterisks. `_Alea iacta est_.` is the correct form under this repo's prettier config.
- [x] **[CRIT] line ~21:** Revert underscore emphasis to asterisks.
      **Done:** `base + ops = next` is a computational formula — code span is correct. Kept as-is per user decision.
- [x] **[MIN] line ~111:** Foreign phrase requires italics, not emphasis.
      **Done:** Changed to `<i>Alea iacta est</i>.` (semantic HTML for foreign phrase, avoids prettier underscore/asterisk conflicts).

### spec-merkle-commit.md (3 items — 2 fixed, 1 partially)

- [x] **[MIN] line 6:** Root-relative link.
      Fixed `/guide/warp-primer` → `guide/warp-primer.md`.
- [x] **[MAJ] line 78:** Parent count validation in `compute_commit_hash_v2()`.
      Added MUST-validate sentence.
- [x] **[TRIV] line 203:** Consolidate empty digest definition.
      `EMPTY_LEN_DIGEST` constant defined at line 195 with cross-reference to engine's `DIGEST_LEN0_U64`. Invariants section (lines 201-203) explains the semantic distinction from `blake3(b"")`.

### Other locally-fixed files (13 items)

- [x] **[MIN] docs/adr/ADR-0004-No-Global-State.md:178** — Over-escaped `install\_\*` in code block.
      **Done:** Prettier re-escapes underscores/asterisks inside ` ```markdown ` fences (it formats the content as markdown). The escapes are cosmetic — they render identically to unescaped versions in markdown renderers. Accepted as prettier-enforced.
- [x] **[CRIT] docs/archive/spec-geom-collision.md:34** — Broken cross-ref `SPEC_DETERMINISTIC_MATH.md`. Fixed → `spec-deterministic-math.md`.
- [x] **[CRIT] docs/notes/scheduler-optimization-followups.md:30** — Proptest missing.
      **Done:** Added 3 proptests to `scheduler.rs`: `proptest_drain_matches_btreemap_reference` (fuzzes both sort paths against BTreeMap reference, n=1..2048), `proptest_insertion_order_independence` (verifies drain output is order-invariant), `threshold_boundary_determinism` (exercises n=1023/1024/1025). Also fixed a pre-existing radix sort bug: `bucket16` scope pair index was inverted (LSD passes processed MSB-first instead of LSB-first), causing comparison-sort and radix-sort paths to produce different orderings at the SMALL_SORT_THRESHOLD boundary.
- [x] **[MAJ] docs/notes/scheduler-optimization-followups.md:65** — Radix sort docs incomplete.
      **Done:** Added comprehensive "Radix Sort Internals" subsection documenting: `RewriteThin` layout, 20-pass rationale, 16-bit digit trade-off table, LSD stability requirement, pass sequence diagram, `bucket16` digit extraction, three-phase counting sort algorithm, ping-pong buffer pattern, and threshold justification.
- [x] **[MAJ] docs/notes/scheduler-optimization-followups.md:201** — Ambiguous benchmark note. Fixed.
- [x] **[MIN] docs/spec-ecs-storage.md:6** — Root-relative link. Fixed.
- [x] **[MIN] docs/spec-geom-collision.md:7** — Vague deferral. Fixed.
- [x] **[MIN] docs/spec-mwmr-concurrency.md:6** — Broken link. Fixed.
- [x] **[MIN] docs/spec-mwmr-concurrency.md:51** — Name "Theorem A".
      **Done:** Replaced with "Skeleton-plane Tick Confluence theorem (Paper II, §6, Thm. 6.1)" — the formal statement that any two serialisations of a scheduler-admissible batch yield isomorphic successors.
- [x] **[MAJ] docs/spec-warp-confluence.md:66** — Signing canonicalization underspecified.
      **Done:** Added "Signing Canonicalization" normative subsection with exact 8-field canonical byte sequence (root_hash, parent_hash, diff_count, diff_hashes, signer_id, capability_count, capabilities, timestamp), encoding types, and MUST-reject clause.
- [x] **[MIN] docs/spec-warp-confluence.md:6** — Root-relative link. Fixed.
- [x] **[MAJ] docs/spec-world-api.md:6** — Broken primer link. Fixed.
- [x] **[MAJ] docs/spec-world-api.md:~92** — Version management too vague.
      **Done:** Added "Breaking-Change Criteria" (4 criteria) and "Deprecation Timeline" (3-phase: announce → no-op → remove, minimum 2 minor releases or 90 days).

---

## Group B: New Work Required (34 items)

These files have no local changes yet. Each needs investigation + fix.

### spec-branch-tree.md (10 items — spec completeness)

This spec has significant gaps that CR flagged. Every item relates to
determinism: undefined types/formulas mean implementations could diverge.

- [x] **[MAJ] line 36:** Define `ReadKey` and `WriteKey` as formal interfaces.
      **Done:** Defined `AccessKey = { slot: u32, fieldPath?: CanonicalFieldPath }` with `ReadKey`/`WriteKey` aliases. Added `QualifiedKey` for cross-scope use. Documented layering: Aion `Del/Use` → confluence `R/W/D/A` → ECS `slot+fieldPath`.
- [x] **[MAJ] line 60:** Formalize `MergeStrategyId` type.
      **Done:** Extensible namespaced string (`core:lww`, `core:sum`, etc.). Non-core strategies require resolver manifest digest. Removed `domainResolver` (escape hatch, not a strategy). Plugin loading ABI deferred to post-Phase 0.
- [x] **[CRIT] line 116:** Hash formula references non-existent field.
      **Done:** Extracted `TimelineNodeCore` (hashable subset: `parents`, `branchId`, `chronos`, `snapshotId`, `diffId`). Replaced `parentId + mergeParents?` with `parents: Hash[]`. Moved `aionWeight`/`strainDelta` to `TimelineMetadata` sidecar. Formula: `id = BLAKE3(canonicalEncode(TimelineNodeCore))`.
- [x] **[MAJ] line 177:** Define entropy formula weights.
      **Done:** Renamed to "branch strain." Configurable per-world, fixed-point integers. Defaults: wF=5, wC=25, wP=50, wM=15, wX=20. Raw total in [0,∞), floor at 0. `imports` = cross-branch messages.
- [x] **[MAJ] line 199:** Clarify byte-level encoding in seed derivation.
      **Done:** Domain-separated canonical encoding: `BLAKE3(canonicalEncode({ domain: "echo.branch-seed.v1", seed: 32 bytes, branchId: length-prefixed UTF-8, chronos: u64 LE }))`.
- [x] **[MAJ] line 206:** Clarify all GC modes are deterministic.
      **Done:** Three explicit modes: `periodic`, `checkpoint`, `none`. No adaptive mode. Split "disabled" vs "deferred." Pin semantics: full transitive reachable closure.
- [x] **[MAJ] line 252:** Define `WorldView` and `GCPolicy` types.
      **Done:** `WorldView`: lightweight read-only handle with `chronos`, `schemaLedgerId`, `getChunkVersion()`, `readComponentCanonical()`. `GCPolicy`: `mode` + `intervalTicks` + `retainDepth` + `retainBaseSnapshots` + `respectPins`.
- [x] **[MIN] line 300:** Define causal relation semantics.
      **Done:** Layered model: within-node (Paper II tick-event poset), cross-tick (parents/chronos ancestry), cross-branch (merge parents). Network frontier causality out of scope. Defined all four edge relations. Added note: Chronos is per-branch, not global.
- [x] **[MIN] line 373:** Specify entropy bounds and initialization.
      **Done:** Renamed to "strain." Genesis at 0. Fork inherits parent total. Merge continues from target + delta. Per-node canonical, BranchRecord caches head. No reset on collapse. Saturation → gameplay policy, not scalar behavior.
- [x] **[MIN] line 390:** Define capability token structure.
      **Done:** Forward-reference to `spec-capabilities-and-security.md`. Branch-tree stores `CapabilityAssertion { tokenDigest, scope }`. Violations emit deterministic error nodes.

### spec-temporal-bridge.md (1 item)

- [x] **[CRIT] line 115:** API exposes opaque `NodeId`s but lifecycle rules dereference them as full nodes.
      **Done:** Added `getNode(id: NodeId): TimelineNode` to `BridgeContext`. Added disambiguation note clarifying timeline `NodeId` (hex-encoded content-addressed `Hash`) vs echo-graph `NodeId` (`u64`). API keeps `NodeId` as parameter type; bridge resolves internally via `getNode()`.

### spec-runtime-config.md (1 item)

- [x] **[CRIT] line ~54:** `world:config` capability undefined.
      **Done:** Added `"world:config"` to `Capability` union type and `Runtime config` row to Capability Scopes table in `spec-capabilities-and-security.md`. Removed "not yet defined" warning from `spec-runtime-config.md` line 61.

### spec-serialization-protocol.md (2 items)

- [x] **[MIN] line 6:** Root-relative link. Fix `/guide/eli5` → `guide/eli5.md`.
      **Done:** Already `guide/eli5.md` — verified correct.
- [x] **[MAJ] line 141:** `payloads` field semantics and serialization order incomplete.
      **Done:** Specified `BlockManifest` encoding: declaration-order sections, each with `sectionTag (uint8)` + `count (uint32 LE)` + sorted hashes. Empty sections encoded with `count = 0` and tag always present.

### spec-time-streams-and-wormholes.md (2 items)

- [x] **[MAJ] line 189:** StreamAdmissionDecision canonical field ordering.
      **Done:** Line 192 has normative MUST language: "Implementations MUST NOT reorder fields." Verified sufficient.
- [x] **[TRIV] line 509:** Narrative example numbering creates maintenance burden.
      **Done:** Converted numbered steps (1–4) to bold-labeled bullets.

### SPEC-0002-descended-attachments-v1.md (5 items — formatting consistency)

- [x] **[MAJ] line 3:** Blank-line policy chaos.
      **Done:** Ran prettier (already formatted). Verified blank lines after all headings are consistent.
- [x] **[MAJ] line 52:** AttachmentPlane consolidation inconsistency.
      **Done:** Unified all enum definitions to sub-bullet style: `AttachmentPlane`, `AttachmentOwner`, `AttachmentValue`, and `PortalInit` all use `name:` header + sub-bullet variants with em-dash descriptions.
- [x] **[MAJ] line ~53:** Enum variant nesting inconsistency.
      **Done:** Covered by the enum style unification above.
- [x] **[TRIV] line 192:** Algorithm formatting — verify logical structure preserved.
      **Done:** Verified S3 DAG slicing algorithm: 4 steps intact, step 2 has (a)/(b) sub-cases, step 4 has portal-chain closure. Logical structure correct.
- [x] **[TRIV] line 226:** Header spacing — acceptable but inconsistent.
      **Done:** Prettier confirms formatting is correct. All headings followed by blank lines.

### SPEC-0003-dpo-concurrency-litmus-v0.md (1 item)

- [x] **[MIN] line 45:** Calling read/read overlap "disjoint" is wrong.
      **Done:** Changed `remain disjoint` → `are non-conflicting` on line 61.

### Other docs (12 items)

- [x] **[TRIV] docs/DETERMINISTIC_MATH.md:52** — Tighten "very small numbers" to "subnormal values (magnitude < 2^−126)".
      **Done:** Updated line 47 with `subnormal values (magnitude < 2⁻¹²⁶)`.
- [x] **[TRIV] docs/branch-merge-playbook.md:3** — Remove unexplained blank line or add markdownlint disable comment.
      **Dismissed:** Blank line after SPDX header is repo-wide prettier convention.
- [x] **[TRIV] docs/branch-merge-playbook.md:37** — Same: extra blank line before code block.
      **Dismissed:** Blank line before code fence is standard markdown formatting.
- [x] **[TRIV] docs/branch-merge-playbook.md:44** — Explain or revert indentation changes in code block.
      **Dismissed:** Code block uses correct 4-space TypeScript indentation.
- [x] **[MIN] docs/branch-merge-playbook.md:58** — Add brief inline definition of "Aion" (Echo's timeline concept) on first use.
      **Done:** Added parenthetical `(Echo's per-node timeline weight)` after "Aion".
- [x] **[MAJ] docs/guide/cargo-features.md:10** — Provenance note says "check individual crates" but doesn't give a verification command that actually works. Either provide a real command or remove the claim.
      **Done:** Replaced vague text with a concrete `cargo metadata | jq` command that lists all workspace feature flags.
- [x] **[MIN] docs/guide/warp-primer.md:128** — Emphasis style still inconsistent. Normalize all italic to `_underscores_` (the file's majority style) or all to `*asterisks*`.
      **Done:** Ran prettier — file already normalized (reported unchanged).
- [x] **[MAJ] docs/notes/claude-musings-on-determinism.md:1** — SPDX `MIND-UCAL-1.0` is non-standard. This is project-wide (327 files). Decide: change all 327 to `LicenseRef-MIND-UCAL-1.0`, or document the convention and dismiss.
      **Done:** Renamed across 328 files (336 occurrences) in commit `a4d4101`.
- [x] **[TRIV] docs/notes/claude-musings-on-determinism.md:3** — Blank line after copyright — justified by prettier. Already verified as project-wide convention. Dismiss with explanation.
      **Dismissed:** Blank line after copyright is repo-wide prettier convention.
- [x] **[CRIT] docs/spec-knots-in-time.md:~75** — `SweptVolumeProxy` → `SweepProxy` and module path.
      **Done:** Round-1 fix verified. Line 75 uses `SweepProxy` (canonical name). `warp-geom/src/temporal/manifold.rs:13` matches.
- [x] **[MAJ] docs/tasks/issue-canonical-f32.md:41** — Expand serde acceptance criteria: add NaN canonicalization and subnormal flushing test items.
      **Done:** Expanded single checkbox into 4 separate acceptance criteria: NaN canonicalization, subnormal flushing, serde NaN roundtrip, and serde subnormal roundtrip.
- [x] **[MIN] docs/warp-math-claims.md:8** — Emphasis style change (asterisk → underscore). Revert to match file's dominant style.
      **Done:** Ran prettier — file already normalized (reported unchanged).

---

## Execution Order

1. **Verify Group A** — confirm all local fixes are correct.
2. **Group B: Critical items first** — spec-branch-tree hash formula, spec-temporal-bridge NodeId, spec-runtime-config capability, spec-knots verification.
3. **Group B: Major items** — spec completeness gaps, formatting passes.
4. **Group B: Minor/Trivial** — emphasis, phrasing, blank lines.
5. **Final lint pass** — `cargo xtask docs-lint`, `cargo clippy -p xtask`.
6. **Single commit, single push.**
