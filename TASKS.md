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

- [ ] **[MAJ] line 50:** `phases` cannot be both required and defaulted.
      Made `phases` optional with `?` suffix. Registration already had `?? ["update"]` fallback. Contract is now consistent.
- [ ] **[MAJ] line 143:** Dedup rule broken by unconditional `inDegree` bumps.
      Added `.has()` guard before `.add()` + inDegree increment in both after/before loops.
- [ ] **[MAJ] line 183:** `initialize` never runs for systems added after tick 1.
      Changed to run INITIALIZE unconditionally every tick; only systems with `status: "pending"` execute.
- [ ] **[MAJ] line 278:** `unpauseable` is not a conflict predicate.
      Removed `and not unpauseable` from batching condition. Unpauseable only affects pause handling.
- [ ] **[MIN] line 406:** Wrong warp-core type name.
      Changed `FootprintInfo` → `Footprint` in open questions.

### spec-editor-and-inspector.md (7 items — 3 locally fixed, 4 need work)

- [ ] **[MAJ] line 6:** Broken cross-reference links.
      Fixed `/guide/warp-primer` → `guide/warp-primer.md` and `docs/spec-time-streams...` → `spec-time-streams...`.
- [ ] **[CRIT] line 36:** `object` typing too permissive.
      Changed `payload: object` → `payload: unknown`.
- [ ] **[MAJ] line 40:** Draft note needs exact missing artifacts.
      Updated to name `types.ts` and `registry.ts` with specific missing types.
- [ ] **[CRIT] line 43:** Sorting algorithm for deterministic frame ordering unspecified.
      **NEEDS WORK.** Add: stable sort ascending by `(tick, frameType)` with lexicographic comparison on frameType string. Ties preserve insertion order.
- [ ] **[MAJ] line 51:** "Signed session token" undefined.
      **NEEDS WORK.** Specify token format (HMAC-SHA256 over session ID + capability set + expiry, issued by engine host) or reference a canonical token spec.
- [ ] **[MAJ] line 62:** `filter` field structure undefined.
      **NEEDS WORK.** Define: `filter` is a key-value map where keys are field names from the subscribed `FrameType` schema. Values are exact-match predicates. Example: `{ branchId: "kairos-42" }`.
- [ ] **[MAJ] line 96:** `producer` return type too loose.
      **NEEDS WORK.** Change `producer` return type from `object` to `unknown`.

### xtask/src/main.rs (1 item — locally fixed)

- [ ] **[MAJ] line 791:** SPDX repair ignores `--root`.
      Scoped `ensure_spdx.sh` to pass `md_files` as positional args instead of running repo-wide.

### memorials/2026-01-18-phase4-rubicon.md (3 items — 1 fixed, 2 need work)

- [ ] **[MIN] line 111:** Emphasis half-fixed.
      Changed `_Alea iacta est_.` → `*Alea iacta est*.`
- [ ] **[CRIT] line ~21:** Revert underscore emphasis to asterisks.
      **NEEDS WORK.** `_base + ops = next_` was changed to `` `base + ops = next` `` in round 1, but CR wants asterisks for non-code emphasis. Verify: if it's computational (formula), code span is correct; if it's rhetorical emphasis, use `*...*`.
- [ ] **[MIN] line ~111:** Foreign phrase requires italics, not emphasis.
      **NEEDS WORK.** CR wants `<i>Alea iacta est</i>.` for semantic HTML. Currently `*Alea iacta est*.` — Markdown emphasis is the pragmatic choice for this repo. Decide: keep `*...*` or use `<i>`.

### spec-merkle-commit.md (3 items — 2 fixed, 1 partially)

- [ ] **[MIN] line 6:** Root-relative link.
      Fixed `/guide/warp-primer` → `guide/warp-primer.md`.
- [ ] **[MAJ] line 78:** Parent count validation in `compute_commit_hash_v2()`.
      Added MUST-validate sentence.
- [ ] **[TRIV] line 203:** Consolidate empty digest definition.
      Already added `EMPTY_LEN_DIGEST` constant in round 1. Verify this comment is about the same thing or a new concern.

### Other locally-fixed files (13 items)

- [ ] **[MIN] docs/adr/ADR-0004-No-Global-State.md:178** — Over-escaped `install\_\*` in code block. Fixed.
- [ ] **[CRIT] docs/archive/spec-geom-collision.md:34** — Broken cross-ref `SPEC_DETERMINISTIC_MATH.md`. Fixed → `spec-deterministic-math.md`.
- [ ] **[CRIT] docs/notes/scheduler-optimization-followups.md:30** — Proptest missing.
      Round-1 added "future work" note. CR says that's not enough — proptest is **required** for determinism. **NEEDS WORK.** Either write the proptest or make the note stronger with a tracking issue.
