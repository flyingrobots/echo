⏺ Echo Docs Audit — Prompt Sequence

  Execute these in order. Each prompt is self-contained.

  ---
  1. Refresh execution-plan.md dates

  Review docs/execution-plan.md and update any stale dates or status markers. The document was last updated around 2025-12-30. Update the "Today's Intent" section and any phase/milestone statuses to reflect current reality. Cross-reference against recent git history and GitHub issues to verify what's actually complete vs in-progress.

  ---
  2. Add status badges to METHODOLOGY.md

  Add implementation status badges to docs/METHODOLOGY.md using the same pattern as architecture-outline.md:

  - Add a legend: ✅ Implemented, ⚠️ Partial, 🗺️ Planned
  - Mark each section/feature with appropriate status
  - The "5x Duty Model" is largely aspirational — mark it as such
  - Note which parts are enforced today vs future vision
  - Update any "Codex's Baby" references to "Event Bus" or "MaterializationBus (ADR-0003)"

  ---
  3. Verify serialization spec vs implementation

  Compare docs/spec-serialization-protocol.md against the actual implementation in crates/warp-core/src/snapshot.rs.

  For each claim in the spec:
  1. Check if it matches the code
  2. Note any discrepancies
  3. Recommend whether to update the spec or the code

  Focus on: encoding format, field ordering, hash algorithm, and any TypeScript/WASM claims that may be aspirational.

  ---
  4. Audit and surface orphaned docs

  Find all markdown files in docs/ that have zero or one inbound references from other docs. For each orphaned file:

  1. Read it to understand its purpose
  2. Decide: DELETE (outdated/superseded), LINK (add to docs-index.md), or MERGE (consolidate into parent doc)
  3. Execute the decision

  Known orphans to check:
  - docs/BENCHMARK_GUIDE.md
  - docs/jitos/spec-0000.md
  - docs/js-cbor-mapping.md
  - docs/append-only-invariants.md
  - docs/dind-harness.md
  - docs/branch-merge-playbook.md
  - docs/rust-rhai-ts-division.md
  - docs/two-lane-abi.md
  - docs/guide/splash-guy.md
  - docs/notes/*.md files

  ---
  5. Add back-links from specs to guides

  For each spec file in docs/spec-*.md, add a "Background Reading" note at the top linking to the relevant guide or primer. Pattern:

  > **Background:** For a gentler introduction, see [WARP Primer](/guide/warp-primer).

  Apply to at least:
  - spec-warp-core.md → guide/warp-primer.md
  - spec-scheduler.md → scheduler.md (the hub doc)
  - spec-branch-tree.md → guide/warp-primer.md
  - spec-serialization-protocol.md → guide/eli5.md (hashing section)

  This prevents readers from getting "stuck" in technical specs with no path back to context.

  ---
  6. Consolidate small stubs

  Find all docs under 25 lines and decide whether to expand or merge them:

  Known stubs:
  - docs/guide/collision-tour.md (11 lines) — merge into guide/start-here.md or expand
  - docs/determinism-invariants.md (15 lines) — merge into spec-warp-core.md or expand
  - docs/scheduler-reserve-complexity.md (11 lines) — already a redirect stub, verify it works
  - docs/guide/course/README.md (14 lines) — merge into guide/course/index.md

  For each: either expand with real content or merge into the appropriate parent doc and leave a redirect stub.

  ---
  7. Complete or remove course modules

  Review docs/guide/course/. The index promises modules 00-09 but most are outlines only.

  Options:
  1. DELETE the incomplete modules and update index.md to reflect what actually exists
  2. WRITE the missing content for modules 02-09

  Recommend option 1 unless the user specifically wants the course content written. Remove promises for content that doesn't exist.

  ---
  8. Add site search to VitePress

  Add search functionality to the VitePress docs site.

  1. Check docs/.vitepress/config.ts for existing search config
  2. Add local search or Algolia DocSearch
  3. For local search, add:
     ```ts
     themeConfig: {
       search: {
         provider: 'local'
       }
     }
  4. Test that search works with pnpm docs:dev

  ---

  ### 9. Create visual topic map

  Create a Mermaid diagram in docs/index.md or docs/docs-index.md showing the documentation structure. Include:

  - Entry points (eli5, start-here, warp-primer)
  - Core specs (spec-warp-core, spec-scheduler, etc.)
  - How they connect (arrows showing "leads to" relationships)
  - Implementation status indicators

  Place it near the top of the docs landing page so new readers can orient themselves visually.

  ---

  ### 10. Create docs/meta/ folder

  Create a docs/meta/ folder for documentation-about-documentation:

  1. Move these files to docs/meta/:
    - docs/docs-audit.md
    - docs/docs-index.md (or keep at root and symlink)
    - docs/legacy-excavation.md
  2. Update any references to moved files
  3. Add a README in docs/meta/ explaining its purpose

  ---

  ### Summary

  | # | Task | Priority | Est. Effort |
  |---|------|----------|-------------|
  | 1 | Refresh execution-plan.md | High | 30 min |
  | 2 | Status badges for METHODOLOGY.md | High | 20 min |
  | 3 | Verify serialization spec | High | 45 min |
  | 4 | Surface orphaned docs | Medium | 60 min |
  | 5 | Add back-links to specs | Medium | 30 min |
  | 6 | Consolidate small stubs | Medium | 30 min |
  | 7 | Complete/remove course modules | Medium | 20 min |
  | 8 | Add site search | Low | 15 min |
  | 9 | Create visual topic map | Low | 30 min |
  | 10 | Create meta/ folder | Low | 15 min |