- [ ] **[MAJ] docs/notes/scheduler-optimization-followups.md:65** — Radix sort docs incomplete.
      Round-1 added "see also" note. CR says docs are still incomplete. **NEEDS WORK.** Add the missing inline documentation to the notes file — pass sequence, digit size rationale, LSD vs MSD, thin/fat separation, histogram algorithm.
- [ ] **[MAJ] docs/notes/scheduler-optimization-followups.md:201** — Ambiguous benchmark note. Fixed.
- [ ] **[MIN] docs/spec-ecs-storage.md:6** — Root-relative link. Fixed.
- [ ] **[MIN] docs/spec-geom-collision.md:7** — Vague deferral. Fixed.
- [ ] **[MIN] docs/spec-mwmr-concurrency.md:6** — Broken link. Fixed.
- [ ] **[MIN] docs/spec-mwmr-concurrency.md:51** — Name "Theorem A". **NEEDS WORK.** Replace with the actual theorem name from the warp-math papers.
- [ ] **[MAJ] docs/spec-warp-confluence.md:66** — Signing canonicalization underspecified.
      Round-1 added a note. CR says it's still not enough. **NEEDS WORK.** Add a normative subsection specifying the exact field list and encoding order used for signing.
- [ ] **[MIN] docs/spec-warp-confluence.md:6** — Root-relative link. Fixed.
- [ ] **[MAJ] docs/spec-world-api.md:6** — Broken primer link. Fixed.
- [ ] **[MAJ] docs/spec-world-api.md:~92** — Version management too vague.
      Round-1 added SemVer note. CR says still vague. **NEEDS WORK.** Add explicit breaking-change criteria and deprecation timeline.

---

## Group B: New Work Required (34 items)

These files have no local changes yet. Each needs investigation + fix.

### spec-branch-tree.md (10 items — spec completeness)

This spec has significant gaps that CR flagged. Every item relates to
determinism: undefined types/formulas mean implementations could diverge.

- [ ] **[MAJ] line 36:** Define `ReadKey` and `WriteKey` as formal interfaces.
      Need: canonical type definitions with fields, ordering semantics, and serialization format.
- [ ] **[MAJ] line 60:** Formalize `MergeStrategyId` type.
      Need: concrete type (string enum? u32? hash?), canonical values, and how new strategies are registered.
- [ ] **[CRIT] line 116:** Hash formula references non-existent field.
      Need: verify the `id` formula against actual struct fields. Fix or add the missing field.
- [ ] **[MAJ] line 177:** Define entropy formula weights.
      Need: specify `w_M` and `w_P` default values and how they're configured.
- [ ] **[MAJ] line 199:** Clarify byte-level encoding in seed derivation.
      Need: specify endianness, concatenation order, and padding for `blake3(branch_id || parent_seed || epoch)`.
- [ ] **[MAJ] line 206:** Clarify all GC modes are deterministic.
      Need: replace ambiguous language with explicit "all GC modes MUST produce identical results given identical inputs."
- [ ] **[MAJ] line 252:** Define `WorldView` and `GCPolicy` types.
      Need: interfaces with fields, types, and defaults.
- [ ] **[MIN] line 300:** Define causal relation semantics.
      Need: clarify what "causal" means — happened-before? vector clock? Lamport timestamp?
- [ ] **[MIN] line 373:** Specify entropy bounds and initialization.
      Need: initial entropy value, min/max bounds, overflow behavior.
- [ ] **[MIN] line 390:** Define capability token structure.
      Need: specify token format or reference the capabilities spec.

### spec-temporal-bridge.md (1 item)

- [ ] **[CRIT] line 115:** API exposes opaque `NodeId`s but lifecycle rules dereference them as full nodes.
      Need: clarify whether the API returns `NodeId` (opaque handle requiring a separate lookup) or the full `TimelineNode` (data included). Update type signatures to match.

### spec-runtime-config.md (1 item)

- [ ] **[CRIT] line ~54:** `world:config` capability undefined.
      Need: add `world:config` to `spec-capabilities-and-security.md` with description, required privileges, and scope.

### spec-serialization-protocol.md (2 items)

- [ ] **[MIN] line 6:** Root-relative link. Fix `/guide/eli5` → `guide/eli5.md`.
      (Agent may have already fixed this — verify.)
- [ ] **[MAJ] line 141:** `payloads` field semantics and serialization order incomplete.
      Need: specify field ordering, whether payloads is length-prefixed or delimiter-separated, and how empty payloads are encoded.

### spec-time-streams-and-wormholes.md (2 items)

- [ ] **[MAJ] line 189:** StreamAdmissionDecision canonical field ordering.
      Round-1 added a note. CR says verify it's actually specified, not just noted. Need: confirm the ordering note is normative.
- [ ] **[TRIV] line 509:** Narrative example numbering creates maintenance burden.
      Need: convert numbered examples to bullet list or label them semantically.

### SPEC-0002-descended-attachments-v1.md (5 items — formatting consistency)

- [ ] **[MAJ] line 3:** Blank-line policy chaos.
      Need: run a full formatting pass — one blank line after every heading, consistent indentation throughout.
- [ ] **[MAJ] line 52:** AttachmentPlane consolidation inconsistency.
      Need: propagate the inline enum variant style to ALL enum definitions, or revert to the previous sub-bullet style for all.
- [ ] **[MAJ] line ~53:** Enum variant nesting inconsistency.
      Same as above — pick one style and apply uniformly.
- [ ] **[TRIV] line 192:** Algorithm formatting — verify logical structure preserved.
      Need: manual review of step 2/step 4 after round-1 restructuring.
- [ ] **[TRIV] line 226:** Header spacing — acceptable but inconsistent.
      Covered by the blank-line pass above.

### SPEC-0003-dpo-concurrency-litmus-v0.md (1 item)

- [ ] **[MIN] line 45:** Calling read/read overlap "disjoint" is wrong.
      Need: replace "disjoint" with "non-conflicting" or "compatible" for read/read access.

### Other docs (12 items)

- [ ] **[TRIV] docs/DETERMINISTIC_MATH.md:52** — Tighten "very small numbers" to "subnormal values (magnitude < 2^−126)".
- [ ] **[TRIV] docs/branch-merge-playbook.md:3** — Remove unexplained blank line or add markdownlint disable comment.
- [ ] **[TRIV] docs/branch-merge-playbook.md:37** — Same: extra blank line before code block.
- [ ] **[TRIV] docs/branch-merge-playbook.md:44** — Explain or revert indentation changes in code block.
- [ ] **[MIN] docs/branch-merge-playbook.md:58** — Add brief inline definition of "Aion" (Echo's timeline concept) on first use.
- [ ] **[MAJ] docs/guide/cargo-features.md:10** — Provenance note says "check individual crates" but doesn't give a verification command that actually works. Either provide a real command or remove the claim.
- [ ] **[MIN] docs/guide/warp-primer.md:128** — Emphasis style still inconsistent. Normalize all italic to `_underscores_` (the file's majority style) or all to `*asterisks*`.
- [ ] **[MAJ] docs/notes/claude-musings-on-determinism.md:1** — SPDX `MIND-UCAL-1.0` is non-standard. This is project-wide (327 files). Decide: change all 327 to `LicenseRef-MIND-UCAL-1.0`, or document the convention and dismiss.
- [ ] **[TRIV] docs/notes/claude-musings-on-determinism.md:3** — Blank line after copyright — justified by prettier. Already verified as project-wide convention. Dismiss with explanation.
- [ ] **[CRIT] docs/spec-knots-in-time.md:~75** — `SweptVolumeProxy` → `SweepProxy` and module path.
      Round-1 already fixed this. Verify the fix is correct and this is a stale comment.
- [ ] **[MAJ] docs/tasks/issue-canonical-f32.md:41** — Expand serde acceptance criteria: add NaN canonicalization and subnormal flushing test items.
- [ ] **[MIN] docs/warp-math-claims.md:8** — Emphasis style change (asterisk → underscore). Revert to match file's dominant style.

---

## Execution Order

1. **Verify Group A** — confirm all local fixes are correct.
2. **Group B: Critical items first** — spec-branch-tree hash formula, spec-temporal-bridge NodeId, spec-runtime-config capability, spec-knots verification.
3. **Group B: Major items** — spec completeness gaps, formatting passes.
4. **Group B: Minor/Trivial** — emphasis, phrasing, blank lines.
5. **Final lint pass** — `cargo xtask docs-lint`, `cargo clippy -p xtask`.
6. **Single commit, single push.**
